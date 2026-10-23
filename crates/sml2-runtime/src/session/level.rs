//! Live level state and its ordered simulation tick.

use super::{
    note_handoff_screen, player_behind_bg, BOSS_LOAD_VBLANKS, Tilesets,
    apply_level_entry, boss_def_index, boss_def_portable, build_contact,
    sample_level_player_inputs, scene_frame_head, seed_level_timer, update_tile_animation,
    BellArm, CameraSim, EndingEntry, EndingFrame, EndingInput, EndingScene,
    EndingSequenceController, FireHit, HudStrip, LevelLayout, LevelManifest,
    MapHandoffOutcome, MapNodes, ObjectClock, ObjectSim, ObjectTables, PlayerKeys,
    PlayerSim, SceneScripts, SceneStep, SceneVm, SpriteCtx, SpriteInst, TileAnimationClock,
};

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
    /// `$25AF` has swapped in a boss arena's tileset. Presentation only: DX
    /// draws arenas with their own tables, which are not imported yet.
    pub arena_loaded: bool,
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
            arena_loaded: false,
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
        // `$05D5` runs after this frame's VBlank decrement and before `$20A4`'s
        // scene, and clears `hKeysHeld`/`hKeysPressed` while `sFadeTimer` is
        // nonzero, so the arena intro holds Mario still.
        let fade_locks_input = self.sim.fade_timer > 1;
        // `$5E12` (inside `$2C61`) writes its scripted keys after that clear,
        // so the lock masks only the joypad the ending head may replace.
        let joypad = if fade_locks_input {
            EndingInput::default()
        } else {
            EndingInput {
                held: keys,
                pressed: PlayerKeys {
                    a: a_pressed,
                    b: b_pressed,
                    start: start_pressed,
                    select: select_pressed,
                    ..PlayerKeys::default()
                },
            }
        };

        let ending_frame = scene_frame_head(
            &mut self.scene_vm,
            &mut self.ending_entry,
            &mut self.sim,
            &mut self.cam,
            joypad,
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
                            credits_text: (!self.credits_text.is_empty())
                                .then_some(self.credits_text.as_slice()),
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
            let transition_before = self.cam.transition;
            self.cam.step(&mut self.sim);
            if transition_before != 0 && self.cam.transition == 0 {
                self.scene_reload_blank = false;
            }
            self.cam.update_enables(self.sim.x);
        }
        self.cam.update_player_pan(self.sim.y, was_dead, !frozen);

        if let Some(tables) = &obj_tables {
            if !frozen {
                let mario = build_contact(&self.sim, &keys, self.cam.scroll_pan, self.cam.scene_state);
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
                            // `$3C94` runs inside this update and outlasts its
                            // frame: a VBlank lands before it returns and
                            // decrements `sFadeTimer` with no update of its own.
                            self.sim.fade_timer = self.sim.fade_timer.saturating_sub(BOSS_LOAD_VBLANKS);
                        }
                    } else if self.cam.take_blocked_scene_load() {
                        self.obj.note_blocked_scene_load();
                    }
                }
                self.obj.sample_camera_inputs(&self.cam);
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

        let scene_camera = (self.cam.x, self.cam.y);
        let stepped = if gameplay || ending_frame.is_none() {
            self.scene_vm
                .step(&mut self.sim, &mut self.cam, &self.scripts)
        } else {
            SceneStep::Idle
        };
        crate::game::apply_scene_transfer(&mut self.scene_vm, &mut self.bg_live, self.spr.as_mut());
        if let Some(s) = self.spr.as_mut() {
            crate::render::PipeBackground::update(
                &mut s.pipe_background,
                scene_camera,
                &self.cam,
                stepped,
            );
        }
        if let SceneStep::Ran(op) = stepped {
            if op >> 4 == 0x6 {
                // `$266D` clears BGP, OBP0 and OBP1 before re-enabling LCD.
                self.scene_reload_blank = true;
                let index = self.sim.level_gfx_index;
                let obj_tiles = self.spr.as_mut().map(|s| &mut s.objblock);
                self.arena_loaded |= match obj_tiles {
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
                    // `$25AF` calls `$0361`, replacing the decompressed map.
                    self.sim.layout_writes.clear();
                }
            }
        }

        let f = if was_dead {
            107
        } else {
            // `$530D` only runs on simulated frames. Its walk request must not
            // leak out of a paused, dead-fade or exit-fade frame — the tone
            // would sustain for as long as Mario's last speed byte survives.
            let held_sfx0 = self.sim.sfx0;
            let f = self.sim.frame_index(self.sim.walk_timer);
            if frozen || held_sfx0 != 0 {
                self.sim.sfx0 = held_sfx0;
            }
            f
        };
        if gameplay && !was_dead {
            self.sim.block_update();
        }
        let streak = if gameplay && !was_dead {
            self.obj.tick_stomp_bounce_timer()
        } else {
            None
        };

        if let Some(s) = self.spr.as_mut() {
            s.player = Some((self.sim.x, self.sim.y, f));
            s.streak_counter = streak;
            // `$530D`: pipe travel ($A221 >= $10) clears `$FFBB`, so the
            // `$5297` expander sets attr bit 7 and the pipe's BG tiles cover
            // Mario instead of him being drawn over them.
            s.player_behind_bg = player_behind_bg(self.sim.pipe_dir);
            // `$54C8`: `$FFC7` from the star countdown; the dead draw (`$5550`)
            // does not go through it.
            s.player_star_flash = !was_dead && self.sim.star_flash();
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
                    .map(|(i, o)| SpriteInst {
                        sprite: o.sprite(),
                        x: o.x(),
                        y: o.y(),
                        flip: o.b[sml2_core::objects::B_DEFEAT]
                            ^ o.b[sml2_core::objects::B_FLIP]
                            ^ o.b[sml2_core::objects::B_FLASH],
                        slot: i as u8,
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
        self.tile_animation_copied |= update_tile_animation(
            &self.sim,
            self.cam.scene_id != 0,
            &mut self.bg_live,
            &self.anim_rows,
            &mut self.tile_animation,
        );
    }

    /// True if the level has completed fade-out and finished the handoff.
    pub fn is_finished(&self) -> bool {
        self.sim.map_handoff_done || (self.sim.game_over && self.sim.fade_timer == 0)
    }
}

