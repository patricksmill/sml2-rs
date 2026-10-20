use super::*;

/// One uninterrupted normal-entry run crosses the thorn beds, reveals the
/// hidden ladder, and follows the upper platforms to the alternate bell.
#[test]
fn level19_header_spawn_hidden_ladder_to_alternate_bell_through_live_play() {
    let right = PlayerKeys {
        right: true,
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
    let left = PlayerKeys {
        left: true,
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
        // Catch column 81's raised foothold before the taller formation.
        (210, right_high_jump),
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
    tape.extend(
        [
            (25, right),
            (55, right_high_jump),
            (65, right),
            (7, left_high_jump),
            (150, high_jump),
            (10, idle),
        ]
        .into_iter()
        .flat_map(|(n, k)| vec![k; n]),
    );
    // Reach the ladder base and settle after braking, using ordinary input.
    tape.extend(
        [(25, right), (5, left), (3, idle)]
            .into_iter()
            .flat_map(|(frames, held)| vec![held; frames]),
    );
    let ladder = [
        // Reveal the bottom hidden block from the original floor.
        (30, right),
        (5, left),
        (100, high_jump),
        (100, idle),
        // Clear the source platform's underside and land on row 25.
        (40, right),
        (4, left),
        (100, high_jump),
        (100, idle),
        // Reveal row 19 while landing on the now-solid row-23 block.
        (30, high_jump),
        (30, right_high_jump),
        (5, left_high_jump),
        (130, high_jump),
        (10, idle),
        // Jump around the row-19 block and land on its top.
        (18, left_high_jump),
        (6, right_high_jump),
        (30, high_jump),
        (15, right_high_jump),
        (7, left_high_jump),
        (130, high_jump),
        (10, idle),
        // Reveal row 15, then jump around it to its top.
        (100, high_jump),
        (30, idle),
        (18, left_high_jump),
        (6, right_high_jump),
        (30, high_jump),
        (20, right_high_jump),
        (7, left_high_jump),
        (130, high_jump),
        (10, idle),
        // Land on the first upper platform at (1983,164).
        (40, high_jump),
        (40, right_high_jump),
        (5, left_high_jump),
        (130, high_jump),
        (10, idle),
    ];
    tape.extend(
        ladder
            .into_iter()
            .flat_map(|(frames, held)| vec![held; frames]),
    );
    // Fixed travel spans for the six source platform gaps. Each leftward air
    // brake stops the drift before landing; releasing A allows the next jump.
    for travel in [175, 175, 175, 165, 155, 115] {
        tape.extend(
            [
                (travel, right_high_jump),
                (7, left_high_jump),
                (140, high_jump),
                (10, idle),
            ]
            .into_iter()
            .flat_map(|(frames, held)| vec![held; frames]),
        );
    }
    tape.extend(vec![right; 200]);
    tape.extend(vec![idle; 220]);
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
        assert_eq!(layout.cells[11 * 256 + 190], 0x4B);
        for row in [23, 19, 15] {
            assert_eq!(layout.cells[row * 256 + 121], 0x7E, "source hidden ladder");
        }
    });
    for row in [23, 19, 15] {
        let cell = (row * 256 + 121) as u32;
        assert_eq!(
            outcome
                .sim
                .layout_writes
                .iter()
                .find(|(key, _)| *key == cell)
                .map(|(_, tile)| *tile),
            Some(0x07),
            "native bonks must reveal each standable foothold"
        );
    }
    let arm = outcome
        .bell_arm
        .expect("the native ladder and platform approach must reach $4B");
    assert_eq!((arm.frame, arm.position), (6922, (3033, 164)));
    assert_eq!(
        (
            arm.fade_timer,
            arm.music_request,
            arm.exit_tile_flag,
            arm.dead
        ),
        (192, 3, 0xFF, 0)
    );
    assert_eq!(outcome.gameplay_death_frames, 0);
    assert_eq!(outcome.frames, 7254);
    assert_eq!(outcome.exit_completed_frame, Some(7114));
    assert_eq!(outcome.map_handoff_frame, Some(7115));
    assert_eq!((outcome.sim.x, outcome.sim.y), arm.position);
    assert_eq!((outcome.cam.x, outcome.cam.y), (2992, 181));
    assert_eq!(
        (
            outcome.sim.fade_timer,
            outcome.sim.level_tally,
            outcome.sim.dead
        ),
        (0, 0, 0),
        "$4B guards the BCD tally"
    );
    assert!(outcome.sim.exit_phase && outcome.sim.exit_completed && outcome.sim.map_handoff_done);
    assert_eq!(
        (outcome.sim.secret_route, outcome.sim.level_progress_flags),
        (0, 0)
    );
    // $3EF56 rewrites node 60 to slotless node 64 before progress marking;
    // $3D540 then selects node 67 and ORs bit 0 into the original byte $00.
    assert_eq!(outcome.sim.map_node, 67);
    assert_eq!(outcome.sim.level_progress[16], 1);
    assert_eq!(outcome.sim.save_record(0)[8 + 16], 1);
    let handoff = outcome.map_handoff.unwrap();
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        (67, Some(4), Some(28), 1)
    );
    assert!(!handoff.cutscene);
    let float = outcome
        .timed_float_use
        .expect("the climb consumes the source float budget");
    assert_eq!(
        (
            float.frame,
            float.timer_before,
            float.timer_after,
            float.vvel_before,
            float.vvel_after
        ),
        (82, 50, 49, 74, 73)
    );
    assert!(outcome.timed_float_frames > 0);
    assert!(outcome.active_object_frames > 0);
    assert!(outcome.obj.unported.is_empty());
}
