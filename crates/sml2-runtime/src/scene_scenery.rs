//! Authored surrounds for fixed scenes. Coordinates use 16-pixel cells with
//! the original 160×144 picture at (0, 0); its interior is always protected.

use sml2_core::scenery::SceneryOverlay;
use std::collections::BTreeMap;

pub const SCENE_SWATCHES: usize = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SceneTarget {
    Map(u8),
    Title,
    FileSelect,
    GameOver,
    Castle,
    Arena { level: u8, gfx: u8 },
    Ending,
}

impl SceneTarget {
    pub fn file_stem(self) -> String {
        match self {
            Self::Map(id) => format!("map-{id:02}"),
            Self::Title => "title".into(),
            Self::FileSelect => "file-select".into(),
            Self::GameOver => "game-over".into(),
            Self::Castle => "castle".into(),
            Self::Arena { level, gfx } => format!("arena-{level:02}-{gfx:02}"),
            Self::Ending => "ending".into(),
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Map(0) => "Overworld hub".into(),
            Self::Map(1) => "Pumpkin Zone map".into(),
            Self::Map(2) => "Tree Zone map".into(),
            Self::Map(3) => "Macro Zone map".into(),
            Self::Map(4) => "Space Zone map".into(),
            Self::Map(5) => "Turtle Zone map".into(),
            Self::Map(6) => "Mario Zone map".into(),
            Self::Map(7) => "Wario Castle map".into(),
            Self::Map(8) => "Castle / minigame doors".into(),
            Self::Map(id @ 9..=12) => format!("Coin minigame · room {id}"),
            Self::Map(id) => format!("Bell minigame · room {id}"),
            Self::Title => "Title screen".into(),
            Self::FileSelect => "File select".into(),
            Self::GameOver => "Game over".into(),
            Self::Castle => "Castle reveal".into(),
            Self::Arena { level, .. } => format!("Boss arena · level {level}"),
            Self::Ending => "Ending".into(),
        }
    }

    fn validate(self) -> Result<(), String> {
        match self {
            Self::Map(id) if id > 14 => Err(format!("invalid scenery map {id}")),
            Self::Arena { level, gfx } if level >= 32 || gfx >= 7 => {
                Err(format!("invalid scenery arena {level}/{gfx}"))
            }
            _ => Ok(()),
        }
    }
}

/// Approved fixed-scene exports. Register only files reviewed for release:
/// `(SceneTarget::Map(9), include_str!("../../../assets/scenery/scenery-map-09.json"))`.
/// Local authoring drafts are deliberately absent from this registry.
const BUNDLED: &[(SceneTarget, &str)] = &[
    // Tree Zone boss: continue the clouds around the original nest.
    (
        SceneTarget::Arena { level: 5, gfx: 0 },
        include_str!("../../../assets/scenery/scenery-arena-05-00.json"),
    ),
    // Tree Zone map: a leaf scatter made only of the picture's own cells.
    (
        SceneTarget::Map(2),
        include_str!("../../../assets/scenery/scenery-map-02.json"),
    ),
];

#[derive(Debug, Clone, Default)]
pub struct SceneScenery {
    paints: BTreeMap<SceneTarget, SceneryOverlay>,
    empty: SceneryOverlay,
}

impl SceneScenery {
    pub fn bundled() -> Result<Self, String> {
        Self::from_exports(BUNDLED)
    }

    pub fn from_exports(exports: &[(SceneTarget, &str)]) -> Result<Self, String> {
        let mut scenery = Self::default();
        for &(target, json) in exports {
            if scenery.paints.contains_key(&target) {
                return Err(format!("duplicate scenery scene {}", target.file_stem()));
            }
            let overlay = serde_json::from_str(json)
                .map_err(|error| format!("scene {}: {error}", target.file_stem()))?;
            scenery.set(target, overlay)?;
        }
        Ok(scenery)
    }

    pub fn get(&self, target: SceneTarget) -> &SceneryOverlay {
        self.paints.get(&target).unwrap_or(&self.empty)
    }

    pub fn set(&mut self, target: SceneTarget, overlay: SceneryOverlay) -> Result<(), String> {
        target.validate()?;
        let overlay = SceneryOverlay::try_from_paints(overlay.paints(), SCENE_SWATCHES)
            .map_err(|error| format!("scene {}: {error}", target.file_stem()))?;
        self.paints.insert(target, overlay);
        Ok(())
    }
}

/// Derive arena reloads from whole scene instructions, never from operands
/// whose byte may coincidentally be in the $6x range.
pub(crate) fn arena_gfx(script: &[u8]) -> Option<u8> {
    let mut cursor = 0;
    while let Some(&opcode) = script.get(cursor) {
        let operands = match opcode >> 4 {
            0 => 7,
            1 | 4 => 1,
            2 | 5 | 8 | 10 => 0,
            3 => 2,
            6 => return Some(opcode & 15),
            11 => 3,
            _ => return None,
        };
        cursor += operands + 1;
    }
    None
}

impl crate::Session {
    /// Scene targets available from the currently loaded original content.
    pub fn scenery_scenes(&self) -> Vec<SceneTarget> {
        let mut targets = vec![
            SceneTarget::Title,
            SceneTarget::FileSelect,
            SceneTarget::GameOver,
            SceneTarget::Castle,
        ];
        targets.extend((0..=14).map(SceneTarget::Map));
        let scripts = self.assets.scripts.scripts();
        for level in 0..32 {
            let Ok(content) = self.assets.level(level) else {
                continue;
            };
            let Some(map) = &content.scene_map else {
                continue;
            };
            for &(id, ref bytes) in &scripts {
                if map.contains(&id) {
                    if let Some(gfx) = arena_gfx(bytes) {
                        targets.push(SceneTarget::Arena { level, gfx });
                    }
                }
            }
        }
        if self.assets.ending_scene.is_some() {
            targets.push(SceneTarget::Ending);
        }
        targets.sort();
        targets.dedup();
        targets
    }

    /// Stage an authoring preview through the same scene loaders used by play.
    /// Hosts must use an isolated session rather than a player's saved game.
    pub fn stage_scenery_scene(&mut self, target: SceneTarget) -> Result<(), String> {
        use crate::session::{GameOverScreen, SessionState};
        use sml2_core::{castle_sequence::CastleSequence, player_sim::PlayerSim};
        target.validate()?;
        match target {
            SceneTarget::Title => self.enter_title(),
            SceneTarget::FileSelect => {
                self.enter_file_select();
                if let SessionState::FileSelect(select) = &mut self.state {
                    for _ in 0..45 {
                        select.step(&mut self.slots, 0, 0);
                    }
                }
            }
            SceneTarget::GameOver => {
                self.state = SessionState::GameOver(Box::new(GameOverScreen {
                    sim: PlayerSim::new(),
                    obp: [0xd0, 0x38],
                }))
            }
            SceneTarget::Castle => self.begin_castle_sequence(CastleSequence::default(), 8, 0, 0),
            SceneTarget::Map(id @ 0..=8) => self.debug_warp_map(id, 0)?,
            SceneTarget::Map(id @ 9..=12) => self.begin_coin_room(id),
            SceneTarget::Map(id) => self.begin_bell_room(id, 0, 0),
            SceneTarget::Arena { level, gfx } => self.stage_scenery_arena(level, gfx)?,
            SceneTarget::Ending => {
                let assets = self
                    .assets
                    .ending_scene
                    .clone()
                    .ok_or("ending assets are missing")?;
                self.begin_level(24, 0)?;
                let SessionState::InLevel(ctx) = &mut self.state else {
                    unreachable!()
                };
                let mut phase = 3;
                let mut fade = 0;
                for _ in 0..3 {
                    ctx.ending_entry.frame_head(
                        &mut phase,
                        0xa2,
                        0xe0,
                        &mut fade,
                        Default::default(),
                    );
                    fade = 0;
                }
                ctx.ending_scene.load(
                    assets,
                    &mut ctx.ending_sequence.substate,
                    &mut ctx.ending_sequence.fade_timer,
                );
                ctx.ending_sequence.substate = 5;
            }
        }
        self.active_slot = None;
        Ok(())
    }

    fn stage_scenery_arena(&mut self, level: u8, gfx: u8) -> Result<(), String> {
        use crate::session::SessionState;
        use sml2_core::scene::{boss_def_index, boss_def_portable, SceneStep};
        let content = self.assets.level(level)?;
        let scripts = self.assets.scripts.scripts();
        let map = content
            .scene_map
            .as_ref()
            .ok_or("arena scene map is missing")?;
        let (scene, cell) = scripts
            .iter()
            .find_map(|(id, bytes)| {
                (arena_gfx(bytes) == Some(gfx))
                    .then(|| map.iter().position(|p| p == id).map(|cell| (*id, cell)))
                    .flatten()
            })
            .ok_or_else(|| format!("level {level} has no arena graphics {gfx}"))?;
        // The door scene runs inside the room slide that crossing a locked
        // edge of its screen began, and the slide then carries the camera one
        // screen on into the arena. Every door screen is open only on the
        // side Mario arrives from (`$0D`: the left), so he leaves the other way.
        let locks = content
            .edge_map
            .as_ref()
            .and_then(|e| e.get(cell))
            .copied()
            .unwrap_or(0);
        let onward: (i16, i16) = match locks {
            l if l & 2 == 0 => (1, 0),
            l if l & 1 == 0 => (-1, 0),
            l if l & 4 == 0 => (0, 1),
            l if l & 8 == 0 => (0, -1),
            _ => (1, 0),
        };
        self.begin_level(level, 0)?;
        let SessionState::InLevel(ctx) = &mut self.state else {
            unreachable!()
        };
        ctx.cam.x = (cell % 16) as u16 * 256 + 80;
        ctx.cam.y = (cell / 16) as u16 * 256 + 72;
        ctx.sim.x = ctx.cam.x;
        ctx.sim.y = ctx.cam.y;
        ctx.cam.scene_id = scene;
        ctx.scene_vm.start();
        let mut ended = false;
        for _ in 0..1024 {
            let step = ctx.scene_vm.step(&mut ctx.sim, &mut ctx.cam, &ctx.scripts);
            match step {
                SceneStep::Ran(op) if op >> 4 == 6 => {
                    let sprite = ctx.spr.as_mut().ok_or("arena sprite assets missing")?;
                    if !ctx.tilesets.apply(
                        gfx,
                        &mut ctx.bg_live,
                        &mut sprite.objblock,
                        &mut ctx.manifest.metatiles,
                    ) {
                        return Err(format!("arena graphics {gfx} cannot load"));
                    }
                    ctx.arena_loaded = true;
                    ctx.load_arena()?;
                }
                SceneStep::End => {
                    ended = true;
                    break;
                }
                SceneStep::BadCursor | SceneStep::Stalled(_) | SceneStep::Unported(_) => {
                    return Err(format!("arena scene {scene} cannot be staged: {step:?}"));
                }
                _ => {}
            }
        }
        if !ended {
            return Err(format!("arena scene {scene} did not finish"));
        }
        let world = ctx.cam.world;
        let on = |v: u16, screens: i16| world.wrap(v.wrapping_add_signed(screens * 0x100));
        (ctx.cam.x, ctx.cam.y) = (on(ctx.cam.x, onward.0), on(ctx.cam.y, onward.1));
        (ctx.sim.x, ctx.sim.y) = (on(ctx.sim.x, onward.0), on(ctx.sim.y, onward.1));
        // The script arms $0EB0; run the same boss-load primitives as its
        // runtime consumer, without advancing an authoring preview frame.
        let definition = boss_def_index(level, ctx.cam.boss_fight_phase);
        if !boss_def_portable(definition) {
            return Err(format!("arena boss {definition} is unavailable"));
        }
        ctx.cam.scene_load_pending = true;
        if let Some((music, song)) = ctx.cam.begin_scene_load(level) {
            ctx.sim.music_request = music;
            ctx.sim.level_song = song;
            ctx.obj.spawn_boss(definition);
            ctx.boss_definition = Some(definition);
        }
        ctx.sim.fade_timer = 0;
        ctx.scene_reload_blank = false;
        Ok(())
    }
}
