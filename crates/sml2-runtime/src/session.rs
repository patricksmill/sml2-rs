//! Multi-state overworld runtime session coordinating map navigation,
//! level entry, and clear/death returns.

pub use crate::content::SessionAssets;
// Compatibility facade for existing CLI and tests; new hosts can own Session directly.
pub use crate::host::{AudioClock, SessionRunner};

use sml2_core::audio::driver::SoundDriver;
use sml2_core::camera::CameraSim;
use sml2_core::ending_entry::{EndingEntry, EndingFrame, EndingInput};
use sml2_core::ending_scene::EndingScene;
use sml2_core::ending_sequence::EndingSequenceController;
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest};
use sml2_core::map::MapNodes;
use sml2_core::map_sim::MapSim;
use sml2_core::object::ObjectTables;
use sml2_core::objects::{FireHit, ObjectSim};
use sml2_core::player_sim::{MapHandoffOutcome, PlayerKeys, PlayerSim};
use sml2_core::scene::{boss_def_index, boss_def_portable, SceneScripts, SceneStep, SceneVm};
use sml2_core::tileset::Tilesets;

use crate::game::{
    apply_level_entry, build_contact, note_handoff_screen, sample_level_player_inputs,
    scene_frame_head, seed_level_timer, BellArm,
};
use crate::render::{SpriteCtx, SpriteInst};

/// Pack boolean keyboard inputs into the Game Boy hardware joypad register format
/// consumed by `sml2_core::map_sim::MapInput`.
#[allow(clippy::too_many_arguments)]
pub fn pack_joypad(
    right: bool,
    left: bool,
    up: bool,
    down: bool,
    a: bool,
    b: bool,
    select: bool,
    start: bool,
) -> u8 {
    (if right { 0x10 } else { 0 })
        | (if left { 0x20 } else { 0 })
        | (if up { 0x40 } else { 0 })
        | (if down { 0x80 } else { 0 })
        | (if a { 0x01 } else { 0 })
        | (if b { 0x02 } else { 0 })
        | (if select { 0x04 } else { 0 })
        | (if start { 0x08 } else { 0 })
}

/// In-level simulation state and working buffers for a live level play.
pub struct LevelContext {
    pub manifest: LevelManifest,
    pub layout: LevelLayout,
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
    pub ending_entry: EndingEntry,
    pub ending_sequence: EndingSequenceController,
    pub ending_scene: EndingScene,
    pub scripts: SceneScripts,
    pub tilesets: Tilesets,
    pub arena_layout: Option<LevelLayout>,
    pub anim_timer: u32,
    pub anim_phase: u8,
    pub frame_n: u64,
    pub map_handoff: Option<MapHandoffOutcome>,
    pub map_handoff_frame: Option<u64>,
    pub exit_completed_frame: Option<u64>,
    pub(crate) bell_arm: Option<BellArm>,
    pub active_object_frames: usize,
    pub gameplay_death_frames: usize,
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
        apply_level_entry(&mut sim, &mut cam, level_entry);
        cam.prev_mario_y_lo = sim.y as u8;
        cam.map_scroll_limit = manifest.properties.unknown != 0;
        cam.auto_scroll = manifest.properties.auto_scroll;
        cam.scene_state = manifest.header.scene_state;

        if let Some(edge) = content.edge_map {
            cam.edge_map = edge;
        }
        if let Some(scene) = &content.scene_map {
            cam.scene_map.clone_from(scene);
        }

        let mut obj = ObjectSim::new(manifest.spawns.records.clone(), cam.x);
        obj.configure(level_id);

        let spr = Some(SpriteCtx {
            insts: Vec::new(),
            objblock,
            tables: object_tables,
            level_id,
            obp0: manifest.header.palettes[1],
            obp1: manifest.header.palettes[2],
            player: Some((sim.x, sim.y, 0)),
            player_behind_bg: false,
            player_shield: None,
            player_pan: 0,
            scroll_pan: 0,
            bumpers: Vec::new(),
            fireballs: Vec::new(),
            boss_oam: Vec::new(),
        });

        Self {
            layout_live: layout.clone(),
            bg_live: bg_base.clone(),
            manifest,
            layout,
            bg_base,
            anim_rows,
            hud,
            hud_y: 136,
            spr,
            sim,
            cam,
            obj,
            scene_vm: SceneVm::new(),
            ending_entry: EndingEntry::default(),
            ending_sequence: EndingSequenceController::new(),
            ending_scene: EndingScene::default(),
            scripts,
            tilesets,
            arena_layout,
            anim_timer: 0,
            anim_phase: 0,
            frame_n: 0,
            map_handoff: None,
            map_handoff_frame: None,
            exit_completed_frame: None,
            bell_arm: None,
            active_object_frames: 0,
            gameplay_death_frames: 0,
        }
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
        self.sim.camx = self.cam.x;
        self.sim.camy = self.cam.y;
        self.sim.pan_timer = self.cam.pan_timer;
        self.sim.phase = (self.sim.phase + 1) & 7;
        self.sim.frame_counter = self.sim.frame_counter.wrapping_add(1);

        let ending_frame = scene_frame_head(
            &mut self.scene_vm,
            &mut self.ending_entry,
            &mut self.sim,
            &mut self.cam,
            EndingInput {
                held: keys,
                pressed: PlayerKeys {
                    a: a_pressed,
                    b: b_pressed,
                    start: start_pressed,
                    select: select_pressed,
                    ..PlayerKeys::default()
                },
            },
        );
        let gameplay = matches!(ending_frame, Some(EndingFrame::Gameplay(_)));
        let input = match ending_frame {
            Some(EndingFrame::Gameplay(input)) => input,
            _ => EndingInput::default(),
        };
        let keys = input.held;
        let (a_pressed, b_pressed) = (input.pressed.a, input.pressed.b);
        let (start_pressed, select_pressed) = (input.pressed.start, input.pressed.select);
        let was_dead = self.sim.dead == 0xFF;
        let frozen = !gameplay || self.sim.paused || self.sim.dead_fade || self.sim.exit_phase;
        let exit_done_at_entry = self.sim.exit_completed;

        let obj_tables = self.spr.as_ref().map(|s| s.tables.clone());

        if !gameplay {
            if let Some(EndingFrame::SequencePending) = ending_frame {
                let res =
                    self.ending_sequence
                        .step(&mut sml2_core::ending_sequence::EndingStepContext {
                            scene: &mut self.ending_scene,
                            rom: None,
                            credits_text: None,
                            object_tables: obj_tables.as_ref(),
                            decrement_fade: true,
                        });
                if res.sfx_request_0 != 0 {
                    self.sim.sfx0 = res.sfx_request_0;
                }
                if res.sfx_request_1 != 0 {
                    self.sim.sfx1 = res.sfx_request_1;
                }
            }
        } else if frozen {
            if self.sim.exit_phase {
                self.sim.exit_fade_step();
            } else if self.sim.dead_fade {
                self.sim.fade_step();
                if self.sim.fade_timer == 0 {
                    self.sim.map_handoff_done = true;
                }
            }
            if exit_done_at_entry {
                let handoff = self.sim.map_handoff(nodes);
                note_handoff_screen(&handoff);
                if self.map_handoff.is_none() {
                    self.map_handoff = Some(handoff);
                }
            }
        } else if was_dead {
            self.sim.death_arc(self.cam.y);
        } else {
            if self.cam.transition == 0 {
                self.cam.crouch_pan(&self.sim, &keys);
                self.sim
                    .step(&keys, a_pressed, b_pressed, &self.layout_live);
            } else {
                self.sim.tick_fade_timer();
            }
            if self.sim.take_stomp_chain_reset() {
                self.obj.stomp_chain = 0;
            }
            if let Some((by, bx)) = self.sim.below_bonk.take() {
                let special = self
                    .obj
                    .below_bonk(by, bx, self.cam.scroll_y, self.cam.scroll_x);
                self.sim.resolve_bump_coin(special);
            }
            if let Some((by, bx)) = self.sim.fire_below_bonk.take() {
                self.obj
                    .below_bonk(by, bx, self.cam.scroll_y, self.cam.scroll_x);
            }
            for i in 0..self.sim.fire_scan.len() {
                let Some((sy, sx)) = self.sim.fire_scan[i].take() else {
                    continue;
                };
                let hit = self.obj.fire_hit(sy, sx);
                if hit == FireHit::Die {
                    self.sim.sfx0 = 2;
                }
                self.sim.resolve_fire_hit(i, hit != FireHit::None);
            }
            self.cam.step(&mut self.sim);
            self.cam.update_enables(self.sim.x);
        }
        self.cam.update_player_pan(self.sim.y, was_dead, !frozen);

        if let Some(tables) = &obj_tables {
            if !frozen {
                let mario = build_contact(&self.sim, &keys, self.cam.scroll_pan);
                sample_level_player_inputs(&mut self.obj, &self.sim);
                self.obj.div = div;
                if self.cam.scene_load_pending {
                    let def_index =
                        boss_def_index(self.manifest.level_id, self.cam.boss_fight_phase);
                    if boss_def_portable(def_index) {
                        if let Some((music, level_song)) =
                            self.cam.begin_scene_load(self.manifest.level_id)
                        {
                            self.sim.music_request = music;
                            self.sim.level_song = level_song;
                            self.obj.spawn_boss(def_index);
                        }
                    } else if self.cam.take_blocked_scene_load() {
                        self.obj.note_blocked_scene_load();
                    }
                }
                self.obj.step(
                    self.cam.x,
                    self.cam.y,
                    &self.layout_live,
                    tables,
                    &mario,
                    self.cam.boss_mode(),
                );
                if self
                    .obj
                    .slots
                    .iter()
                    .any(|slot| slot.state() == sml2_core::objects::SlotState::Active)
                {
                    self.active_object_frames += 1;
                }
                self.sim.apply_object_result(&mut self.obj, keys.a);
                self.cam.apply_object_result(&mut self.obj);
            }
        }

        let stepped = if gameplay || ending_frame.is_none() {
            self.scene_vm
                .step(&mut self.sim, &mut self.cam, &self.scripts)
        } else {
            SceneStep::Idle
        };
        if let SceneStep::Ran(op) = stepped {
            if op >> 4 == 0x6 {
                let index = self.sim.level_gfx_index;
                let obj_tiles = self.spr.as_mut().map(|s| &mut s.objblock);
                let _ = match obj_tiles {
                    Some(o) => self.tilesets.apply(
                        index,
                        &mut self.bg_live,
                        o,
                        &mut self.manifest.metatiles,
                    ),
                    None => false,
                };
                if let Some(a) = self.arena_layout.as_ref() {
                    self.layout_live = a.clone();
                }
            }
        }

        let f = if was_dead {
            107
        } else {
            self.sim.frame_index(self.sim.walk_timer)
        };
        if gameplay && !was_dead {
            self.sim.block_update();
        }

        if let Some(s) = self.spr.as_mut() {
            s.player = Some((self.sim.x, self.sim.y, f));
            // `$530D`: pipe travel ($A221 >= $10) clears `$FFBB`, so the
            // `$5297` expander sets attr bit 7 and the pipe's BG tiles cover
            // Mario instead of him being drawn over them.
            s.player_behind_bg = player_behind_bg(self.sim.pipe_dir);
            s.player_shield = if was_dead {
                None
            } else {
                self.sim
                    .moon_shield_frame()
                    .map(|sf| (self.sim.x, self.sim.y, sf))
            };
            s.player_pan = self.cam.player_pan;
            s.scroll_pan = self.cam.scroll_pan;
            if was_dead {
                s.bumpers.clear();
            } else {
                s.bumpers = self.sim.block_draw.clone();
            }
            s.fireballs = self.sim.fire_draw.clone();
            if self.cam.boss_mode() {
                s.insts.clear();
                s.boss_oam = self.obj.boss_oam.clone();
            } else {
                s.boss_oam.clear();
                s.insts = self
                    .obj
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(i, o)| {
                        o.state() == sml2_core::objects::SlotState::Active
                            && !self.obj.slot_done[*i]
                    })
                    .map(|(_, o)| SpriteInst {
                        sprite: o.sprite(),
                        x: o.x(),
                        y: o.y(),
                        flip: o.b[sml2_core::objects::B_DEFEAT]
                            ^ o.b[sml2_core::objects::B_FLIP]
                            ^ o.b[sml2_core::objects::B_FLASH],
                    })
                    .collect();
            }
        }

        if gameplay && self.sim.fade_timer == 0 && self.cam.transition == 0 {
            if start_pressed {
                if self.sim.paused {
                    self.sim.pause_exit();
                } else {
                    self.sim.pause_enter();
                }
            } else if self.sim.paused && select_pressed {
                self.sim.pause_select_exit();
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
        if gameplay && !frozen && self.sim.dead == 0xFF {
            self.gameplay_death_frames += 1;
        }
        if self.sim.exit_completed && self.exit_completed_frame.is_none() {
            self.exit_completed_frame = Some(self.frame_n);
        }
        if self.sim.map_handoff_done && self.map_handoff_frame.is_none() {
            self.map_handoff_frame = Some(self.frame_n);
        }

        let flash = if self.sim.paused {
            Some(self.sim.pause_flash())
        } else {
            None
        };
        if let Some(s) = self.spr.as_mut() {
            s.obp0 = flash.unwrap_or(self.manifest.header.palettes[1]);
        }
        self.anim_timer += 1;
        if self.manifest.tile_anim.enabled
            && self.anim_timer >= self.manifest.tile_anim.speed as u32 * 2
        {
            self.anim_timer = 0;
            self.anim_phase = (self.anim_phase + 1) & 3;
        }
    }

    /// True if the level has completed fade-out and finished the handoff.
    pub fn is_finished(&self) -> bool {
        self.sim.map_handoff_done
    }
}

/// Every nondeterministic gameplay input is supplied by the host.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameInput {
    pub keys: PlayerKeys,
    pub div: u8,
}

/// Effects are data: the host decides when and where to apply them.
pub struct FrameEffects {
    pub outcome: StepOutcome,
    pub saves: Vec<[u8; 0x50]>,
    pub diagnostics: Vec<String>,
}

/// The three major modes of the runtime session.
pub enum SessionState {
    Overworld(MapSim),
    InLevel(Box<LevelContext>),
    GameOver,
}

/// Outcome of a single stepped frame in `SessionRunner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Continue,
    LevelEntered(u8),
    LevelExited { node: u8, screen: u8, cleared: bool },
    GameOver,
}

/// A level entry requested from the map, held for the console's own transition
/// before the level becomes live.
pub struct MapTransition {
    pub ctx: Box<LevelContext>,
    pub level_id: u8,
    pub frame: u16,
}

/// Frames from the first faded map frame to the first level frame: PyBoy shows
/// $FF9B $0C at the last normal map frame, $0B one frame after the palette hits
/// white, $02 for two frames, $03 for twenty, and the first $04 frame 37 frames
/// after the fade started.
pub const MAP_ENTRY_FRAMES: u16 = 37;

/// Frames the map spends in the first fade step before the palette reaches
/// white: `rBGP` $90 -> $40 at the fade start and -> $00 twelve frames later.
pub const MAP_FADE_STEP: u16 = 12;

/// `$530D`: `sPipeTravelDirection >= $10` clears `$FFBB`, which makes the
/// `$5297` expander set attr bit 7 on every player entry - Mario goes behind
/// the background, so the pipe he is travelling through covers him. Every
/// other state leaves `$FFBB` set (or restores it) and draws him in front.
pub(crate) fn player_behind_bg(pipe_dir: u8) -> bool {
    pipe_dir >= 0x10
}

/// Native session state. Advancing it performs no I/O or clock sampling.
pub struct Session {
    pub state: SessionState,
    /// Screen whose loader-run music request was last applied, so a screen's
    /// song is requested once per load instead of every frame.
    pub last_map_screen: Option<u8>,
    /// `$3E853`'s hub shimmer, seeded when the hub screen loads.
    pub map_wave: Option<sml2_core::map::MapWave>,
    /// Set while a map level entry runs its 37-frame transition.
    pub pending_level: Option<Box<MapTransition>>,
    pub assets: SessionAssets,
    pub lives: u8,
    pub powerup: u8,
    pub coins_low: u8,
    pub coins_high: u8,
    pub level_tally: u8,
    pub money_bag: u8,
    pub prev_keys_held: u8,
    pub prev_a: bool,
    pub prev_b: bool,
    pub prev_start: bool,
    pub prev_select: bool,
    pub frame_n: u64,
    pub sound_driver: SoundDriver,
}

impl Session {
    pub fn new(assets: SessionAssets) -> Self {
        let mut map = MapSim::new();
        map.load_screen(0, 0, &assets.node_records);
        let sound_driver = SoundDriver::with_music_bank(assets.music_bank.clone());
        Self {
            state: SessionState::Overworld(map),
            last_map_screen: None,
            map_wave: None,
            pending_level: None,
            assets,
            lives: 5,
            powerup: 0,
            coins_low: 0,
            coins_high: 0,
            level_tally: 0,
            money_bag: 0,
            prev_keys_held: 0,
            prev_a: false,
            prev_b: false,
            prev_start: false,
            prev_select: false,
            frame_n: 0,
            sound_driver,
        }
    }

    /// Apply an already-read ROM-format progress record.
    pub fn apply_save_record(&mut self, data: &[u8]) -> bool {
        if !sml2_core::map::save_record_valid(data) {
            return false;
        }
        let mut map = MapSim::new();
        if map.apply_save_record(data, &self.assets.node_records) {
            self.lives = data[0x43];
            self.coins_low = data[0x40];
            self.coins_high = data[0x41];
            self.level_tally = data[0x42];
            self.money_bag = data[0x44];
            self.state = SessionState::Overworld(map);
            true
        } else {
            false
        }
    }

    /// Produce progress bytes without writing them.
    pub fn save_record(&self) -> [u8; 0x50] {
        match &self.state {
            SessionState::Overworld(map) => {
                map.save_record(self.lives, self.level_tally, self.money_bag)
            }
            SessionState::InLevel(ctx) => ctx.sim.save_record(self.money_bag),
            SessionState::GameOver => {
                let map = MapSim::new();
                map.save_record(self.lives, self.level_tally, self.money_bag)
            }
        }
    }

    /// Advance exactly one tick from explicit inputs, emitting deferred host effects.
    pub fn advance(&mut self, input: FrameInput) -> FrameEffects {
        let FrameInput { keys, div } = input;
        self.frame_n += 1;
        let mut saves = Vec::new();
        let mut diagnostics = Vec::new();

        let a_pressed = keys.a && !self.prev_a;
        self.prev_a = keys.a;
        let b_pressed = keys.b && !self.prev_b;
        self.prev_b = keys.b;
        let start_pressed = keys.start && !self.prev_start;
        self.prev_start = keys.start;
        let select_pressed = keys.select && !self.prev_select;
        self.prev_select = keys.select;

        let mut outcome = match &mut self.state {
            SessionState::Overworld(ref mut map_sim) => 'map: {
                // A level entry from the map is the console's own 37-frame
                // transition (measured: $0C at f66 -> $0B f80 -> $02 f81 ->
                // $03 f83 -> $04 f103). The map handler stops running, so the
                // cursor freezes while the palettes fade and the level loads.
                if let Some(t) = self.pending_level.as_mut() {
                    t.frame += 1;
                    break 'map StepOutcome::Continue;
                }
                let keys_held = pack_joypad(
                    keys.right,
                    keys.left,
                    keys.up,
                    keys.down,
                    keys.a,
                    keys.b,
                    keys.select,
                    keys.start,
                );
                let keys_pressed = keys_held & !self.prev_keys_held;
                self.prev_keys_held = keys_held;

                let input = sml2_core::map_sim::MapInput::new(keys_held, keys_pressed, div);

                let entry = map_sim.tick(
                    input,
                    &self.assets.nodes,
                    &self.assets.walk,
                    &self.assets.node_records,
                );

                if let Some(music) = entry.music {
                    self.sound_driver.music_request = music;
                }
                if let Some(sfx) = entry.sfx {
                    self.sound_driver.sfx_request_1 = sfx;
                }

                // `$3EAFF`: only the hub runs the wave strips, and `$3E7BE`
                // seeds them from the tiles the loader just copied.
                if map_sim.screen_id == 0 {
                    if self.map_wave.is_none() {
                        self.map_wave = sml2_core::map::MapWave::seed(
                            self.assets.overworld_tiles.screen_raw(0),
                        );
                    }
                    if let Some(wave) = self.map_wave.as_mut() {
                        wave.tick();
                    }
                } else {
                    self.map_wave = None;
                }

                if map_sim.pending_save || entry.saved {
                    map_sim.pending_save = false;
                    let rec = map_sim.save_record(self.lives, self.level_tally, self.money_bag);
                    saves.push(rec);
                }

                if let Some(level_id) = entry.level {
                    match self.assets.level(level_id).map(|content| {
                        LevelContext::from_content(
                            level_id,
                            content,
                            entry.progress_flags,
                            self.lives,
                            map_sim.coins_low,
                            map_sim.coins_high,
                            self.powerup,
                            map_sim.node,
                            self.assets.scripts.clone(),
                            self.assets.tilesets.clone(),
                            self.assets.arena_layout.clone(),
                            self.assets.anim_rows.clone(),
                            self.assets.hud.clone(),
                            self.assets.object_tables.clone(),
                        )
                    }) {
                        Ok(ctx) => {
                            // $3C1A2 stops the map theme before loading, and the
                            // console spends the next 37 frames fading the map
                            // to white with the level loading behind a blank
                            // LCD; the level goes live at the end of that.
                            self.sound_driver.music_request = 0xFF;
                            self.pending_level = Some(Box::new(MapTransition {
                                ctx: Box::new(ctx),
                                level_id,
                                frame: 0,
                            }));
                            StepOutcome::Continue
                        }
                        Err(e) => {
                            diagnostics.push(format!("failed to load level {level_id}: {e}"));
                            StepOutcome::Continue
                        }
                    }
                } else {
                    StepOutcome::Continue
                }
            }

            SessionState::InLevel(ref mut ctx) => {
                ctx.step_frame(
                    keys,
                    a_pressed,
                    b_pressed,
                    start_pressed,
                    select_pressed,
                    div,
                    &self.assets.nodes,
                );

                if ctx.sim.sfx0 != 0 {
                    self.sound_driver.sfx_request_0 = ctx.sim.sfx0;
                    ctx.sim.sfx0 = 0;
                }
                if ctx.sim.sfx1 != 0 {
                    self.sound_driver.sfx_request_1 = ctx.sim.sfx1;
                    ctx.sim.sfx1 = 0;
                }
                if ctx.sim.sfx2 != 0 {
                    self.sound_driver.sfx_request_2 = ctx.sim.sfx2;
                    ctx.sim.sfx2 = 0;
                }
                if ctx.sim.music_request != 0 {
                    self.sound_driver.music_request = ctx.sim.music_request;
                    ctx.sim.music_request = 0;
                }
                self.sound_driver.stomp_chain = ctx.obj.stomp_chain;
                self.sound_driver.low_time_flag = ctx.sim.low_time_flag != 0;

                if ctx.is_finished() {
                    let handoff = ctx
                        .map_handoff
                        .unwrap_or_else(|| ctx.sim.map_handoff(&self.assets.nodes));

                    if ctx.sim.dead != 0 {
                        // Player died in level
                        if ctx.sim.lives == 0 || ctx.sim.game_over || ctx.sim.lives == 0x99 {
                            self.state = SessionState::GameOver;
                            StepOutcome::GameOver
                        } else {
                            self.lives = ctx.sim.lives;
                            self.powerup = 0; // Death resets powerup
                            let target_node = handoff.node;
                            let target_screen = handoff.screen.unwrap_or(0);
                            let mut map_sim = MapSim::from_player(&ctx.sim);
                            map_sim.mario_dead = 0xFF;
                            map_sim.load_screen(
                                target_screen,
                                ctx.sim.exit_tile_flag,
                                &self.assets.node_records,
                            );
                            map_sim.node = target_node;
                            map_sim.load_node(target_node, &self.assets.node_records);
                            self.state = SessionState::Overworld(map_sim);
                            if handoff.saves > 0 {
                                saves.push(self.save_record());
                            }
                            StepOutcome::LevelExited {
                                node: target_node,
                                screen: target_screen,
                                cleared: false,
                            }
                        }
                    } else {
                        // Player cleared level
                        self.lives = ctx.sim.lives;
                        self.coins_low = ctx.sim.coins_low;
                        self.coins_high = ctx.sim.coins_high;
                        self.level_tally = ctx.sim.level_tally;
                        self.powerup = ctx.sim.powerup;
                        let target_node = handoff.node;
                        let target_screen = handoff.screen.unwrap_or(0);
                        let mut map_sim = MapSim::from_player(&ctx.sim);
                        map_sim.load_screen(
                            target_screen,
                            ctx.sim.exit_tile_flag,
                            &self.assets.node_records,
                        );
                        map_sim.node = target_node;
                        map_sim.load_node(target_node, &self.assets.node_records);
                        self.state = SessionState::Overworld(map_sim);
                        if handoff.saves > 0 {
                            saves.push(self.save_record());
                        }
                        StepOutcome::LevelExited {
                            node: target_node,
                            screen: target_screen,
                            cleared: true,
                        }
                    }
                } else {
                    StepOutcome::Continue
                }
            }

            SessionState::GameOver => {
                self.lives = 5;
                self.powerup = 0;
                let mut map_sim = MapSim::new();
                map_sim.load_screen(0, 0, &self.assets.node_records);
                self.state = SessionState::Overworld(map_sim);
                StepOutcome::Continue
            }
        };

        // The frozen map arm advances the counter; once it reaches the console's
        // entry length the level goes live (its own music request starts on the
        // first InLevel frame).
        if let Some(t) = self.pending_level.as_mut() {
            if t.frame >= MAP_ENTRY_FRAMES {
                let t = self.pending_level.take().expect("checked above");
                self.state = SessionState::InLevel(t.ctx);
                outcome = StepOutcome::LevelEntered(t.level_id);
            }
        }

        // The map screen loaders each request their own song on entry
        // ($3D0FF..$3D880) and call $2AAA right after, so the request is applied
        // when a screen is loaded (not every frame) and starts this same frame.
        if let SessionState::Overworld(map) = &self.state {
            if self.last_map_screen != Some(map.screen_id) {
                self.last_map_screen = Some(map.screen_id);
                self.sound_driver.music_request = sml2_core::map_sim::screen_music(map.screen_id);
            }
        } else {
            self.last_map_screen = None;
        }

        self.sound_driver.update_sound();
        // The ROM reads `sCurSong` from the level timer's music gate ($5DCB) on
        // the frame AFTER `_UpdateSound` wrote it, so publish the driver's song
        // id after the update, never before it.
        if let SessionState::InLevel(ref mut ctx) = self.state {
            ctx.sim.current_song = self.sound_driver.active_song;
        }
        FrameEffects {
            outcome,
            saves,
            diagnostics,
        }
    }
}
