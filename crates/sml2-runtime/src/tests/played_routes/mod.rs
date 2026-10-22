use super::*;

mod boss_routes;

mod continuous;
mod level19_alternate;

mod level19_header;

mod level19_header_alternate;

mod levels_1_to_5;

mod search;

/// Held-button spans written `KEYS*frames`, space separated: `R` `L` `U` `D`
/// `A` `B` `S` (start) `E` (select), or `-` for nothing held.
pub(super) fn spans(text: &str) -> Vec<PlayerKeys> {
    text.split_whitespace()
        .flat_map(|span| {
            let (held, frames) = span.split_once('*').expect("KEYS*frames");
            let mut keys = PlayerKeys::default();
            for button in held.chars() {
                match button {
                    'R' => keys.right = true,
                    'L' => keys.left = true,
                    'U' => keys.up = true,
                    'D' => keys.down = true,
                    'A' => keys.a = true,
                    'B' => keys.b = true,
                    'S' => keys.start = true,
                    'E' => keys.select = true,
                    '-' => {}
                    other => panic!("unknown button {other:?} in {span:?}"),
                }
            }
            vec![keys; frames.parse().expect("frame count")]
        })
        .collect()
}

/// The inverse of [`spans`].
pub(super) fn to_spans(tape: &[PlayerKeys]) -> String {
    let name = |k: &PlayerKeys| {
        let held: String = [
            (k.right, 'R'),
            (k.left, 'L'),
            (k.up, 'U'),
            (k.down, 'D'),
            (k.a, 'A'),
            (k.b, 'B'),
            (k.start, 'S'),
            (k.select, 'E'),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, c)| *c)
        .collect();
        if held.is_empty() {
            "-".to_string()
        } else {
            held
        }
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < tape.len() {
        let mut j = i;
        while j < tape.len() && name(&tape[j]) == name(&tape[i]) {
            j += 1;
        }
        out.push(format!("{}*{}", name(&tape[i]), j - i));
        i = j;
    }
    out.join(" ")
}

/// Extracted level28 (decimal CLI id 27), ordinary header entry. The fixed
/// right-only tape descends from the upper starting ledge and walks the lower
/// corridor to the source $4A cell (78,30). Objects, scenes, camera, fade and
/// handoff all run through the same dispatch as interactive play.
#[test]
fn level28_header_spawn_to_bell_through_live_play() {
    for tape in [
        PlayTape::Ascii("R".repeat(1400)),
        PlayTape::Buttons(vec![
            PlayerKeys {
                right: true,
                ..Default::default()
            };
            1400
        ]),
    ] {
        let outcome = run_header_route(28, tape, |manifest, layout, sim, cam, nodes| {
            assert_eq!(manifest.level_id, 27);
            assert_eq!(layout.cells[30 * 256 + 78], 0x4A);
            assert_eq!((sim.x, sim.y, cam.x, cam.y), (51, 260, 80, 246));
            assert_eq!(
                (sim.map_node, nodes.progress_slot(sim.map_node)),
                (120, Some(38))
            );
            assert_eq!(manifest.spawns.records.len(), 14);
        });
        let arm = outcome
            .bell_arm
            .expect("right-only approach must arm the bell");
        assert_eq!(arm.frame, 1077, "deterministic native approach timing");
        assert_eq!(arm.position, (1242, 468));
        assert_eq!(
            (arm.fade_timer, arm.music_request, arm.exit_tile_flag),
            (192, 3, 0)
        );
        assert_eq!(outcome.exit_completed_frame, Some(arm.frame + 192));
        assert_eq!(outcome.map_handoff_frame, Some(arm.frame + 193));
        assert_eq!(outcome.frames, 1400);
        assert_eq!((outcome.sim.x, outcome.sim.y), arm.position);
        assert_eq!((outcome.cam.x, outcome.cam.y), (1275, 448));
        assert_eq!((outcome.sim.fade_timer, outcome.sim.level_tally), (0, 1));
        assert!(
            outcome.sim.exit_phase && outcome.sim.exit_completed && outcome.sim.map_handoff_done
        );
        assert_eq!(outcome.sim.dead, 0);
        assert_eq!(outcome.sim.map_node, 46, "$3F104's node $78 rewrite");
        assert_eq!(outcome.sim.level_progress[38], 0x80, "cleared gate byte");
        assert_eq!(outcome.sim.save_record(0)[8 + 38], 0x80);
        let handoff = outcome.map_handoff.expect("the live handoff must run");
        assert_eq!(
            (handoff.node, handoff.screen, handoff.music, handoff.saves),
            (46, Some(1), Some(30), 2)
        );
        assert!(!handoff.cutscene);
        assert!(outcome.obj.unported.is_empty());
        assert!(
            outcome.active_object_frames > 0,
            "ordinary objects must run during the approach"
        );
    }
}

fn run_header_route(
    level: u8,
    tape: impl Into<PlayTape>,
    check_entry: impl FnOnce(
        &LevelManifest,
        &LevelLayout,
        &PlayerSim,
        &sml2_core::camera::CameraSim,
        &MapNodes,
    ),
) -> PlayOutcome {
    run_entry_route(level, 0, tape, check_entry)
}

fn run_entry_route(
    level: u8,
    progress: u8,
    tape: impl Into<PlayTape>,
    check_entry: impl FnOnce(
        &LevelManifest,
        &LevelLayout,
        &PlayerSim,
        &sml2_core::camera::CameraSim,
        &MapNodes,
    ),
) -> PlayOutcome {
    let assets = RouteAssets::load(level);
    let tape = tape.into();
    let session_tape: Vec<_> = (0..tape.len())
        .map(|frame| tape.frame(frame).unwrap())
        .collect();
    let (outcome, map_node) = assets.play(progress, tape, check_entry);
    continuous::verify_map_entry_completion_and_reload(
        &assets.manifest,
        map_node,
        progress,
        &session_tape,
        &outcome,
    );
    outcome
}

/// One extracted level's original assets, loaded once so a route can be
/// replayed many times from its source map node.
pub(super) struct RouteAssets {
    pub(super) manifest: LevelManifest,
    layout: LevelLayout,
    objblock: Vec<u8>,
    tables: sml2_core::object::ObjectTables,
    hud: HudStrip,
    bg_base: Vec<u8>,
    tile_anim: Vec<u8>,
    edge: Vec<u8>,
    scene: Vec<u8>,
    pub(super) nodes: MapNodes,
    /// Divider ticks the map-entered session spends before the level's
    /// first frame, per entry progress byte.
    entry_ticks: std::sync::Mutex<std::collections::HashMap<u8, u64>>,
}

impl RouteAssets {
    /// Frames a map-entered session steps (the A press, then the entry
    /// transition) before this level's first frame. Each step ticks the
    /// session's divider once.
    fn map_entry_ticks(&self, node: u8, progress: u8) -> u64 {
        use crate::session::{SessionAssets, StepOutcome, MAP_ENTRY_FRAMES};
        *self
            .entry_ticks
            .lock()
            .unwrap()
            .entry(progress)
            .or_insert_with(|| {
                let path = std::env::temp_dir().join(format!(
                    "sml2_route_entry_{}_{node}_{progress}.sav",
                    std::process::id()
                ));
                let (mut runner, ..) = continuous::map_entry_runner(
                    SessionAssets::load().unwrap(),
                    node,
                    progress,
                    &path,
                );
                runner.step_frame(PlayerKeys {
                    a: true,
                    ..Default::default()
                });
                let entry = (1..=u64::from(MAP_ENTRY_FRAMES) + 8)
                    .find(|_| {
                        matches!(
                            runner.step_frame(PlayerKeys::default()),
                            StepOutcome::LevelEntered(_)
                        )
                    })
                    .expect("the level loads from its map node");
                let _ = fs::remove_file(path);
                1 + entry
            })
    }

    pub(super) fn load(level: u8) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
        let levels = root.join("levels");
        let read = |name: String| fs::read(levels.join(name)).unwrap();
        let manifest: LevelManifest = serde_json::from_str(
            &fs::read_to_string(levels.join(format!("level{level:02}.manifest.json"))).unwrap(),
        )
        .unwrap();
        let layout = LevelLayout::from_bytes(&read(manifest.layout.file.clone())).unwrap();
        Self {
            layout,
            objblock: read(format!("level{level:02}.objblock.bin")),
            tables: serde_json::from_str(&fs::read_to_string(root.join("objects.json")).unwrap())
                .unwrap(),
            hud: serde_json::from_str(&fs::read_to_string(root.join("hud.json")).unwrap()).unwrap(),
            bg_base: read(format!("level{level:02}.bgblock.bin")),
            tile_anim: fs::read(root.join("tileanim.bin")).unwrap(),
            edge: read(format!("level{level:02}.edge.bin")),
            scene: read(format!("level{level:02}.scene.bin")),
            nodes: load_map_nodes(),
            entry_ticks: Default::default(),
            manifest,
        }
    }

    /// Enter from the unique source map node with `progress` in its slot,
    /// then run the tape through live `run_play`. Returns the source node.
    pub(super) fn play(
        &self,
        progress: u8,
        tape: PlayTape,
        check_entry: impl FnOnce(
            &LevelManifest,
            &LevelLayout,
            &PlayerSim,
            &sml2_core::camera::CameraSim,
            &MapNodes,
        ),
    ) -> (PlayOutcome, u8) {
        let manifest = &self.manifest;
        let mut sim = PlayerSim::new();
        seed_level_timer(&mut sim, manifest);
        sim.powerup = 0;
        seed_unique_level_map_node(&mut sim, manifest.level_id, &self.nodes)
            .expect("unique source node");
        let slot = self
            .nodes
            .progress_slot(sim.map_node)
            .expect("source progress slot");
        sim.level_progress[slot as usize] = progress;
        let entry = manifest
            .header
            .entry(sim.level_progress[slot as usize], manifest.secret_entry);
        let mut cam = sml2_core::camera::CameraSim::new(entry.start.camera_x, entry.start.camera_y);
        apply_level_entry(&mut sim, &mut cam, entry);
        cam.prev_mario_y_lo = sim.y as u8;
        cam.map_scroll_limit = manifest.properties.unknown != 0;
        cam.auto_scroll = manifest.properties.auto_scroll;
        cam.scene_state = manifest.header.scene_state;
        sim.seed_moon_physics(manifest.properties.space_physics);
        check_entry(manifest, &self.layout, &sim, &cam, &self.nodes);
        assert_eq!((sim.powerup, sim.dead, sim.invincibility), (0, 0, 0));
        let mut obj = sml2_core::objects::ObjectSim::new(manifest.spawns.records.clone(), cam.x);
        crate::game::initialize_level_objects(&mut obj, 0);
        assert_eq!(sim.moon_float_timer, 0);
        let spr = SpriteCtx {
            insts: Vec::new(),
            objblock: self.objblock.clone(),
            tables: self.tables.clone(),
            level_id: manifest.level_id,
            obp0: manifest.header.palettes[1],
            obp1: manifest.header.palettes[2],
            player: None,
            player_shield: None,
            streak_counter: None,
            player_behind_bg: false,
            pipe_background: None,
            player_star_flash: false,
            player_pan: 0,
            scroll_pan: 0,
            bumpers: Vec::new(),
            fireballs: Vec::new(),
            boss_oam: Vec::new(),
        };
        let frames = tape.len() as u64;
        let map_node = sim.map_node;
        // The level continues the clock the map entry was running on.
        let mut divider = crate::divider_clock::VirtualDividerClock::new(0);
        divider.advance_ticks(self.map_entry_ticks(map_node, progress));
        let outcome = run_play(
            manifest,
            &self.layout,
            &self.bg_base,
            &self.tile_anim,
            &self.hud,
            self.hud.window_y,
            1,
            (SCREEN_W, SCREEN_H),
            Some(spr),
            sim,
            cam,
            obj,
            Some(self.edge.clone()),
            Some(self.scene.clone()),
            Some(tape),
            Some(frames),
            divider,
        )
        .expect("source route must use implemented native paths");
        (outcome, map_node)
    }
}

#[test]
fn direct_level_node_seed_requires_a_unique_source_mapping() {
    let nodes = load_map_nodes();
    assert!(!nodes.is_empty());
    let mut sim = PlayerSim::new();
    sim.map_node = 99;
    assert_eq!(seed_unique_level_map_node(&mut sim, 17, &nodes), None);
    assert_eq!(
        sim.map_node, 99,
        "two nodes require overworld route context"
    );
    assert_eq!(seed_unique_level_map_node(&mut sim, 24, &nodes), None);
    assert_eq!(sim.map_node, 99, "castle has no direct level node");
    assert_eq!(
        seed_unique_level_map_node(&mut sim, 0, &MapNodes::default()),
        None
    );
    assert_eq!(sim.map_node, 99, "missing tables cannot choose a node");
    assert_eq!(seed_unique_level_map_node(&mut sim, 0, &nodes), Some(0));
    assert_eq!(sim.map_node, 0);
}

/// The $08 timed-float level: horizontal inertia carries across A-only spans,
/// so the original ASCII tape can climb the source terrain without new input
/// syntax. No movement or physics state is rewritten after header entry.
#[test]
fn level29_timed_float_header_spawn_to_bell_through_live_play() {
    let tape = "R".repeat(100) + &("A".repeat(50) + &"R".repeat(100)).repeat(22);
    let outcome = run_header_route(29, tape, |manifest, layout, sim, cam, nodes| {
        assert_eq!(manifest.level_id, 28);
        assert_eq!(layout.cells[28 * 256 + 94], 0x4A);
        assert_eq!((sim.x, sim.y, cam.x, cam.y), (40, 452, 80, 438));
        assert_eq!(sim.moon, 8);
        assert_eq!(manifest.spawns.records.len(), 6);
        assert_eq!(
            (sim.map_node, nodes.progress_slot(sim.map_node)),
            (68, Some(41))
        );
    });
    let float = outcome
        .timed_float_use
        .expect("held A must consume the grounded float budget");
    assert_eq!(float.frame, 102, "second frame of the first held-A span");
    assert_eq!((float.timer_before, float.timer_after), (50, 49));
    assert_eq!(
        (float.vvel_before, float.vvel_after),
        (47, 47),
        "live frame includes gravity's moon apex snap"
    );
    assert!(outcome.timed_float_frames > 0);
    let arm = outcome
        .bell_arm
        .expect("native timed-float approach must arm the bell");
    // $15CF cancels timed flight on ceiling contact, allowing the tape to
    // land and resume its next jump sooner.
    assert_eq!((arm.frame, arm.position), (2284, (1507, 436)));
    assert_eq!(outcome.frames, 3400);
    assert_eq!((outcome.cam.x, outcome.cam.y), (1456, 448));
    assert_eq!(
        (
            arm.fade_timer,
            arm.music_request,
            arm.exit_tile_flag,
            arm.dead
        ),
        (192, 3, 0, 0)
    );
    assert_eq!(outcome.gameplay_death_frames, 0);
    assert_eq!(outcome.exit_completed_frame, Some(arm.frame + 192));
    assert_eq!(outcome.map_handoff_frame, Some(arm.frame + 193));
    assert_eq!((outcome.sim.fade_timer, outcome.sim.level_tally), (0, 1));
    assert!(outcome.sim.exit_completed && outcome.sim.map_handoff_done);
    assert_eq!(outcome.sim.map_node, 69);
    assert_eq!(outcome.sim.level_progress[41], 0x80);
    assert_eq!(outcome.sim.save_record(0)[8 + 41], 0x80);
    assert_eq!(
        outcome.sim.dead, 0xFF,
        "$3D573's post-clear node rewrite latch"
    );
    let handoff = outcome.map_handoff.unwrap();
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        (69, Some(4), Some(28), 2)
    );
    assert!(outcome.active_object_frames > 0);
    assert!(outcome.obj.unported.is_empty());
}

/// First played secret-entry route. These frozen held-button spans use ordinary
/// moon jumps and the source Up+A high jump to cross the lower spike corridors.
/// The upper $4B bell is covered by the separate hidden ladder route test.
#[test]
fn level19_secret_entry_to_plain_bell_through_live_play() {
    let right = PlayerKeys {
        right: true,
        ..Default::default()
    };
    let right_jump = PlayerKeys {
        right: true,
        a: true,
        ..Default::default()
    };
    let right_high_jump = PlayerKeys {
        right: true,
        up: true,
        a: true,
        ..Default::default()
    };
    let high_jump = PlayerKeys {
        up: true,
        a: true,
        ..Default::default()
    };
    let idle = PlayerKeys::default();
    let spans = [
        (20, right),
        (20, high_jump),
        (80, right_jump),
        (80, right),
        (60, right_jump),
        (60, right),
        (20, right_high_jump),
        (80, right_jump),
        (80, right),
        (20, right_jump),
        (20, right),
        (100, high_jump),
        (20, idle),
        (20, right),
        (80, high_jump),
        (100, idle),
        (40, high_jump),
        (20, idle),
        (20, right),
        (60, high_jump),
        (100, idle),
        (80, right),
        (220, idle),
    ];
    let tape = spans
        .into_iter()
        .flat_map(|(frames, held)| vec![held; frames])
        .collect::<Vec<_>>();
    let outcome = run_entry_route(19, 8, tape, |manifest, layout, sim, cam, nodes| {
        assert_eq!(manifest.level_id, 18);
        assert_eq!(manifest.spawns.records.len(), 25);
        assert_eq!((sim.x, sim.y, cam.x, cam.y), (1905, 436, 1938, 445));
        let copied = sml2_core::level::LevelStart {
            mario_y: sim.y,
            mario_x: sim.x,
            camera_y: cam.y,
            camera_x: cam.x,
        }
        .to_bytes();
        assert_eq!(copied, [0xB4, 1, 0x71, 7, 0xBD, 1, 0x92, 7]);
        assert_eq!(
            Some(copied),
            manifest.secret_entry.map(|entry| entry.to_bytes())
        );
        assert_eq!(
            (
                sim.secret_route,
                sim.level_progress_flags,
                sim.level_progress[16]
            ),
            (255, 8, 8)
        );
        assert_eq!(
            (sim.map_node, nodes.progress_slot(sim.map_node)),
            (60, Some(16))
        );
        assert_eq!(sim.moon, 8);
        assert_eq!(layout.cells[29 * 256 + 190], 0x4A);
        assert_eq!(layout.cells[11 * 256 + 190], 0x4B);
    });
    let arm = outcome
        .bell_arm
        .expect("genuine secret entry must reach the lower bell");
    assert_eq!((arm.frame, arm.position), (1173, (3043, 452)));
    assert_eq!(
        (
            arm.fade_timer,
            arm.music_request,
            arm.exit_tile_flag,
            arm.dead
        ),
        (192, 3, 0, 0)
    );
    assert_eq!(outcome.gameplay_death_frames, 0);
    assert_eq!(outcome.frames, 1400);
    assert_eq!(outcome.exit_completed_frame, Some(1365));
    assert_eq!(outcome.map_handoff_frame, Some(1366));
    assert_eq!((outcome.sim.x, outcome.sim.y), arm.position);
    assert_eq!((outcome.cam.x, outcome.cam.y), (2992, 448));
    assert_eq!(
        (
            outcome.sim.fade_timer,
            outcome.sim.level_tally,
            outcome.sim.dead
        ),
        (0, 1, 0)
    );
    assert!(outcome.sim.exit_phase && outcome.sim.exit_completed && outcome.sim.map_handoff_done);
    assert_eq!(outcome.sim.map_node, 60);
    assert_eq!(
        (outcome.sim.secret_route, outcome.sim.level_progress_flags),
        (0, 8)
    );
    assert_eq!(
        outcome.sim.level_progress[16], 0x80,
        "plain clear removes the entry's secret-route bit"
    );
    assert_eq!(outcome.sim.save_record(0)[8 + 16], 0x80);
    let handoff = outcome.map_handoff.unwrap();
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        (60, Some(4), Some(28), 2)
    );
    let float = outcome
        .timed_float_use
        .expect("native jumps must consume the genuine grounded budget");
    assert_eq!((float.timer_before, float.timer_after), (50, 49));
    assert_eq!(
        (float.frame, float.vvel_before, float.vvel_after),
        (22, 74, 73)
    );
    assert!(outcome.timed_float_frames > 0);
    assert!(outcome.active_object_frames > 0);
    assert!(outcome.obj.unported.is_empty());
}
