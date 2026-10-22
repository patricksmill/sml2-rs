//! Multi-state overworld runtime session coordinating map navigation,
//! level entry, and clear/death returns.

pub use crate::content::SessionAssets;
// Compatibility facade for existing CLI and tests; new hosts can own Session directly.
pub use crate::host::{AudioClock, SessionRunner};

mod castle;
mod debug;
mod object_clock;
use object_clock::ObjectClock;
mod file_select_loading;
pub use file_select_loading::FileSelectLoading;
mod map_pan;
mod map_transition;
pub use map_transition::MapScreenTransition;
pub use map_pan::{MapPanReturn, MAP_PAN_RELOAD_FRAMES, MAP_PAN_RETURN_FRAMES};

use sml2_core::audio::driver::SoundDriver;
use sml2_core::camera::CameraSim;
use sml2_core::ending_entry::{EndingEntry, EndingFrame, EndingInput};
use sml2_core::ending_scene::EndingScene;
use sml2_core::ending_sequence::EndingSequenceController;
use sml2_core::front_end::{
    DemoTape, FileSelect, FileSelectAction, SaveSlots, TitleAction, TitleScreen, DEMO_LEVELS,
};
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest};
use sml2_core::map::MapNodes;
use sml2_core::map_sim::MapSim;
use sml2_core::object::ObjectTables;
use sml2_core::objects::{FireHit, ObjectSim};
use sml2_core::player_sim::{MapHandoffOutcome, PlayerKeys, PlayerSim};
use sml2_core::scene::{boss_def_index, boss_def_portable, SceneScripts, SceneStep, SceneVm};
use sml2_core::tile_animation::TileAnimationClock;
use sml2_core::tileset::Tilesets;

use crate::game::{
    apply_level_entry, build_contact, note_handoff_screen, sample_level_player_inputs, scene_frame_head,
    seed_level_timer, update_tile_animation, BellArm,
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
            Some(EndingFrame::Gameplay(_)) if fade_locks_input => EndingInput::default(),
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
        update_tile_animation(
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

/// Every nondeterministic gameplay input is supplied by the host.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameInput {
    pub keys: PlayerKeys,
    pub div: u8,
}

/// Timing a host observed on the console for one frame (a ROM comparison):
/// the full `$FF97` and every `rDIV` value read, in order. The map's random
/// state then follows the console exactly; without it the port derives both.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RomTiming {
    pub ff97: u8,
    pub div_reads: Vec<u8>,
}

/// Effects are data: the host decides when and where to apply them.
pub struct FrameEffects {
    pub outcome: StepOutcome,
    pub saves: Vec<[u8; 0x50]>,
    pub diagnostics: Vec<String>,
}

/// The three major modes of the runtime session.
pub enum SessionState {
    /// `$00`/`$01`: the title screen.
    Title(TitleScreen),
    /// `$19`: title exit has requested the file-select loader.
    FileSelectLoading(FileSelectLoading),
    /// `$1A`: the file select, after loading has completed.
    FileSelect(FileSelect),
    /// `$0B`: an attract demo level driven by a recorded tape.
    Demo(Box<DemoContext>),
    Overworld(MapSim),
    InLevel(Box<LevelContext>),
    /// `$1B`-`$21`: the golden-coin castle reveal.
    Castle(sml2_core::castle_sequence::CastleSequence),
    /// `$05`/`$07`: the GAME OVER screen.
    GameOver(Box<GameOverScreen>),
}

/// State `$07`'s inputs: the player record `$27AF` reset, which carries the
/// fade, `$FF97` and the folded golden-coin flags, and the level's sprite
/// palettes, which `$27AF` leaves untouched.
#[cfg(test)]
#[path = "session/tests/frame_clock.rs"]
mod frame_clock_tests;

pub struct GameOverScreen {
    pub sim: PlayerSim,
    pub obp: [u8; 2],
}

/// Which state a fresh session starts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boot {
    /// Straight to the overworld, for the existing `--play` path and saves.
    Overworld,
    /// The ROM's boot flow: title -> file select.
    Title,
}

/// Outcome of a single stepped frame in `SessionRunner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Continue,
    LevelEntered(u8),
    LevelExited { node: u8, screen: u8, cleared: bool },
    GameOver,
}

/// `$0B`: a demo level plus its run-length input page.
pub struct DemoContext {
    pub ctx: Box<LevelContext>,
    pub tape: DemoTape,
    pub level_id: u8,
    pub index: u8,
}

/// A level entry requested from the map, held for the console's own transition
/// before the level becomes live.
pub struct MapTransition {
    pub ctx: Box<LevelContext>,
    pub level_id: u8,
    pub frame: u16,
    /// Frame `$3573` hands Easy mode's card to `$35CD` (state `$17`); `None`
    /// without Easy mode, where `$3573` goes straight to `$0B`.
    pub easy_card: Option<u16>,
    /// Frame the level goes live (its first state-`$04` frame).
    pub live_at: u16,
}

/// The map's A press reaches state `$16` here: `$3C1EC` runs after the
/// `$3E9FB` fade, one frame before the measured `$0B`.
const MAP_EASY_CARD_START: u16 = 50;

/// `$0B`, two `$02` frames and the first `$03` frame run the frame wait
/// before the `$03` loader turns VBlank off.
const MAP_ENTRY_TICKING_FRAMES: u16 = 4;

impl MapTransition {
    /// A map entry: `MAP_ENTRY_FRAMES`, or 64 more when the card's `$17`
    /// frames come before `$0B` and push the rest of the load back.
    pub fn from_map(ctx: Box<LevelContext>, level_id: u8, easy: bool) -> Self {
        let card = sml2_core::easy_mode_card::EASY_CARD_FRAMES;
        Self {
            ctx,
            level_id,
            frame: 0,
            easy_card: easy.then_some(MAP_EASY_CARD_START),
            live_at: MAP_ENTRY_FRAMES + if easy { card } else { 0 },
        }
    }

    /// Whether transition frame `frame` runs the `$0300` frame wait: not
    /// during `$3E9FB`'s fade and `$2934` save (`$3E9EC` waits), and not
    /// during the `$03` loader with VBlank off; the `$0B`/`$02` frames and
    /// the Easy card's `$17` frames do.
    pub fn frame_waits(&self, frame: u16) -> bool {
        let card = self
            .easy_card
            .map_or(0, |_| sml2_core::easy_mode_card::EASY_CARD_FRAMES);
        let fade_end = MAP_EASY_CARD_START;
        let loader_start = fade_end + card + MAP_ENTRY_TICKING_FRAMES;
        frame > fade_end && frame <= loader_start
    }

    /// Whether `$35CD` draws the card on the current frame.
    pub fn easy_card_visible(&self) -> bool {
        self.easy_card.is_some_and(|start| {
            (start..start + sml2_core::easy_mode_card::EASY_CARD_FRAMES).contains(&self.frame)
        })
    }
}

/// Frames from the map's A press to the first state-`$04` frame. `$3C1EC`
/// calls `$3E9FB` (four palette steps, twelve `$3E9EC` waits each) and then
/// clears the tilemap before state `$0B`; `$02` lasts two frames and `$03`
/// twenty. Measured live: the frame whose `ReadJoypad` first sees A is 50
/// frames before `$0B` and 73 before `$04`, and the first `$181C` is the frame
/// after `$04`. The read can trail the input row by a frame, which is where an
/// earlier "74" came from. 73 lands the level on that `$04` frame, so the next
/// step is the first player update.
pub const MAP_ENTRY_FRAMES: u16 = 73;

/// VBlank interrupts taken inside the gameplay update that runs the boss load
/// (`$3C94`). Measured on the level-9 arena (2026-10-20 recording): the update
/// spans two frame boundaries, but `sFadeTimer` reads 306 at its `$05D5` and
/// 304 at the next one, so one extra decrement lands in it.
const BOSS_LOAD_VBLANKS: u16 = 1;

/// Frames after the bell script reaches state `$0A` before the map handler
/// runs again: `$3E9FB`'s fade, then `$3EF2B` loads the cleared node's screen.
/// Measured in PyBoy on every bell-room return in the local recordings: the
/// hub (`$3F156` and `$3EA9A`'s fade-in) takes 84 on five returns; screen 6
/// takes 59 on three and screen 1 takes 58 on one. Other zone screens are
/// assumed to load like screen 6.
fn bell_return_hold(screen: u8) -> u16 {
    match screen {
        0 => 84,
        1 => 58,
        _ => 59,
    }
}

/// `$3EF2B` run again for a bell room's return: the progress write and the
/// screen the cleared node belongs to.
fn bell_handoff(
    map_sim: &MapSim,
    node: u8,
    flag: u8,
    nodes: &sml2_core::map::MapNodes,
) -> (PlayerSim, sml2_core::player_sim::MapHandoffOutcome) {
    let mut sim = PlayerSim::new();
    map_sim.to_player(&mut sim);
    sim.map_node = node;
    sim.dead = map_sim.mario_dead;
    sim.exit_tile_flag = flag;
    let handoff = sim.map_handoff(nodes);
    (sim, handoff)
}

/// Frames `$3F4B1` spends in its graphics copy before the bell script runs.
/// The setup begins on the frame state becomes `$14`; the script's first step
/// is eight frames later, so the room waits through the seven frames in between.
const BELL_ENTRY_HOLD: u16 = 7;

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
    object_clock: ObjectClock,
    /// `$A298` and `$A2CD` survive map time and level loads.
    tile_animation: TileAnimationClock,
    /// Console timing for the next `advance` only ([`Session::set_rom_timing`]).
    rom_timing: Option<RomTiming>,
    /// The map work bytes that outlive a `MapSim` and that level loads
    /// overwrite ([`sml2_core::map_sim::MapRam`]); zero after boot. Each new
    /// map is seeded from it once and the live map writes it back every tick.
    map_ram: sml2_core::map_sim::MapRam,
    pub state: SessionState,
    /// Screen whose loader-run music request was last applied, so a screen's
    /// song is requested once per load instead of every frame.
    pub last_map_screen: Option<u8>,
    /// `$3E853`'s hub shimmer, seeded when the hub screen loads.
    pub map_wave: Option<sml2_core::map::MapWave>,
    /// Set while a map level entry runs its 37-frame transition.
    pub pending_level: Option<Box<MapTransition>>,
    /// White-screen hub reload and palette return after releasing camera pan.
    pub map_pan_return: Option<MapPanReturn>,
    pub map_screen_transition: Option<MapScreenTransition>,
    pub assets: SessionAssets,
    pub lives: u8,
    pub powerup: u8,
    pub coins_low: u8,
    pub coins_high: u8,
    pub level_tally: u8,
    pub money_bag: u8,
    /// `sFlutterTimer` `$A24D`, `sFlutterArmed` `$A284`, `sFlutterCharge`
    /// `$A27C`. Level load and the map never write these, so the next level
    /// starts with whatever the previous one left.
    flutter: u8,
    flutter_armed: bool,
    flutter_charge: u8,
    pub prev_keys_held: u8,
    pub prev_a: bool,
    pub prev_b: bool,
    pub prev_start: bool,
    pub prev_select: bool,
    /// `$FF97`: advanced only on frames the ROM's `$0300` frame wait runs
    /// (see [`Session::main_loop_waits`]), every level frame included.
    ff97: u8,
    pub frame_n: u64,
    pub sound_driver: SoundDriver,
    /// The three `$2913` save slots, validated when the file select opens.
    pub slots: SaveSlots,
    /// The slot the current game was loaded from, so saves update it in place.
    pub active_slot: Option<u8>,
    /// Whether Easy mode is active for the current session.
    pub easy_mode: bool,
    /// Deferred goal-bell handoff inputs: `(node, exit_tile_flag)`.
    bell_return: Option<(u8, u8)>,
    /// Hold frames already spent after the bell script reached state `$0A`.
    bell_fade: Option<u16>,
    /// The current bell return's hold ([`bell_return_hold`]).
    bell_return_hold: u16,
    /// Hold frames already spent before the bell script's first step.
    bell_entry: Option<u16>,
    /// Where the castle reveal lands: `(screen, node, exit_tile_flag, move)`.
    castle_return: Option<(u8, u8, u8, Option<u8>)>,
    /// `sLevelProgress` while the reveal owns it.
    castle_progress: [u8; 42],
}

impl Session {
    /// `$A298`/`$A2CD` as the current state runs them, for diagnostics.
    pub fn tile_animation_clock(&self) -> TileAnimationClock {
        match &self.state {
            SessionState::InLevel(ctx) => ctx.tile_animation,
            SessionState::Demo(demo) => demo.ctx.tile_animation,
            _ => self.tile_animation,
        }
    }

    fn remember_clocks(&mut self) {
        self.object_clock.remember(&self.state);
        match &self.state {
            SessionState::InLevel(ctx) => self.tile_animation = ctx.tile_animation,
            SessionState::Demo(demo) => self.tile_animation = demo.ctx.tile_animation,
            _ => {}
        }
    }

    /// The session's `$FF97`, for the map and diagnostics.
    pub fn ff97(&self) -> u8 {
        self.ff97
    }

    /// A level continues the tile-animation bytes. Its movement phase keeps
    /// its own origin: the console's depends on every load's length and every
    /// lag frame since power-on, which a native port does not reproduce.
    fn seed_level_clocks(&self, ctx: &mut LevelContext) {
        ctx.tile_animation = self.tile_animation;
    }

    /// Whether this frame runs the `$0300` frame wait that increments `$FF97`
    /// and `sTileAnimTimer`. Loaders that turn VBlank off, and bank 15's own
    /// `$3E9EC` fade waits, block inside one console frame after another
    /// without it. In-level lag frames are console CPU time, not modelled.
    fn main_loop_waits(&self) -> bool {
        match &self.state {
            SessionState::Overworld(_) => {
                if let Some(t) = &self.pending_level {
                    return t.frame_waits(t.frame + 1);
                }
                // `$3E9FB`'s fade and the screen load block; the hold's last
                // two frames already run the map's frame wait (measured on
                // the hub return: 83 frozen frames of its 84).
                let bell_fade_blocks = self
                    .bell_fade
                    .is_some_and(|held| held + 2 < self.bell_return_hold);
                // A hold's last frame clears it and runs the map handler.
                self.map_screen_transition.is_none()
                    && self.map_pan_return.is_none()
                    && !bell_fade_blocks
                    && !self.bell_entry.is_some_and(|held| held < BELL_ENTRY_HOLD)
            }
            // `$19` turns VBlank off in the frame its handler first runs.
            SessionState::FileSelectLoading(_) => false,
            _ => true,
        }
    }

    /// Start the golden-coin castle reveal and remember where it lands.
    pub fn begin_castle_sequence(
        &mut self,
        seq: sml2_core::castle_sequence::CastleSequence,
        screen: u8,
        node: u8,
        exit_tile_flag: u8,
    ) {
        self.remember_clocks();
        self.castle_progress = match &self.state {
            SessionState::Overworld(map) => map.level_progress,
            SessionState::InLevel(ctx) => ctx.sim.level_progress,
            _ => [0; 42],
        };
        for (index, off) in sml2_core::castle_sequence::GOLDEN_COIN_OFFSETS
            .into_iter()
            .enumerate()
        {
            self.castle_progress[off] = seq.coins[index];
        }
        self.castle_return = Some((screen, node, exit_tile_flag, None));
        self.state = SessionState::Castle(seq);
    }

    /// Put a level live immediately (the map's own 37-frame transition is
    /// skipped), for acceptance tapes.
    pub fn begin_level(&mut self, level_id: u8, progress_flags: u8) -> Result<(), String> {
        self.remember_clocks();
        let content = self.assets.level(level_id)?.clone();
        let ctx = LevelContext::from_content(
            level_id,
            &content,
            progress_flags,
            self.lives,
            self.coins_low,
            self.coins_high,
            self.powerup,
            0,
            self.assets.scripts.clone(),
            self.assets.tilesets.clone(),
            self.assets.arena_layout.clone(),
            self.assets.anim_rows.clone(),
            self.assets.hud.clone(),
            self.assets.object_tables.clone(),
        );
        let mut ctx = Box::new(ctx);
        self.object_clock.initialize_level(&mut ctx.obj);
        self.seed_level_clocks(&mut ctx);
        self.map_ram = sml2_core::map_sim::MapRam::from_level(&ctx.manifest.metatiles);
        ctx.obj.easy_mode = self.easy_mode;
        ctx.obj.money_bag = self.money_bag;
        ctx.credits_text = self.assets.credits_text.clone();
        if let Some(assets) = self.assets.ending_scene.clone() {
            let mut state = 4;
            let mut fade = 0x0100;
            ctx.ending_scene.load(assets, &mut state, &mut fade);
        }
        self.state = SessionState::InLevel(ctx);
        Ok(())
    }

    /// Stage a bell room the way the goal-bell exit does, for acceptance tapes.
    pub fn begin_bell_room(&mut self, screen: u8, node: u8, exit_tile_flag: u8) {
        self.remember_clocks();
        let mut map = MapSim::new();
        map.set_castle_scripts(self.assets.castle_scripts.clone());
        map.lives = self.lives;
        map.powerup = self.powerup;
        map.coins_low = self.coins_low;
        map.coins_high = self.coins_high;
        let scripts = self.assets.castle_scripts.clone();
        map.room = Some(if screen == 13 {
            sml2_core::castle_room::CastleRoom::enter_bell_a(&scripts, 0)
        } else {
            let tilemap = self
                .assets
                .overworld_tilemaps
                .screen_tilemap(14)
                .unwrap_or(&[]);
            sml2_core::castle_room::CastleRoom::enter_bell_b(&scripts, tilemap)
        });
        map.screen_id = screen;
        map.node = node;
        map.frame_counter = 0;
        self.bell_return = Some((node, exit_tile_flag));
        self.state = SessionState::Overworld(map);
    }

    /// Stage a coin-door room the way `$3F3B9` does, for tapes and previews.
    /// Start the blocking bank-15 door walk once a door has run inside
    /// `MapSim::tick`: `source` is the map as the frame began, `destination`
    /// the screen the door loaded.
    pub(crate) fn begin_door_transition(&mut self, source: MapSim, direction: u8, destination: u8) {
        self.map_screen_transition = Some(MapScreenTransition::new(source, direction));
        self.last_map_screen = Some(destination);
        self.map_wave = if destination == 0 {
            sml2_core::map::MapWave::seed(self.assets.overworld_tiles.screen_raw(0))
        } else {
            None
        };
        self.sound_driver.music_request = if destination == 0 { 0 } else { 0xFF };
    }

    pub fn begin_coin_room(&mut self, screen: u8) {
        self.remember_clocks();
        let mut map = MapSim::new();
        map.set_castle_scripts(self.assets.castle_scripts.clone());
        map.lives = self.lives;
        map.powerup = self.powerup;
        map.coins_low = self.coins_low;
        map.coins_high = self.coins_high;
        map.room = Some(sml2_core::castle_room::CastleRoom::enter_coin(
            &self.assets.castle_scripts,
            screen,
        ));
        map.screen_id = screen;
        map.frame_counter = 0;
        self.state = SessionState::Overworld(map);
    }

    pub fn new(assets: SessionAssets) -> Self {
        let mut map = MapSim::new();
        map.set_castle_scripts(assets.castle_scripts.clone());
        map.lives = 5;
        map.load_screen(0, 0, &assets.node_records);
        map.set_map_ram(Default::default());
        let sound_driver = SoundDriver::with_music_bank(assets.music_bank.clone());
        Self {
            state: SessionState::Overworld(map),
            object_clock: ObjectClock::default(),
            tile_animation: TileAnimationClock::default(),
            rom_timing: None,
            map_ram: Default::default(),
            last_map_screen: None,
            map_wave: None,
            pending_level: None,
            map_pan_return: None,
            map_screen_transition: None,
            assets,
            lives: 5,
            powerup: 0,
            coins_low: 0,
            coins_high: 0,
            level_tally: 0,
            money_bag: 0,
            flutter: 0,
            flutter_armed: false,
            flutter_charge: 0,
            prev_keys_held: 0,
            prev_a: false,
            prev_b: false,
            prev_start: false,
            prev_select: false,
            ff97: 0,
            frame_n: 0,
            sound_driver,
            slots: SaveSlots::new(),
            active_slot: None,
            easy_mode: false,
            bell_return: None,
            bell_fade: None,
            bell_return_hold: 0,
            bell_entry: None,
            castle_return: None,
            castle_progress: [0; 42],
        }
    }

    /// `$30000`: enter the file select directly (desktop smoke/evidence).
    pub fn enter_file_select(&mut self) {
        self.remember_clocks();
        let mut file_select = FileSelect::new(&mut self.slots);
        self.sound_driver.music_request = file_select.take_music();
        self.state = SessionState::FileSelect(file_select);
    }

    /// `$00`: enter the title screen with a specific `sDemoIndex`.
    pub fn enter_title_with_demo(&mut self, demo_index: u8) {
        self.enter_title();
        if let SessionState::Title(title) = &mut self.state {
            title.set_demo_index(demo_index & 0x03);
        }
    }

    /// `$00`: enter the title screen instead of the overworld.
    pub fn enter_title(&mut self) {
        self.remember_clocks();
        self.state = SessionState::Title(TitleScreen::new());
        self.last_map_screen = None;
        self.active_slot = None;
        // `$14043` stops all sound while the logo holds.
        self.sound_driver.music_request = 0xFF;
    }

    /// `$140D6`/`$140A8`: load the demo's level and copy its tape page.
    pub fn start_demo(&mut self, index: u8) -> bool {
        self.remember_clocks();
        let index = index & 0x03;
        let level_id = DEMO_LEVELS[index as usize];
        let Ok(content) = self.assets.level(level_id) else {
            return false;
        };
        let mut ctx = LevelContext::from_content(
            level_id,
            content,
            0,
            5,
            0,
            0,
            0,
            0,
            self.assets.scripts.clone(),
            self.assets.tilesets.clone(),
            self.assets.arena_layout.clone(),
            self.assets.anim_rows.clone(),
            self.assets.hud.clone(),
            self.assets.object_tables.clone(),
        );
        self.object_clock.initialize_level(&mut ctx.obj);
        self.seed_level_clocks(&mut ctx);
        self.map_ram = sml2_core::map_sim::MapRam::from_level(&ctx.manifest.metatiles);
        self.state = SessionState::Demo(Box::new(DemoContext {
            ctx: Box::new(ctx),
            tape: DemoTape::new(self.assets.front_end.demo_tape(index)),
            level_id,
            index,
        }));
        // `$140D6`: `sMusicRequest = $FF` before state $0B.
        self.sound_driver.music_request = 0xFF;
        true
    }

    /// `$38CE`: start `(index + 1) & 3` on the next title loop.
    fn finish_demo(&mut self, index: u8) {
        let mut title = TitleScreen::new();
        title.set_demo_index((index + 1) & 0x03);
        self.state = SessionState::Title(title);
        self.last_map_screen = None;
        self.sound_driver.music_request = 0xFF;
    }

    /// Front-end states have no progress to persist yet.
    pub fn saveable(&self) -> bool {
        matches!(
            self.state,
            SessionState::Overworld(_) | SessionState::InLevel(_) | SessionState::GameOver(_)
        )
    }

    /// Apply an already-read ROM-format progress record.
    pub fn apply_save_record(&mut self, data: &[u8]) -> bool {
        self.remember_clocks();
        if !sml2_core::map::save_record_valid(data) {
            return false;
        }
        let mut map = MapSim::new();
        map.set_castle_scripts(self.assets.castle_scripts.clone());
        map.lives = data[0x43];
        if map.apply_save_record(data, &self.assets.node_records) {
            // $353E -> $14: resume through the same node/zone landing as play.
            let mut sim = PlayerSim::new();
            map.to_player(&mut sim);
            sim.dead = 21;
            let handoff = sim.map_handoff(&self.assets.nodes);
            map = MapSim::from_player(&sim);
            map.set_castle_scripts(self.assets.castle_scripts.clone());
            map.load_screen(handoff.screen.unwrap_or(0), 0, &self.assets.node_records);
            map.load_node(map.node, &self.assets.node_records);
            self.lives = data[0x43];
            self.coins_low = data[0x40];
            self.coins_high = data[0x41];
            self.level_tally = data[0x42];
            self.money_bag = data[0x44];
            // The `$50`-byte record has no flutter bytes. A loaded file is a
            // fresh WRAM image for `$A24D`/`$A284`/`$A27C`.
            self.flutter = 0;
            self.flutter_armed = false;
            self.flutter_charge = 0;
            self.state = SessionState::Overworld(map);
            self.map_pan_return = None;
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
            SessionState::Castle(_) => {
                let mut map = MapSim::new();
                map.level_progress = self.castle_progress;
                if let Some((screen, node, _, _)) = self.castle_return {
                    map.load_screen(screen, 0, &self.assets.node_records);
                    map.node = node;
                    map.load_node(node, &self.assets.node_records);
                }
                map.coins_low = self.coins_low;
                map.coins_high = self.coins_high;
                map.save_record(self.lives, self.level_tally, self.money_bag)
            }
            SessionState::GameOver(screen) => screen.sim.save_record(self.money_bag),
            SessionState::Title(_)
            | SessionState::FileSelectLoading(_)
            | SessionState::FileSelect(_)
            | SessionState::Demo(_) => {
                match self
                    .active_slot
                    .and_then(|slot| self.slots.slot(slot as usize))
                {
                    Some(record) => *record,
                    None => sml2_core::front_end::save_slots::fresh_template(),
                }
            }
        }
    }

    /// Advance exactly one tick from explicit inputs, emitting deferred host effects.
    /// Supply the console's `$FF97` and `rDIV` reads for the next `advance`.
    pub fn set_rom_timing(&mut self, timing: RomTiming) {
        self.rom_timing = Some(timing);
    }

    pub fn advance(&mut self, input: FrameInput) -> FrameEffects {
        self.remember_clocks();
        let FrameInput { keys, div } = input;
        let rom_timing = self.rom_timing.take();
        if self.main_loop_waits() {
            self.ff97 = self.ff97.wrapping_add(1);
            // Levels and demos tick their own copy with their config.
            if !matches!(self.state, SessionState::InLevel(_) | SessionState::Demo(_)) {
                self.tile_animation.tick(0, false);
            }
        }
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

        let mut deferred_sfx0: Option<u8> = None;
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
                if let Some(transition) = self.map_screen_transition.as_mut() {
                    self.prev_keys_held = pack_joypad(
                        keys.right,
                        keys.left,
                        keys.up,
                        keys.down,
                        keys.a,
                        keys.b,
                        keys.select,
                        keys.start,
                    );
                    if transition.advance(map_sim, div) {
                        let entering = transition.entering;
                        self.map_screen_transition = None;
                        if !entering {
                            self.map_pan_return = Some(MapPanReturn::default());
                            self.sound_driver.music_request = 0xFF;
                        }
                    } else if transition.entering && transition.frame == 48 {
                        self.sound_driver.music_request =
                            sml2_core::map_sim::screen_music(map_sim.screen_id);
                    }
                    break 'map StepOutcome::Continue;
                }
                if let Some(held) = self.bell_entry {
                    if held < BELL_ENTRY_HOLD {
                        self.prev_keys_held = pack_joypad(
                            keys.right,
                            keys.left,
                            keys.up,
                            keys.down,
                            keys.a,
                            keys.b,
                            keys.select,
                            keys.start,
                        );
                        self.bell_entry = Some(held + 1);
                        break 'map StepOutcome::Continue;
                    }
                    self.bell_entry = None;
                }
                if let Some(held) = self.bell_fade {
                    if held < self.bell_return_hold {
                        self.prev_keys_held = pack_joypad(
                            keys.right,
                            keys.left,
                            keys.up,
                            keys.down,
                            keys.a,
                            keys.b,
                            keys.select,
                            keys.start,
                        );
                        self.bell_fade = Some(held + 1);
                        break 'map StepOutcome::Continue;
                    }
                    self.bell_fade = None;
                    // The room clears sGoalBell; its return runs $3EF2B
                    // again, including the progress write and zone selection.
                    if let Some((node, flag)) = self.bell_return.take() {
                        let (sim, handoff) = bell_handoff(map_sim, node, flag, &self.assets.nodes);
                        map_sim.room = None;
                        map_sim.level_progress = sim.level_progress;
                        map_sim.mario_dead = sim.dead;
                        map_sim.node = handoff.node;
                        map_sim.load_screen(
                            handoff.screen.unwrap_or(0),
                            flag,
                            &self.assets.node_records,
                        );
                        map_sim.load_node(handoff.node, &self.assets.node_records);
                        if let Some(direction) = handoff.cursor_move {
                            map_sim.cursor_move = direction;
                        }
                        map_sim.pending_save = handoff.saves > 0;
                    }
                }
                if let Some(pan) = self.map_pan_return.as_mut() {
                    // Blocking loader/fade waits do not run map gameplay. Keep
                    // the input edge baseline current so held keys do not fire
                    // as new presses when the transition returns.
                    self.prev_keys_held = pack_joypad(
                        keys.right,
                        keys.left,
                        keys.up,
                        keys.down,
                        keys.a,
                        keys.b,
                        keys.select,
                        keys.start,
                    );
                    let finished = pan.advance();
                    if pan.frame == MAP_PAN_RELOAD_FRAMES {
                        self.sound_driver.music_request = 6;
                    }
                    if finished {
                        self.map_pan_return = None;
                    }
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

                let ff97 = rom_timing.as_ref().map_or(self.ff97, |t| t.ff97);
                map_sim.set_zone_animation_clock(ff97, 0);
                if !map_sim.ram_seeded {
                    map_sim.set_map_ram(self.map_ram);
                }
                if let Some(timing) = &rom_timing {
                    map_sim.div_reads = timing.div_reads.iter().copied().collect();
                }

                // A door replaces the screen inside `tick`, and its walk still
                // shows the screen it left. Snapshot without the room scripts:
                // they are the only heap payload and a door walk never reads them.
                let scripts = map_sim.castle_scripts.take();
                let source_map = map_sim.clone();
                map_sim.castle_scripts = scripts;
                let entry = map_sim.tick(
                    input,
                    &self.assets.nodes,
                    &self.assets.walk,
                    &self.assets.node_records,
                );
                self.map_ram = map_sim.map_ram();

                if let Some(direction) = entry.transition {
                    let destination = map_sim.screen_id;
                    self.begin_door_transition(source_map, direction, destination);
                    break 'map StepOutcome::Continue;
                }

                // State `$0A` is written by the script after this frame's
                // screen check, so the `$3E9FB` fade starts on the next frame.
                // Count the hold from the frame after this one.
                if self.bell_fade.is_none()
                    && map_sim.screen_id >= 13
                    && map_sim.room.as_ref().is_some_and(|r| r.state >= 10)
                {
                    self.bell_fade = Some(0);
                    let screen = self.bell_return.map_or(0, |(node, flag)| {
                        bell_handoff(map_sim, node, flag, &self.assets.nodes).1.screen.unwrap_or(0)
                    });
                    self.bell_return_hold = bell_return_hold(screen);
                }
                // A bonus room writes `sLives`/`sCurPowerup`/`sCoin*`; mirror
                // them back while the room owns the screen.
                if map_sim.room.is_some() {
                    self.lives = map_sim.lives;
                    self.powerup = map_sim.powerup;
                    self.coins_low = map_sim.coins_low;
                    self.coins_high = map_sim.coins_high;
                }

                if entry.hub_reload {
                    // `$3F156` first stops sound and blanks the LCD; its final
                    // song request is deferred until the loader interval ends.
                    self.sound_driver.music_request = 0xFF;
                    self.map_pan_return = Some(MapPanReturn::default());
                    self.map_wave =
                        sml2_core::map::MapWave::seed(self.assets.overworld_tiles.screen_raw(0));
                    break 'map StepOutcome::Continue;
                }

                if let Some(music) = entry.music {
                    self.sound_driver.music_request = music;
                }
                if let Some(sfx) = entry.sfx {
                    if self.sound_driver.music_request != 0 {
                        // `$3C1EC` posts `sSfxRequest0` *after* `$3C1A2`'s
                        // `$2AAA` already ran `_UpdateSound`; posting it before
                        // this frame's update would let the stop-all clear it.
                        deferred_sfx0 = Some(sfx);
                    } else {
                        self.sound_driver.sfx_request_0 = sfx;
                    }
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
                    if let Some(slot) = self.active_slot {
                        self.slots.write_slot(slot as usize, &rec);
                    }
                    saves.push(rec);
                }

                if entry.level == Some(0xFF) {
                    // $3573: this is a castle visit, not a level manifest.
                    let mut seq = sml2_core::castle_sequence::CastleSequence::default();
                    seq.enter(
                        &sml2_core::castle_sequence::GOLDEN_COIN_OFFSETS
                            .map(|off| map_sim.level_progress[off]),
                    );
                    seq.level_tally = self.level_tally;
                    self.castle_progress = map_sim.level_progress;
                    self.castle_return = Some((map_sim.screen_id, map_sim.node, 0, None));
                    self.state = SessionState::Castle(seq);
                    break 'map StepOutcome::Continue;
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
                        Ok(mut ctx) => {
                            self.object_clock.initialize_level(&mut ctx.obj);
                            self.map_ram = sml2_core::map_sim::MapRam::from_level(&ctx.manifest.metatiles);
                            ctx.obj.easy_mode = self.easy_mode;
                            // The level loader shares the map's persistent block;
                            // entering one level must retain every other clear.
                            ctx.sim.level_progress = map_sim.level_progress;
                            ctx.sim.level_tally = self.level_tally;
                            ctx.sim.flutter = self.flutter;
                            ctx.sim.flutter_armed = self.flutter_armed;
                            ctx.sim.flutter_charge = self.flutter_charge;
                            // $3C1A2 stops the map theme before loading. `$3E9FB`
                            // fades the map out and the level goes live on the
                            // first state-`$04` frame, `MAP_ENTRY_FRAMES` later.
                            ctx.credits_text = self.assets.credits_text.clone();
                            if let Some(assets) = self.assets.ending_scene.clone() {
                                let mut state = 4;
                                let mut fade = 0x0100;
                                ctx.ending_scene.load(assets, &mut state, &mut fade);
                            }
                            self.sound_driver.music_request = 0xFF;
                            self.pending_level = Some(Box::new(MapTransition::from_map(
                                Box::new(ctx),
                                level_id,
                                self.easy_mode,
                            )));
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

            SessionState::Title(ref mut title) => {
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
                match title.step(keys_held, keys_pressed) {
                    TitleAction::OpenFileSelect => {
                        // `$140B1` changes dispatch to `$19`; it does not run
                        // the loader or the cursor's `$1A` update itself.
                        self.state = SessionState::FileSelectLoading(FileSelectLoading::new());
                    }
                    TitleAction::StartDemo(index) => {
                        if !self.start_demo(index) {
                            diagnostics
                                .push(format!("front end: attract demo {index} could not load"));
                        }
                    }
                    TitleAction::None => {}
                }
                StepOutcome::Continue
            }

            SessionState::FileSelectLoading(_) => {
                self.advance_file_select_loading(keys);
                StepOutcome::Continue
            }

            SessionState::FileSelect(ref mut file_select) => {
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
                let out = file_select.step(&mut self.slots, keys_held, keys_pressed);
                if out.sfx0 != 0 {
                    self.sound_driver.sfx_request_0 = out.sfx0;
                }
                if out.sfx2 != 0 {
                    self.sound_driver.sfx_request_2 = out.sfx2;
                }
                match out.action {
                    FileSelectAction::Load { slot, record } => {
                        self.active_slot = Some(slot as u8);
                        self.easy_mode = file_select.easy_mode();
                        // `$30227` copies the slot verbatim. An untouched slot
                        // holds the invalid new-game template, which becomes a
                        // valid fresh record for the map.
                        let record = if sml2_core::map::save_record_valid(&record) {
                            record
                        } else {
                            MapSim::new().save_record(5, 0, 0)
                        };
                        if self.apply_save_record(&record) {
                            self.last_map_screen = None;
                        } else {
                            diagnostics
                                .push("front end: the loaded record was rejected".to_string());
                        }
                    }
                    FileSelectAction::Erased { .. } | FileSelectAction::None => {}
                }
                StepOutcome::Continue
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
                self.money_bag = ctx.obj.money_bag;

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
                // `$273E`/`$2756` write `sSoundMode`; `_UpdateSound` consumes it
                // this same frame and `$10067` clears the byte again.
                if ctx.sim.sound_mode != 0 {
                    self.sound_driver.sound_mode = ctx.sim.sound_mode;
                    ctx.sim.sound_mode = 0;
                }
                self.sound_driver.stomp_chain = ctx.obj.stomp_chain;
                self.sound_driver.low_time_flag = ctx.sim.low_time_flag != 0;

                self.object_clock.observe(&ctx.obj);
                self.tile_animation = ctx.tile_animation;
                if ctx.is_finished() && ctx.sim.game_over {
                    // `$311A`'s lives borrow: state `$05` (`$27AF`), not the map.
                    let mut sim = ctx.sim.clone();
                    let obp = [
                        ctx.manifest.header.palettes[1],
                        ctx.manifest.header.palettes[2],
                    ];
                    self.flutter = sim.flutter;
                    self.flutter_armed = sim.flutter_armed;
                    self.flutter_charge = sim.flutter_charge;
                    sim.game_over_enter();
                    self.lives = sim.lives;
                    self.coins_low = sim.coins_low;
                    self.coins_high = sim.coins_high;
                    self.level_tally = sim.level_tally;
                    self.money_bag = 0;
                    self.powerup = 0;
                    self.sound_driver.music_request = sim.music_request;
                    self.state = SessionState::GameOver(Box::new(GameOverScreen { sim, obp }));
                    // `$2934`: the save the level-clear hand-off also makes.
                    let rec = self.save_record();
                    if let Some(slot) = self.active_slot {
                        self.slots.write_slot(slot as usize, &rec);
                    }
                    saves.push(rec);
                    StepOutcome::GameOver
                } else if ctx.is_finished() {
                    self.flutter = ctx.sim.flutter;
                    self.flutter_armed = ctx.sim.flutter_armed;
                    self.flutter_charge = ctx.sim.flutter_charge;
                    let handoff = ctx
                        .map_handoff
                        .unwrap_or_else(|| ctx.sim.map_handoff(&self.assets.nodes));

                    // Zone loaders can set the shared dead byte after a clear
                    // ($3D573, for example). The completed exit identifies the
                    // bell path independently of that map-arrival latch.
                    if ctx.sim.dead != 0 && !ctx.sim.exit_completed {
                        // Player died in level; 0 lives still continues.
                        {
                            self.lives = ctx.sim.lives;
                            self.powerup = 0; // Death resets powerup
                            let target_node = handoff.node;
                            let target_screen = handoff.screen.unwrap_or(0);
                            let mut map_sim = MapSim::from_player(&ctx.sim);
                            map_sim.set_castle_scripts(self.assets.castle_scripts.clone());
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
                                let rec = self.save_record();
                                if let Some(slot) = self.active_slot {
                                    self.slots.write_slot(slot as usize, &rec);
                                }
                                saves.push(rec);
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
                        if ctx.sim.boss_clear {
                            // Bank 2's boss-clear arm sets `sGameState = $1D`.
                            let mut seq = sml2_core::castle_sequence::CastleSequence::default();
                            let coins = [
                                ctx.sim.level_progress[5],
                                ctx.sim.level_progress[9],
                                ctx.sim.level_progress[14],
                                ctx.sim.level_progress[17],
                                ctx.sim.level_progress[23],
                                ctx.sim.level_progress[29],
                            ];
                            seq.enter(&coins);
                            seq.level = ctx.manifest.level_id;
                            seq.progress_flags = ctx.sim.level_progress_flags;
                            // PlayerSim's reward adapter already credited this tally.
                            seq.level_tally = ctx.sim.level_tally;
                            seq.tally_credited = true;
                            let offset = sml2_core::castle_sequence::LEVEL_PROGRESS_OFFSETS
                                [seq.level as usize]
                                as usize;
                            if let Some(index) = sml2_core::castle_sequence::GOLDEN_COIN_OFFSETS
                                .iter()
                                .position(|&off| off == offset)
                            {
                                seq.coins[index] = ctx.sim.level_progress_flags;
                                seq.golden_count =
                                    sml2_core::castle_sequence::CastleSequence::count_coins(
                                        &seq.coins,
                                    );
                            }
                            seq.state = sml2_core::castle_sequence::STATE_REVEAL_D;
                            self.castle_progress = ctx.sim.level_progress;
                            self.castle_progress[offset] = ctx.sim.level_progress_flags;
                            self.castle_return = Some((
                                target_screen,
                                target_node,
                                ctx.sim.exit_tile_flag,
                                handoff.cursor_move,
                            ));
                            self.state = SessionState::Castle(seq);
                        } else {
                            let mut map_sim = MapSim::from_player(&ctx.sim);
                            map_sim.set_castle_scripts(self.assets.castle_scripts.clone());
                            if handoff.cutscene {
                                // `$3EF2B`: `sGoalBell` sends the exit to the
                                // bonus room, picked by `sMapNode` bit 0.
                                let screen = if target_node & 1 == 0 { 13 } else { 14 };
                                let scripts = self.assets.castle_scripts.clone();
                                let room = if screen == 13 {
                                    sml2_core::castle_room::CastleRoom::enter_bell_a(
                                        &scripts,
                                        ctx.sim.level_timer.high,
                                    )
                                } else {
                                    let tilemap = self
                                        .assets
                                        .overworld_tilemaps
                                        .screen_tilemap(14)
                                        .unwrap_or(&[]);
                                    sml2_core::castle_room::CastleRoom::enter_bell_b(
                                        &scripts, tilemap,
                                    )
                                };
                                map_sim.screen_id = screen;
                                map_sim.frame_counter = 0;
                                map_sim.node = target_node;
                                map_sim.room = Some(room);
                                self.bell_entry = Some(0);
                                self.bell_return =
                                    Some((target_node, ctx.sim.exit_tile_flag));
                            } else {
                                map_sim.load_screen(
                                    target_screen,
                                    ctx.sim.exit_tile_flag,
                                    &self.assets.node_records,
                                );
                                map_sim.node = target_node;
                                map_sim.load_node(target_node, &self.assets.node_records);
                                if let Some(direction) = handoff.cursor_move {
                                    map_sim.cursor_move = direction;
                                }
                            }
                            self.state = SessionState::Overworld(map_sim);
                        }
                        if handoff.saves > 0 {
                            let rec = self.save_record();
                            if let Some(slot) = self.active_slot {
                                self.slots.write_slot(slot as usize, &rec);
                            }
                            saves.push(rec);
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

            SessionState::Demo(ref mut demo) => {
                let mut finished = start_pressed;
                if !finished {
                    match demo.tape.step() {
                        Some((held, pressed)) => {
                            let keys = PlayerKeys {
                                right: held & 0x10 != 0,
                                left: held & 0x20 != 0,
                                up: held & 0x40 != 0,
                                down: held & 0x80 != 0,
                                a: held & 0x01 != 0,
                                b: held & 0x02 != 0,
                                select: held & 0x04 != 0,
                                start: held & 0x08 != 0,
                            };
                            demo.ctx.step_frame(
                                keys,
                                pressed & 0x01 != 0,
                                pressed & 0x02 != 0,
                                false,
                                false,
                                div,
                                &self.assets.nodes,
                            );
                            self.object_clock.observe(&demo.ctx.obj);
                            self.tile_animation = demo.ctx.tile_animation;
                            if demo.ctx.sim.sfx0 != 0 {
                                self.sound_driver.sfx_request_0 = demo.ctx.sim.sfx0;
                                demo.ctx.sim.sfx0 = 0;
                            }
                            if demo.ctx.sim.sfx1 != 0 {
                                self.sound_driver.sfx_request_1 = demo.ctx.sim.sfx1;
                                demo.ctx.sim.sfx1 = 0;
                            }
                            if demo.ctx.sim.sfx2 != 0 {
                                self.sound_driver.sfx_request_2 = demo.ctx.sim.sfx2;
                                demo.ctx.sim.sfx2 = 0;
                            }
                            // The attract demo has no level music.
                            demo.ctx.sim.music_request = 0;
                        }
                        None => finished = true,
                    }
                }
                let index = demo.index;
                if finished {
                    self.finish_demo(index);
                }
                StepOutcome::Continue
            }

            // The castle door's Easy card holds the reveal still.
            SessionState::Castle(_) if self.pending_level.is_some() => {
                if let Some(t) = self.pending_level.as_mut() {
                    t.frame += 1;
                }
                StepOutcome::Continue
            }
            SessionState::Castle(_) => {
                let held = pack_joypad(
                    keys.right,
                    keys.left,
                    keys.up,
                    keys.down,
                    keys.a,
                    keys.b,
                    keys.select,
                    keys.start,
                );
                let pressed = held & !self.prev_keys_held;
                self.prev_keys_held = held;
                self.advance_castle(pressed, &mut saves, &mut diagnostics)
            }

            SessionState::GameOver(screen) => {
                // `$0300`: `$FF97` ticks every frame, VBlank runs the fade.
                screen.sim.frame_counter = screen.sim.frame_counter.wrapping_add(1);
                let held = pack_joypad(
                    keys.right,
                    keys.left,
                    keys.up,
                    keys.down,
                    keys.a,
                    keys.b,
                    keys.select,
                    keys.start,
                );
                let pressed = held & !self.prev_keys_held;
                self.prev_keys_held = held;
                if screen.sim.game_over_step(pressed) {
                    // `$287E` → state `$00`, which reboots to the title.
                    self.enter_title();
                }
                StepOutcome::Continue
            }
        };

        // The frozen map arm advances the counter; once it reaches the console's
        // entry length the level goes live (its own music request starts on the
        // first InLevel frame).
        if let Some(t) = self.pending_level.as_mut() {
            if t.frame >= t.live_at {
                let mut t = self.pending_level.take().expect("checked above");
                self.seed_level_clocks(&mut t.ctx);
                // `sMoneyBagCount` is session state; no level load clears it.
                t.ctx.obj.money_bag = self.money_bag;
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
        if let Some(sfx) = deferred_sfx0 {
            // Applied after the frame's update so the next `_UpdateSound` sees
            // it, exactly like the ROM's `$3C1EC` tail.
            self.sound_driver.sfx_request_0 = sfx;
        }
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
