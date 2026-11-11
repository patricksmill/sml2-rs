//! Live level state and its ordered simulation tick.

use super::{
    BellArm, CameraSim, EndingEntry, EndingInput, EndingScene, EndingSequenceController, HudStrip,
    LevelLayout, LevelManifest, MapHandoffOutcome, MapNodes, ObjectClock, ObjectSim, ObjectTables,
    PlayerKeys, PlayerSim, SceneScripts, SceneVm, SpriteCtx, TileAnimationClock, Tilesets,
    apply_level_entry, note_handoff_screen, player_behind_bg, seed_level_timer,
};

use crate::game::level_frame::{
    LiveLevelFrame, begin_live_level_frame, step_live_level_frame, step_two_player_live_level_frame,
};

/// One frame of the two keyboard players. `first`/`second` are the held
/// joypads; `first_pressed`/`second_pressed` carry the new-press edges.
#[derive(Debug, Clone, Copy, Default)]
pub struct TwoPlayerFrameInput {
    pub first: PlayerKeys,
    pub first_pressed: PlayerKeys,
    pub second: PlayerKeys,
    pub second_pressed: PlayerKeys,
    pub div: u8,
}

/// What the co-op layer must do after a live frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoopFrameResult {
    /// Both players are out of play: restart the level's entry checkpoint.
    pub restart_checkpoint: bool,
    /// The shared lives are exhausted; enter the GAME OVER screen.
    pub game_over: bool,
}

/// The second local player inside a live level. Its `PlayerSim` is a full
/// native player (hazards, items, pipes, exits); `bubble` holds it out of
/// play until the other player pops it.
pub struct SecondPlayer {
    pub sim: PlayerSim,
    pub bubble: Option<Bubble>,
}

/// A player waiting in New Super Mario Bros. Wii's revival bubble
/// (`dAcPy_c::State_Balloon`). The player is out of play and out of the
/// camera's party; after [`Bubble::APPEAR_DELAY`] frames the bubble shows,
/// drifts toward the player still in play, and pops on touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bubble {
    pub x: u16,
    pub y: u16,
    /// Frames left before the bubble appears.
    pub delay: u16,
}

impl Bubble {
    /// `initializeState_Balloon` case 0: 150 frames after a defeat.
    pub const APPEAR_DELAY: u16 = 150;
    /// `finalizeState_Balloon`'s `mDamageInvulnTimer = 127`.
    pub const POP_INVULNERABILITY: u8 = 127;
    /// `mAllBalloonFadeTimer = 210`: an all-bubbled party restarts after it.
    pub const WIPE_FRAMES: u16 = 210;
    /// How close a live player must come to pop it, in pixels.
    pub const POP_REACH: (i32, i32) = (12, 16);
    /// Where the bubble settles relative to the player it follows: beside
    /// it, on whichever side the bubble already is, at head height — so the
    /// player can walk or hop into it without the bubble sitting on top.
    const HOVER: (i16, i16) = (20, -8);
    /// Beyond this many pixels the bubble floats at 1 px per frame per axis;
    /// closer, at half that.
    const FAR: i32 = 48;
    /// `$FFC6` frame drawn inside the bubble: small standing (a bubbled
    /// player has lost its powerup).
    const FRAME: u8 = 32;

    fn new(at: &PlayerSim, delay: u16) -> Self {
        Self {
            x: at.x,
            y: at.y,
            delay,
        }
    }

    /// The bubbled player's drawn position and frame, once visible.
    pub fn sprite(&self) -> Option<(u16, u16, u8)> {
        (self.delay == 0).then_some((self.x, self.y, Self::FRAME))
    }

    /// One frame: count down, then float slowly toward the player in play —
    /// 1 px per frame per axis while more than [`Self::FAR`] away, every other
    /// frame once near — while staying inside the shared view.
    fn drift(&mut self, toward: &PlayerSim, cam: &CameraSim) {
        if self.delay != 0 {
            self.delay -= 1;
            return;
        }
        let even = toward.frame_counter & 1 == 0;
        let step = |from: u16, to: u16| {
            let gap = i32::from(to.wrapping_sub(from) as i16);
            if gap.abs() > Self::FAR || even {
                from.wrapping_add_signed(gap.signum() as i16)
            } else {
                from
            }
        };
        let side = if (self.x.wrapping_sub(toward.x) as i16) < 0 {
            -Self::HOVER.0
        } else {
            Self::HOVER.0
        };
        self.x = step(self.x, toward.x.wrapping_add_signed(side));
        self.y = step(self.y, toward.y.wrapping_add_signed(Self::HOVER.1));
        let (left, width, top, height) = match toward.view {
            Some(v) => (v.x as u16, v.w - 16, v.y as u16, v.h - 32),
            None => (
                cam.x.wrapping_sub(96).wrapping_add(0x10),
                0x90,
                cam.y.wrapping_sub(64),
                112,
            ),
        };
        let clamp = |v: u16, lo: u16, span: i32| {
            let rel = i32::from(v.wrapping_sub(lo) as i16);
            lo.wrapping_add(rel.clamp(0, span) as u16)
        };
        self.x = clamp(self.x, left, width);
        self.y = clamp(self.y, top, height);
    }

    fn touched_by(&self, p: &PlayerSim) -> bool {
        self.delay == 0
            && i32::from(p.x.wrapping_sub(self.x) as i16).abs() < Self::POP_REACH.0
            && i32::from(p.y.wrapping_sub(self.y) as i16).abs() < Self::POP_REACH.1
    }
}

/// Take a player out of play into a bubble at its current position.
fn enter_bubble(sim: &mut PlayerSim, delay: u16) -> Bubble {
    sim.dead_fade = false;
    sim.fade_timer = 0;
    sim.powerup = 0;
    sim.invincibility = 0;
    sim.pipe_dir = 0;
    sim.hvel = 0;
    Bubble::new(sim, delay)
}

/// `setBreakBalloonJump` / `finalizeState_Balloon`: back in play where the
/// bubble burst, briefly invulnerable. Level state (layout edits, counters)
/// is untouched, unlike a level restart.
fn pop_bubble(sim: &mut PlayerSim, bubble: &Bubble) {
    sim.x = bubble.x;
    sim.y = bubble.y;
    sim.dead = 0;
    sim.dead_fade = false;
    sim.fade_timer = 0;
    sim.pipe_dir = 0;
    sim.hvel = 0;
    sim.vvel = 24;
    sim.on_ground = false;
    sim.powerup = 0;
    sim.invincibility = Bubble::POP_INVULNERABILITY;
}

/// Whether a live player may bubble on purpose (`setBalloonButtonA`'s gates
/// that exist here): in play, not travelling, exiting, or paused.
fn may_bubble(sim: &PlayerSim, cam: &CameraSim) -> bool {
    sim.dead == 0
        && !sim.paused
        && !sim.exit_phase
        && !sim.exit_completed
        && sim.pipe_dir == 0
        && cam.transition == 0
}

/// `daPyMng_c::createCourseInit`'s `playerSetDist` for two players.
const SPAWN_GAP: u16 = 24;

/// One player's bubble this frame. `other` is the other player and whether
/// it is in play: a bubble needs someone to follow and pop it, so a player
/// alone in play is only bubbled by a defeat (which then wipes the party).
fn bubble_pass(
    mine: &mut Option<Bubble>,
    me: &mut PlayerSim,
    (other_free, other): (bool, &PlayerSim),
    cam: &CameraSim,
    requested: bool,
    select: bool,
) {
    let other_live = other_free && other.dead != 0xFF;
    if mine.is_none() {
        if me.dead == 0xFF {
            *mine = Some(enter_bubble(me, Bubble::APPEAR_DELAY));
        } else if other_live && (requested || select && may_bubble(me, cam)) {
            *mine = Some(enter_bubble(me, 0));
        }
        return;
    }
    if !other_live {
        return;
    }
    if let Some(bubble) = mine.as_mut() {
        bubble.drift(other, cam);
        if bubble.touched_by(other) {
            let bubble = *bubble;
            pop_bubble(me, &bubble);
            *mine = None;
        }
    }
}

/// Point the sims at the world of a layout swapped in mid-level. The arena
/// layout is ROM-sized; a rebuilt level's camera also returns to the tables
/// the arena was authored against, which `$25AF` leaves in place.
pub(crate) fn enter_layout_world(
    sim: &mut PlayerSim,
    cam: &mut CameraSim,
    obj: &mut ObjectSim,
    layout: &LevelLayout,
    rom_tables: Option<&crate::content::RomTables>,
) {
    (sim.world, cam.world, obj.world) = (layout.world, layout.world, layout.world);
    if let Some(tables) = rom_tables {
        cam.edge_map = tables
            .edge_map
            .clone()
            .unwrap_or_else(|| CameraSim::new(0, 0).edge_map);
        cam.scene_map = tables.scene_map.clone().unwrap_or_default();
    }
}

/// In-level simulation state and working buffers for a live level play.
pub struct LevelContext {
    pub manifest: LevelManifest,
    pub layout: LevelLayout,
    pub scenery: sml2_core::scenery::SceneryOverlay,
    pub layout_live: LevelLayout,
    pub bg_base: Vec<u8>,
    pub bg_live: Vec<u8>,
    pub anim_rows: Vec<u8>,
    pub hud: HudStrip,
    pub hud_y: u8,
    pub(crate) spr: Option<SpriteCtx>,
    pub sim: PlayerSim,
    pub cam: CameraSim,
    pub obj: ObjectSim,
    pub scene_vm: SceneVm,
    /// `$25AF` leaves all three palettes white until `$0EF3` ends the scroll.
    pub(crate) scene_reload_blank: bool,
    pub ending_entry: EndingEntry,
    pub ending_sequence: EndingSequenceController,
    pub ending_scene: EndingScene,
    /// `ending/credits.bin`, sliced into the step context.
    pub credits_text: Vec<u8>,
    pub scripts: SceneScripts,
    pub tilesets: Tilesets,
    pub arena_layout: Option<LevelLayout>,
    /// The tables an arena reload restores in a rebuilt level.
    pub rom_tables: Option<crate::content::RomTables>,
    pub tile_animation: TileAnimationClock,
    /// A phase has been copied to `$9200` since the level loaded. Presentation
    /// only: DX tells a phase from identical base tiles by it.
    pub tile_animation_copied: bool,
    /// DX `$26:$6B0B`'s regional palette state (`$FFAD` and what it loaded).
    pub(crate) dx_regional: super::dx_palette::RegionalPalettes,
    /// `$25AF` has swapped in a boss arena's tileset. Presentation only: DX
    /// adds the arena's VRAM1 tiles on that reload.
    pub arena_loaded: bool,
    /// `sObjectDefIndex` of the last boss fight whose scene load ran
    /// (`$0EB0` -> `$3C94`).
    pub boss_definition: Option<u8>,
    pub frame_n: u64,
    pub map_handoff: Option<MapHandoffOutcome>,
    pub map_handoff_frame: Option<u64>,
    pub exit_completed_frame: Option<u64>,
    pub(crate) bell_arm: Option<BellArm>,
    pub active_object_frames: usize,
    pub gameplay_death_frames: usize,
    /// Local co-op player 2; `None` outside `--multiplayer` sessions.
    pub second: Option<SecondPlayer>,
    /// Player 1 waiting in a revival bubble.
    pub first_bubble: Option<Bubble>,
    /// Frames since the whole party went into bubbles.
    pub wipe_frames: u16,
    /// The level-entry `sLevelProgressFlags` byte, kept for checkpoint restarts.
    pub entry_progress: u8,
}

impl LevelContext {
    #[allow(clippy::too_many_arguments)]
    pub fn from_content(
        level_id: u8,
        content: &crate::content::LevelContent,
        level_progress: u8,
        player_lives: u8,
        player_coins_low: u8,
        player_coins_high: u8,
        player_powerup: u8,
        map_node: u8,
        scripts: SceneScripts,
        tilesets: Tilesets,
        arena_layout: Option<LevelLayout>,
        anim_rows: Vec<u8>,
        hud: HudStrip,
        object_tables: ObjectTables,
    ) -> Self {
        let manifest = content.manifest.clone();
        let layout = content.layout.clone();
        let bg_base = content.bg_base.clone();
        let objblock = content.objblock.clone();

        let mut sim = PlayerSim::new();
        sim.world = layout.world;
        seed_level_timer(&mut sim, &manifest);
        sim.lives = player_lives;
        sim.coins_low = player_coins_low;
        sim.coins_high = player_coins_high;
        sim.powerup = player_powerup;
        sim.map_node = map_node;
        sim.seed_moon_physics(manifest.properties.space_physics);
        sim.level_song = manifest.header.music_track;
        sim.music_request = manifest.header.music_track;

        let level_entry = manifest.header.entry(level_progress, manifest.secret_entry);
        let mut cam = CameraSim::new(level_entry.start.camera_x, level_entry.start.camera_y);
        cam.world = layout.world;
        apply_level_entry(&mut sim, &mut cam, level_entry);
        cam.prev_mario_y_lo = sim.y as u8;
        cam.map_scroll_limit = manifest.properties.unknown != 0;
        cam.auto_scroll = manifest.properties.auto_scroll;
        cam.scene_state = manifest.header.scene_state;

        if let Some(edge) = &content.edge_map {
            cam.edge_map.clone_from(edge);
        }
        if let Some(scene) = &content.scene_map {
            cam.scene_map.clone_from(scene);
        }

        let mut obj = ObjectSim::new(manifest.spawns.records.clone(), cam.x);
        obj.world = layout.world;
        obj.configure(level_id);
        ObjectClock::default().initialize_level(&mut obj);

        let spr = Some(SpriteCtx {
            insts: Vec::new(),
            objblock,
            tables: object_tables,
            level_id,
            obp0: manifest.header.palettes[1],
            obp1: manifest.header.palettes[2],
            player: Some((sim.x, sim.y, 0)),
            player_two: None,
            player_bubble: None,
            player_two_bubble: None,
            player_behind_bg: false,
            player_two_behind_bg: false,
            pipe_background: None,
            player_star_flash: false,
            player_two_star_flash: false,
            player_shield: None,
            player_two_shield: None,
            streak_counter: None,
            player_pan: 0,
            scroll_pan: 0,
            bumpers: Vec::new(),
            fireballs: Vec::new(),
            boss_oam: Vec::new(),
            boss_draws: Vec::new(),
        });

        Self {
            layout_live: layout.clone(),
            bg_live: bg_base.clone(),
            manifest,
            layout,
            scenery: content.scenery.clone(),
            bg_base,
            anim_rows,
            hud,
            hud_y: 136,
            spr,
            sim,
            cam,
            obj,
            scene_vm: SceneVm::new(),
            scene_reload_blank: false,
            ending_entry: EndingEntry::default(),
            ending_sequence: EndingSequenceController::new(),
            ending_scene: EndingScene::default(),
            credits_text: Vec::new(),
            scripts,
            tilesets,
            arena_layout,
            rom_tables: content.rom_tables.clone(),
            tile_animation: TileAnimationClock::default(),
            tile_animation_copied: false,
            dx_regional: Default::default(),
            arena_loaded: false,
            boss_definition: None,
            frame_n: 0,
            map_handoff: None,
            map_handoff_frame: None,
            exit_completed_frame: None,
            bell_arm: None,
            active_object_frames: 0,
            gameplay_death_frames: 0,
            second: None,
            first_bubble: None,
            wipe_frames: 0,
            entry_progress: level_progress,
        }
    }

    /// Create player 2 beside player 1 with independent damage and powerup
    /// state. Shared counters start from the current session. As in
    /// `daPyMng_c::createCourseInit`, the two stand [`SPAWN_GAP`] apart
    /// around the entry point; a wall on either spot keeps that player on
    /// the entry point instead.
    pub fn enable_second_player(&mut self, powerup: u8) {
        if self.second.is_some() {
            return;
        }
        let mut sim = PlayerSim::new();
        sim.world = self.layout.world;
        sim.level_timer = self.sim.level_timer;
        sim.lives = self.sim.lives;
        sim.coins_low = self.sim.coins_low;
        sim.coins_high = self.sim.coins_high;
        sim.level_tally = self.sim.level_tally;
        sim.level_progress = self.sim.level_progress;
        sim.level_progress_flags = self.sim.level_progress_flags;
        sim.map_node = self.sim.map_node;
        sim.secret_route = self.sim.secret_route;
        sim.level_song = self.sim.level_song;
        sim.tile_anim_config = self.sim.tile_anim_config;
        sim.seed_moon_physics(self.manifest.properties.space_physics);
        sim.powerup = powerup;
        sim.y = self.sim.y;
        sim.camx = self.cam.x;
        sim.camy = self.cam.y;
        sim.phase = self.sim.phase;
        sim.frame_counter = self.sim.frame_counter;
        sim.view = self.sim.view;
        sim.view_clamp = self.sim.view_clamp;
        sim.shared_screen = true;
        self.sim.shared_screen = true;
        let entry = self.sim.x;
        self.sim.x = entry.wrapping_add(SPAWN_GAP / 2);
        if self.sim.wall_beside(&self.layout_live, true) {
            self.sim.x = entry;
        }
        sim.x = self.sim.x.wrapping_sub(SPAWN_GAP);
        if sim.wall_beside(&self.layout_live, false) {
            sim.x = entry;
        }
        self.second = Some(SecondPlayer { sim, bubble: None });
    }

    /// This frame's NSMBW bubbles: players the live frame left behind or
    /// moved on without, defeated players, and a player pressing Select to
    /// bubble on purpose; then drift and popping. Returns true once an
    /// all-bubbled party has waited out [`Bubble::WIPE_FRAMES`].
    fn step_bubbles(&mut self, requested: [bool; 2], select: [bool; 2]) -> bool {
        let Some(second) = self.second.as_mut() else {
            return false;
        };
        let cam = &self.cam;
        bubble_pass(
            &mut self.first_bubble,
            &mut self.sim,
            (second.bubble.is_none(), &second.sim),
            cam,
            requested[0],
            select[0],
        );
        bubble_pass(
            &mut second.bubble,
            &mut second.sim,
            (self.first_bubble.is_none(), &self.sim),
            cam,
            requested[1],
            select[1],
        );
        if self.first_bubble.is_some() && second.bubble.is_some() {
            self.wipe_frames = self.wipe_frames.saturating_add(1);
        } else {
            self.wipe_frames = 0;
        }
        self.wipe_frames >= Bubble::WIPE_FRAMES
    }

    /// Copy an exit/boss hand-off the second player earned onto the session
    /// player, which owns the existing level-exit path.
    fn adopt_second_exit(&mut self) {
        let Some(second) = self.second.as_ref() else {
            return;
        };
        let from = &second.sim;
        if !from.exit_phase && !from.exit_completed && !from.boss_clear {
            return;
        }
        if self.sim.exit_phase {
            return;
        }
        self.sim.exit_phase = from.exit_phase;
        self.sim.exit_completed = from.exit_completed;
        self.sim.boss_clear = from.boss_clear;
        // The exiting player is the live one; the hand-off must not read the
        // other player's bubble as a death.
        self.sim.dead = from.dead;
        self.sim.dead_fade = false;
        self.first_bubble = None;
        self.sim.fade_timer = from.fade_timer;
        self.sim.exit_tile_flag = from.exit_tile_flag;
        self.sim.secret_route = from.secret_route;
        self.sim.low_time_flag = from.low_time_flag;
        self.sim.music_request = from.music_request;
        self.sim.tile_anim_config = from.tile_anim_config;
        self.sim.level_tally = from.level_tally;
        self.sim.level_progress = from.level_progress;
        self.sim.level_progress_flags = from.level_progress_flags;
        self.sim.golden_coin_flags = from.golden_coin_flags;
    }

    /// Keep the render context in step with a bubble that was entered (or
    /// left) during this frame, rather than waiting one frame to hide the
    /// dead player's sprite.
    fn refresh_coop_sprites(&mut self) {
        let Some(spr) = self.spr.as_mut() else {
            return;
        };
        let first = self.first_bubble;
        if first.is_some() {
            spr.player = None;
            spr.player_bubble = first.and_then(|b| b.sprite());
        } else {
            spr.player_bubble = None;
            if spr.player.is_none() {
                spr.player = Some((self.sim.x, self.sim.y, 0));
            }
        }
        if let Some(second) = self.second.as_ref() {
            if let Some(bubble) = second.bubble {
                spr.player_two = None;
                spr.player_two_bubble = bubble.sprite();
            } else {
                spr.player_two_bubble = None;
                if spr.player_two.is_none() {
                    spr.player_two = Some((second.sim.x, second.sim.y, 0));
                }
            }
        }
    }

    /// Player 2 probes the same level map: mirror the shared edit list, which
    /// the live frame updates for both players.
    fn share_layout_writes(&mut self) {
        if let Some(second) = self.second.as_mut() {
            second.sim.layout_writes.clone_from(&self.sim.layout_writes);
        }
    }

    /// `$25AF`: swap in the boss arena's layout, which replaces prior block
    /// edits. See [`enter_layout_world`].
    pub(crate) fn load_arena(&mut self) -> Result<(), String> {
        self.layout_live = self.arena_layout.clone().ok_or("arena layout is missing")?;
        self.sim.layout_writes.clear();
        enter_layout_world(
            &mut self.sim,
            &mut self.cam,
            &mut self.obj,
            &self.layout_live,
            self.rom_tables.as_ref(),
        );
        Ok(())
    }

    /// Step the in-level simulation by 1 frame.
    #[allow(clippy::too_many_arguments)]
    pub fn step_frame(
        &mut self,
        keys: PlayerKeys,
        a_pressed: bool,
        b_pressed: bool,
        start_pressed: bool,
        select_pressed: bool,
        div: u8,
        nodes: &MapNodes,
    ) {
        self.frame_n += 1;
        let head = begin_live_level_frame(
            &mut self.sim,
            &mut self.cam,
            &mut self.scene_vm,
            &mut self.ending_entry,
            EndingInput {
                held: keys,
                pressed: PlayerKeys {
                    a: a_pressed,
                    b: b_pressed,
                    start: start_pressed,
                    select: select_pressed,
                    ..Default::default()
                },
            },
        );
        // `$0618` poses Mario before `$0636` stages the next object references.
        // Paused/fade frames and `$0640`'s dead draw do not call `$530D`.
        if head.gameplay && !head.frozen && self.sim.dead != 0xFF {
            self.dx_regional.update_references(
                self.manifest.level_id,
                self.obj.mario_ref_x,
                self.obj.mario_ref_y,
            );
        }
        // Sessions progress death fades and depart; diagnostic play holds them.
        if head.gameplay && head.frozen {
            if self.sim.exit_phase {
                self.sim.exit_fade_step();
            } else if self.sim.dead_fade {
                self.sim.fade_step();
                // A borrow leaves `$FF9B = $05`; only a continue reaches `$14`.
                if self.sim.fade_timer == 0 && !self.sim.game_over {
                    // A death ends in the same `$FF9B = $14` hand-off as a
                    // clear: its zone band reloads the screen Mario died from
                    // (PyBoy: a Tree Zone time-out lands on screen 2, node $23).
                    let handoff = self.sim.map_handoff(nodes);
                    note_handoff_screen(&handoff);
                    if self.map_handoff.is_none() {
                        self.map_handoff = Some(handoff);
                    }
                }
            }
            if head.exit_done_at_entry {
                let handoff = self.sim.map_handoff(nodes);
                note_handoff_screen(&handoff);
                if self.map_handoff.is_none() {
                    self.map_handoff = Some(handoff);
                }
            }
        }
        let result = step_live_level_frame(
            LiveLevelFrame {
                sim: &mut self.sim,
                cam: &mut self.cam,
                obj: &mut self.obj,
                scene_vm: &mut self.scene_vm,
                ending_sequence: &mut self.ending_sequence,
                ending_scene: &mut self.ending_scene,
                credits_text: (!self.credits_text.is_empty())
                    .then_some(self.credits_text.as_slice()),
                manifest: &mut self.manifest,
                layout_live: &mut self.layout_live,
                bg_live: &mut self.bg_live,
                spr: &mut self.spr,
                scripts: &self.scripts,
                tilesets: &self.tilesets,
                arena_layout: self.arena_layout.as_ref(),
                rom_tables: self.rom_tables.as_ref(),
                anim_rows: &self.anim_rows,
                tile_animation: &mut self.tile_animation,
            },
            head,
            div,
        );
        if result.transition_finished {
            self.scene_reload_blank = false;
        }
        if let Some(definition) = result.boss_definition {
            self.boss_definition = Some(definition);
            // `$36:$7306` overwrites palettes after the pose selector ran.
            // Wario 7 (`$7440`) retains the previous palettes; none reset `$FFAD`.
            if definition != 7 {
                self.dx_regional.palette_load();
            }
        }
        if result.active_objects {
            self.active_object_frames += 1;
        }
        if let Some(reload) = result.arena_reload {
            self.scene_reload_blank = true;
            self.arena_loaded |= reload.applied;
        }
        // These presentation latches belong to the session; diagnostic play
        // historically leaves the pipe attribute and expanded boss draws alone.
        if let Some(s) = self.spr.as_mut() {
            s.player_behind_bg = player_behind_bg(self.sim.pipe_dir);
            if self.cam.boss_mode() {
                s.boss_draws.clone_from(&self.obj.boss_draws);
            } else {
                s.boss_draws.clear();
            }
        }
        if self.sim.exit_phase && self.bell_arm.is_none() {
            self.bell_arm = Some(BellArm {
                frame: self.frame_n,
                position: (self.sim.x, self.sim.y),
                fade_timer: self.sim.fade_timer,
                music_request: self.sim.music_request,
                exit_tile_flag: self.sim.exit_tile_flag,
                dead: self.sim.dead,
            });
        }
        if head.gameplay && !head.frozen && self.sim.dead == 0xFF {
            self.gameplay_death_frames += 1;
        }
        if self.sim.exit_completed && self.exit_completed_frame.is_none() {
            self.exit_completed_frame = Some(self.frame_n);
        }
        if self.sim.map_handoff_done && self.map_handoff_frame.is_none() {
            self.map_handoff_frame = Some(self.frame_n);
        }

        let flash = if self.scene_reload_blank {
            Some(0)
        } else if self.sim.paused {
            Some(self.sim.pause_flash())
        } else {
            None
        };
        if let Some(s) = self.spr.as_mut() {
            s.obp0 = flash.unwrap_or(self.manifest.header.palettes[1]);
            s.obp1 = if self.scene_reload_blank {
                0
            } else {
                self.manifest.header.palettes[2]
            };
        }
        self.tile_animation_copied |= result.tile_animation_copied;
    }

    /// One simultaneous local two-player frame. Both players get their own
    /// `$0F71` update and contact pass; the object engine ticks once, and
    /// the camera follows the midpoint of the players still in play.
    pub fn step_two_player_frame(
        &mut self,
        input: TwoPlayerFrameInput,
        nodes: &MapNodes,
    ) -> CoopFrameResult {
        self.frame_n += 1;
        let timer_before = self.sim.level_timer;
        let head = begin_live_level_frame(
            &mut self.sim,
            &mut self.cam,
            &mut self.scene_vm,
            &mut self.ending_entry,
            EndingInput {
                held: input.first,
                pressed: input.first_pressed,
            },
        );
        if let Some(second) = self.second.as_mut() {
            // The single-player head latches these for player 1; do the same
            // for player 2 before either physics step.
            second.sim.camx = self.cam.x;
            second.sim.camy = self.cam.y;
            second.sim.pan_timer = self.cam.pan_timer;
            second.sim.phase = (second.sim.phase + 1) & 7;
            second.sim.frame_counter = second.sim.frame_counter.wrapping_add(1);
            second.sim.view = self.sim.view;
            second.sim.view_clamp = self.sim.view_clamp;
            // `$05D5` for player 2: its own star countdown, and the party's
            // level clock run from the same pre-tick value, so a time-out
            // defeats player 2 too.
            if head.gameplay && !self.sim.paused && second.bubble.is_none() {
                second.sim.level_timer = timer_before;
                crate::game::update_level_timer_at_frame_head(&mut second.sim);
            }
            second.sim.level_timer = self.sim.level_timer;
        }
        let second_input = EndingInput {
            held: if head.gameplay && self.sim.fade_timer <= 1 && !self.sim.exit_phase {
                input.second
            } else {
                PlayerKeys::default()
            },
            pressed: if head.gameplay && self.sim.fade_timer <= 1 && !self.sim.exit_phase {
                input.second_pressed
            } else {
                PlayerKeys::default()
            },
        };
        // The single-player frozen policy still owns exit fades and the
        // hand-off; co-op death is handled by bubbles below instead.
        if head.gameplay && head.frozen {
            if self.sim.exit_phase {
                self.sim.exit_fade_step();
            } else if self.sim.dead_fade {
                self.sim.fade_step();
            }
            if self.sim.fade_timer == 0 && !self.sim.game_over {
                let handoff = self.sim.map_handoff(nodes);
                note_handoff_screen(&handoff);
                if self.map_handoff.is_none() {
                    self.map_handoff = Some(handoff);
                }
            }
            if head.exit_done_at_entry {
                let handoff = self.sim.map_handoff(nodes);
                note_handoff_screen(&handoff);
                if self.map_handoff.is_none() {
                    self.map_handoff = Some(handoff);
                }
            }
        }

        let first_bubbled = self.first_bubble.is_some();
        let first_bubble = self.first_bubble.and_then(|b| b.sprite());
        let second_bubbled = self.second.as_ref().is_some_and(|s| s.bubble.is_some());
        let second_bubble = self
            .second
            .as_ref()
            .and_then(|s| s.bubble)
            .and_then(|b| b.sprite());
        let result = {
            let Some(second) = self.second.as_mut() else {
                // A multiplayer session always seeds player 2 at entry; a
                // context built by an older caller degrades to one player.
                return CoopFrameResult::default();
            };
            step_two_player_live_level_frame(
                LiveLevelFrame {
                    sim: &mut self.sim,
                    cam: &mut self.cam,
                    obj: &mut self.obj,
                    scene_vm: &mut self.scene_vm,
                    ending_sequence: &mut self.ending_sequence,
                    ending_scene: &mut self.ending_scene,
                    credits_text: (!self.credits_text.is_empty())
                        .then_some(self.credits_text.as_slice()),
                    manifest: &mut self.manifest,
                    layout_live: &mut self.layout_live,
                    bg_live: &mut self.bg_live,
                    spr: &mut self.spr,
                    scripts: &self.scripts,
                    tilesets: &self.tilesets,
                    arena_layout: self.arena_layout.as_ref(),
                    rom_tables: self.rom_tables.as_ref(),
                    anim_rows: &self.anim_rows,
                    tile_animation: &mut self.tile_animation,
                },
                head,
                input.div,
                &mut second.sim,
                second_input,
                first_bubbled,
                second_bubbled,
                first_bubble,
                second_bubble,
            )
        };
        if result.transition_finished {
            self.scene_reload_blank = false;
        }
        if let Some(definition) = result.boss_definition {
            self.boss_definition = Some(definition);
            if definition != 7 {
                self.dx_regional.palette_load();
            }
        }
        if result.active_objects {
            self.active_object_frames += 1;
        }
        if let Some(reload) = result.arena_reload {
            self.scene_reload_blank = true;
            self.arena_loaded |= reload.applied;
        }
        if self.sim.exit_phase {
            if let Some(second) = self.second.as_mut() {
                second.sim.exit_phase = true;
            }
        }
        if self.sim.exit_phase {
            // Keep player 2 with the shared transition; its own fade is
            // player 1's (the session's hand-off owner).
            if let Some(second) = self.second.as_mut() {
                second.sim.fade_timer = 0;
            }
        }
        // Co-op parties: NSMBW bubbles instead of deaths; only a party wipe
        // restarts the entry checkpoint.
        let wiped = if head.gameplay && !self.sim.paused && !self.sim.exit_phase {
            self.step_bubbles(
                result.bubble,
                [input.first_pressed.select, input.second_pressed.select],
            )
        } else {
            false
        };
        self.adopt_second_exit();
        self.refresh_coop_sprites();
        self.share_layout_writes();
        let flash = if self.scene_reload_blank {
            Some(0)
        } else if self.sim.paused {
            Some(self.sim.pause_flash())
        } else {
            None
        };
        if let Some(s) = self.spr.as_mut() {
            s.player_behind_bg = player_behind_bg(self.sim.pipe_dir);
            s.obp0 = flash.unwrap_or(self.manifest.header.palettes[1]);
            s.obp1 = if self.scene_reload_blank {
                0
            } else {
                self.manifest.header.palettes[2]
            };
            if self.cam.boss_mode() {
                s.boss_draws.clone_from(&self.obj.boss_draws);
            } else {
                s.boss_draws.clear();
            }
        }
        self.tile_animation_copied |= result.tile_animation_copied;
        if self.sim.exit_phase && self.bell_arm.is_none() {
            self.bell_arm = Some(BellArm {
                frame: self.frame_n,
                position: (self.sim.x, self.sim.y),
                fade_timer: self.sim.fade_timer,
                music_request: self.sim.music_request,
                exit_tile_flag: self.sim.exit_tile_flag,
                dead: self.sim.dead,
            });
        }
        if head.gameplay && !head.frozen && self.sim.dead != 0 {
            self.gameplay_death_frames += 1;
        }
        if let Some(second) = self.second.as_ref() {
            if head.gameplay && !head.frozen && second.sim.dead != 0 {
                self.gameplay_death_frames += 1;
            }
        }
        if self.sim.exit_completed && self.exit_completed_frame.is_none() {
            self.exit_completed_frame = Some(self.frame_n);
        }
        if self.sim.map_handoff_done && self.map_handoff_frame.is_none() {
            self.map_handoff_frame = Some(self.frame_n);
        }
        CoopFrameResult {
            restart_checkpoint: wiped,
            game_over: false,
        }
    }

    /// Read-only render context, for hosts and diagnostics that need the
    /// per-frame sprite draw data without touching simulation state.
    pub fn sprite_ctx(&self) -> Option<&SpriteCtx> {
        self.spr.as_ref()
    }

    /// True if the level has completed fade-out and finished the handoff.
    pub fn is_finished(&self) -> bool {
        self.sim.map_handoff_done || (self.sim.game_over && self.sim.fade_timer == 0)
    }
}

#[cfg(test)]
#[path = "tests/dx_palette_timing.rs"]
mod dx_palette_timing_tests;
