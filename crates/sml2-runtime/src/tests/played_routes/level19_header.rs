use super::*;

/// The normal header entry crosses the early steps and thorn beds before joining
/// the lower corridor. Every foothold and budget refresh comes from live play.
#[test]
fn level19_header_spawn_to_plain_bell_through_live_play() {
    let right = PlayerKeys {
        right: true,
        ..Default::default()
    };
    let right_jump = PlayerKeys {
        right: true,
        a: true,
        ..Default::default()
    };
    let high_jump = PlayerKeys {
        up: true,
        a: true,
        ..Default::default()
    };
    let right_high_jump = PlayerKeys {
        right: true,
        up: true,
        a: true,
        ..Default::default()
    };
    let left_high_jump = PlayerKeys {
        left: true,
        up: true,
        a: true,
        ..Default::default()
    };
    let idle = PlayerKeys::default();
    let mut tape = vec![right; 81];
    // Handwritten native fixture, not recorded ROM input: take off one update
    // later so the descent clears the first moving enemy with the loader clock.
    for _ in 0..9 {
        tape.extend(
            [(80, right_high_jump), (40, right)]
                .into_iter()
                .flat_map(|(frames, held)| vec![held; frames]),
        );
    }
    let first_half = [
        (40, right_high_jump),
        (10, right),
        // Land on column 60's row-27 foothold to refresh the float budget.
        (60, right_high_jump),
        (7, left_high_jump),
        (150, high_jump),
        (10, idle),
        // Cross the eight-cell spike bed and land on column 69.
        (150, right_high_jump),
        (7, left_high_jump),
        (150, high_jump),
        (10, idle),
        // Catch column 81's raised foothold before the taller formation, then
        // leave it before the enemy below fires at the landing spot.
        (250, right_high_jump),
        (7, left_high_jump),
        (150, high_jump),
        (10, idle),
        // Clear that formation and settle on the first lower safe island.
        (180, right_high_jump),
        (6, left_high_jump),
        (180, high_jump),
        (10, idle),
    ];
    tape.extend(
        first_half
            .into_iter()
            .flat_map(|(frames, held)| vec![held; frames]),
    );
    for travel in [120, 150] {
        tape.extend(
            [
                (travel, right_high_jump),
                (7, left_high_jump),
                (150, high_jump),
                (10, idle),
            ]
            .into_iter()
            .flat_map(|(frames, held)| vec![held; frames]),
        );
    }
    let lower_corridor = [
        // Approach the island's right edge; releasing A keeps the next jump
        // below the upper enemy while crossing the remaining thorn gap.
        (25, right),
        (55, right_high_jump),
        (65, right),
        (7, left_high_jump),
        (150, high_jump),
        (10, idle),
        (15, right),
        // Fixed lower-corridor movement, with ordinary enemies still active.
        (16, right),
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
    tape.extend(
        lower_corridor
            .into_iter()
            .flat_map(|(frames, held)| vec![held; frames]),
    );
    let outcome = run_header_route(19, tape, |manifest, layout, sim, cam, nodes| {
        assert_eq!(manifest.level_id, 18);
        assert_eq!(manifest.spawns.records.len(), 25);
        assert_eq!((sim.x, sim.y, cam.x, cam.y), (45, 436, 80, 432));
        let copied = sml2_core::level::LevelStart {
            mario_y: sim.y,
            mario_x: sim.x,
            camera_y: cam.y,
            camera_x: cam.x,
        }
        .to_bytes();
        assert_eq!(copied, [0xB4, 1, 0x2D, 0, 0xB0, 1, 0x50, 0]);
        assert_eq!(
            copied,
            manifest
                .header
                .entry(0, manifest.secret_entry)
                .start
                .to_bytes()
        );
        assert_ne!(
            Some(copied),
            manifest.secret_entry.map(|entry| entry.to_bytes())
        );
        assert_eq!(
            (
                sim.secret_route,
                sim.level_progress_flags,
                sim.level_progress[16]
            ),
            (0, 0, 0)
        );
        assert_eq!(
            (sim.map_node, nodes.progress_slot(sim.map_node)),
            (60, Some(16))
        );
        assert_eq!(sim.moon, 8);
        assert_eq!(
            (
                manifest.header.time_limit,
                manifest.header.music_track,
                sim.level_song
            ),
            (7, 19, 19)
        );
        assert_eq!(sim.level_timer, sml2_core::level_timer::LevelTimer::new(7));
        assert_eq!(layout.cells[29 * 256 + 190], 0x4A);
    });
    let arm = outcome
        .bell_arm
        .expect("the header approach must reach the source plain bell");
    assert_eq!((arm.frame, arm.position), (4648, (3043, 452)));
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
    assert_eq!(outcome.frames, 4875);
    assert_eq!(outcome.exit_completed_frame, Some(4840));
    assert_eq!(outcome.map_handoff_frame, Some(4841));
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
    assert_eq!(
        (outcome.sim.secret_route, outcome.sim.level_progress_flags),
        (0, 0)
    );
    assert_eq!(outcome.sim.map_node, 60);
    assert_eq!(outcome.sim.level_progress[16], 0x80);
    assert_eq!(outcome.sim.save_record(0)[8 + 16], 0x80);
    let handoff = outcome.map_handoff.unwrap();
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        (60, Some(4), Some(28), 2)
    );
    assert!(!handoff.cutscene);
    let float = outcome
        .timed_float_use
        .expect("the header route consumes the source float budget");
    assert_eq!(
        (
            float.frame,
            float.timer_before,
            float.timer_after,
            float.vvel_before,
            float.vvel_after
        ),
        (83, 50, 49, 74, 73)
    );
    assert!(outcome.timed_float_frames > 0);
    assert!(outcome.active_object_frames > 0);
    assert!(outcome.obj.unported.is_empty());
}
