use super::*;

/// Extracted level28 (decimal CLI id 27), ordinary header entry. The fixed
/// right-only tape descends from the upper starting ledge and walks the lower
/// corridor to the source $4A cell (78,30). Objects, scenes, camera, fade and
/// handoff all run through the same dispatch as interactive play.
#[test]
fn level28_header_spawn_to_bell_through_live_play() {
    let outcome = run_header_route(28, "R".repeat(1400), |manifest, layout, sim, cam, nodes| {
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
    assert!(outcome.sim.exit_phase && outcome.sim.exit_completed && outcome.sim.map_handoff_done);
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

fn run_header_route(
    level: u8,
    tape: String,
    check_entry: impl FnOnce(
        &LevelManifest,
        &LevelLayout,
        &PlayerSim,
        &sml2_core::camera::CameraSim,
        &MapNodes,
    ),
) -> PlayOutcome {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    let levels = root.join("levels");
    let manifest: LevelManifest = serde_json::from_str(
        &fs::read_to_string(levels.join(format!("level{level:02}.manifest.json"))).unwrap(),
    )
    .unwrap();
    let layout =
        LevelLayout::from_bytes(&fs::read(levels.join(&manifest.layout.file)).unwrap()).unwrap();
    let entry = manifest.header.entry(0, manifest.secret_entry);
    let mut sim = PlayerSim::new();
    seed_level_timer(&mut sim, &manifest);
    sim.powerup = 0;
    let mut cam = sml2_core::camera::CameraSim::new(entry.start.camera_x, entry.start.camera_y);
    apply_level_entry(&mut sim, &mut cam, entry);
    let nodes = load_map_nodes();
    seed_unique_level_map_node(&mut sim, manifest.level_id, &nodes).expect("unique source node");
    cam.prev_mario_y_lo = sim.y as u8;
    cam.map_scroll_limit = manifest.properties.unknown != 0;
    cam.auto_scroll = manifest.properties.auto_scroll;
    cam.scene_state = manifest.header.scene_state;
    sim.seed_moon_physics(manifest.properties.space_physics);
    check_entry(&manifest, &layout, &sim, &cam, &nodes);
    assert_eq!((sim.powerup, sim.dead, sim.invincibility), (0, 0, 0));
    let obj = sml2_core::objects::ObjectSim::new(manifest.spawns.records.clone(), cam.x);
    assert_eq!(sim.moon_float_timer, 0);
    let spr = SpriteCtx {
        insts: Vec::new(),
        objblock: fs::read(levels.join(format!("level{level:02}.objblock.bin"))).unwrap(),
        tables: serde_json::from_str(&fs::read_to_string(root.join("objects.json")).unwrap())
            .unwrap(),
        level_id: manifest.level_id,
        obp0: manifest.header.palettes[1],
        obp1: manifest.header.palettes[2],
        player: None,
        player_shield: None,
        player_pan: 0,
        scroll_pan: 0,
        bumpers: Vec::new(),
        fireballs: Vec::new(),
        boss_oam: Vec::new(),
    };
    let hud: HudStrip =
        serde_json::from_str(&fs::read_to_string(root.join("hud.json")).unwrap()).unwrap();
    let frames = tape.len() as u64;
    run_play(
        &manifest,
        &layout,
        &fs::read(levels.join(format!("level{level:02}.bgblock.bin"))).unwrap(),
        &fs::read(root.join("tileanim.bin")).unwrap(),
        &hud,
        hud.window_y,
        1,
        (SCREEN_W, SCREEN_H),
        Some(spr),
        sim,
        cam,
        obj,
        Some(fs::read(levels.join(format!("level{level:02}.edge.bin"))).unwrap()),
        Some(fs::read(levels.join(format!("level{level:02}.scene.bin"))).unwrap()),
        Some(tape),
        Some(frames),
    )
    .expect("source route must use implemented native paths")
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
    assert_eq!((arm.frame, arm.position), (2584, (1507, 436)));
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
