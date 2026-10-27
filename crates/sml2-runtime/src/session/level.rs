//! Live level state and its ordered simulation tick.

use super::{
    apply_level_entry, note_handoff_screen, player_behind_bg, seed_level_timer, BellArm, CameraSim,
    EndingEntry, EndingInput, EndingScene, EndingSequenceController, HudStrip, LevelLayout,
    LevelManifest, MapHandoffOutcome, MapNodes, ObjectClock, ObjectSim, ObjectTables, PlayerKeys,
    PlayerSim, SceneScripts, SceneVm, SpriteCtx, TileAnimationClock, Tilesets,
};

use crate::game::level_frame::{begin_live_level_frame, step_live_level_frame, LiveLevelFrame};

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
        ObjectClock::default().initialize_level(&mut obj);

        let spr = Some(SpriteCtx {
            insts: Vec::new(),
            objblock,
            tables: object_tables,
            level_id,
            obp0: manifest.header.palettes[1],
            obp1: manifest.header.palettes[2],
            player: Some((sim.x, sim.y, 0)),
            player_behind_bg: false,
            pipe_background: None,
            player_star_flash: false,
            player_shield: None,
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
        self.dx_regional
            .update(self.manifest.level_id, self.sim.x, self.sim.y);
        self.tile_animation_copied |= result.tile_animation_copied;
    }

    /// True if the level has completed fade-out and finished the handoff.
    pub fn is_finished(&self) -> bool {
        self.sim.map_handoff_done || (self.sim.game_over && self.sim.fade_timer == 0)
    }
}
