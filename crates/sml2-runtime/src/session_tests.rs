use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use sml2_core::map::save_record_valid;
use sml2_core::player_sim::PlayerKeys;

use crate::session::{SessionAssets, SessionRunner, SessionState, StepOutcome};

fn temp_save_path(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("sml2_{name}_{id}_{}.sav", std::process::id()))
}

#[test]
fn hub_navigation_to_level_1_entry_and_clear_bell_return() {
    let tmp_save = temp_save_path("test_hub_level1_clear");
    let _ = fs::remove_file(&tmp_save);

    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);

    // Initial state: Overworld screen 0, node 0, 5 lives, level 1 uncleared.
    match &runner.state {
        SessionState::Overworld(map) => {
            assert_eq!(map.screen_id, 0);
            assert_eq!(map.node, 0);
            assert_eq!(map.level_progress[0] & 0x80, 0);
        }
        _ => panic!("expected Overworld initial state"),
    }
    assert_eq!(runner.lives, 5);

    // Press A to enter Level 1 (level index 0).
    let outcome = runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert_eq!(outcome, StepOutcome::LevelEntered(0));

    // Session is now in level 0.
    match &mut runner.state {
        SessionState::InLevel(ctx) => {
            assert_eq!(ctx.manifest.header.level_id, 0);
            assert_eq!(ctx.sim.map_node, 0);
            // Arm exit bell directly to simulate touching the exit bell.
            ctx.sim.exit_phase = true;
            ctx.sim.fade_timer = 2;
        }
        _ => panic!("expected InLevel state"),
    }

    // Step frames until level exit handoff completes.
    let mut exited = false;
    for _ in 0..100 {
        let outcome = runner.step_frame(PlayerKeys::default());
        if let StepOutcome::LevelExited {
            node,
            screen,
            cleared,
        } = outcome
        {
            assert_eq!(node, 0x14, "cleared level 1 hands off to node $14 on Hub");
            assert_eq!(screen, 0, "screen 0 is Hub");
            assert!(cleared, "level was cleared");
            exited = true;
            break;
        }
    }
    assert!(exited, "level exit must occur after bell arm");

    // Verify session returned to Overworld at node $14 with progress bit set.
    match &runner.state {
        SessionState::Overworld(map) => {
            assert_eq!(map.screen_id, 0);
            assert_eq!(map.node, 0x14);
            assert_eq!(
                map.level_progress[0] & 0x80,
                0x80,
                "progress bit 7 must be set"
            );
        }
        _ => panic!("expected Overworld state after exit"),
    }
    assert_eq!(runner.level_tally, 1);

    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn in_level_hazard_death_and_fade_returns_to_hub_with_decremented_lives() {
    let tmp_save = temp_save_path("test_level_death");
    let _ = fs::remove_file(&tmp_save);

    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);

    // Enter level 0.
    let outcome = runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert_eq!(outcome, StepOutcome::LevelEntered(0));
    assert_eq!(runner.lives, 5);

    // Trigger death: dead = 0xFF, dead_fade = true, fade_timer = 3 for fast deterministic test.
    match &mut runner.state {
        SessionState::InLevel(ctx) => {
            ctx.sim.dead = 0xFF;
            ctx.sim.dead_fade = true;
            ctx.sim.fade_timer = 3;
        }
        _ => panic!("expected InLevel state"),
    }

    let mut exited = false;
    for _ in 0..100 {
        let outcome = runner.step_frame(PlayerKeys::default());
        if let StepOutcome::LevelExited {
            node,
            screen,
            cleared,
        } = outcome
        {
            assert_eq!(node, 0, "death on node 0 returns to node 0");
            assert_eq!(screen, 0, "returns to screen 0");
            assert!(!cleared, "level was not cleared");
            exited = true;
            break;
        }
    }
    assert!(exited, "level exit must complete on death");

    // Lives decremented from 5 to 4.
    assert_eq!(runner.lives, 4);

    // Returned to overworld at node 0, level 1 remains uncleared.
    match &runner.state {
        SessionState::Overworld(map) => {
            assert_eq!(map.screen_id, 0);
            assert_eq!(map.node, 0);
            assert_eq!(map.level_progress[0] & 0x80, 0);
            assert_eq!(map.mario_dead, 0xFF);
        }
        _ => panic!("expected Overworld state"),
    }

    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn save_file_persistence_writes_disk_record_and_reloads_accurately() {
    let tmp_save = temp_save_path("test_save_persistence");
    let _ = fs::remove_file(&tmp_save);

    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);

    // Entering a level triggers $2934 save.
    let outcome = runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert_eq!(outcome, StepOutcome::LevelEntered(0));
    assert!(tmp_save.exists(), "save file must be written to disk");

    let rec = fs::read(&tmp_save).expect("save file must exist");
    assert!(save_record_valid(&rec), "save record must be valid");
    assert_eq!(rec[0x43], 5, "initial lives");

    // Clear level.
    match &mut runner.state {
        SessionState::InLevel(ctx) => {
            ctx.sim.exit_phase = true;
            ctx.sim.fade_timer = 2;
        }
        _ => panic!("expected InLevel state"),
    }

    for _ in 0..100 {
        if let StepOutcome::LevelExited { .. } = runner.step_frame(PlayerKeys::default()) {
            break;
        }
    }

    // Verify disk record updated with clear.
    let rec_after = fs::read(&tmp_save).expect("save file must exist");
    assert!(save_record_valid(&rec_after));
    assert_eq!(rec_after[0x08] & 0x80, 0x80, "progress byte 0 bit 7 set");
    assert_eq!(rec_after[0x42], 1, "level tally incremented");
    assert_eq!(rec_after[0x43], 5, "lives preserved");

    // Create a new runner from the same save path.
    let assets2 = SessionAssets::load().expect("session assets must load");
    let runner2 = SessionRunner::new(assets2, tmp_save.clone(), 1, (160, 144), true);

    assert_eq!(runner2.lives, 5);
    assert_eq!(runner2.level_tally, 1);
    match &runner2.state {
        SessionState::Overworld(map) => {
            assert_eq!(map.screen_id, 0);
            assert_eq!(map.node, 0x14);
            assert_eq!(map.level_progress[0] & 0x80, 0x80);
        }
        _ => panic!("expected Overworld state on restore"),
    }

    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn game_over_when_lives_exhausted() {
    let tmp_save = temp_save_path("test_game_over");
    let _ = fs::remove_file(&tmp_save);

    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    runner.lives = 0; // Will become game over on death

    let outcome = runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert_eq!(outcome, StepOutcome::LevelEntered(0));

    match &mut runner.state {
        SessionState::InLevel(ctx) => {
            ctx.sim.dead = 0xFF;
            ctx.sim.dead_fade = true;
            ctx.sim.fade_timer = 2;
        }
        _ => panic!("expected InLevel state"),
    }

    let mut game_over = false;
    for _ in 0..100 {
        let outcome = runner.step_frame(PlayerKeys::default());
        if outcome == StepOutcome::GameOver {
            game_over = true;
            break;
        }
    }
    assert!(game_over, "must trigger GameOver when lives reach 0");
    assert!(matches!(runner.state, SessionState::GameOver));

    // Stepping past GameOver restarts at Hub with 5 lives.
    let outcome = runner.step_frame(PlayerKeys::default());
    assert_eq!(outcome, StepOutcome::Continue);
    assert_eq!(runner.lives, 5);
    match &runner.state {
        SessionState::Overworld(map) => {
            assert_eq!(map.screen_id, 0);
            assert_eq!(map.node, 0);
        }
        _ => panic!("expected Overworld after game over reset"),
    }

    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn session_audio_synthesis_during_gameplay() {
    let tmp_save = temp_save_path("test_session_audio");
    let _ = fs::remove_file(&tmp_save);

    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);

    // Initial overworld frame generates one console frame of samples: 738
    // stereo pairs on the first frame of the 738/738/739 cadence (1476 f32s).
    let _ = runner.step_frame(PlayerKeys::default());
    assert_eq!(runner.audio_samples().len(), 1476);

    // Enter Level 1
    let outcome = runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert_eq!(outcome, StepOutcome::LevelEntered(0));

    // Step several frames of in-level gameplay to advance music sequencer and SFX
    let mut audible_frames = 0;
    for _ in 0..10 {
        runner.step_frame(PlayerKeys {
            right: true,
            ..Default::default()
        });
        // One console frame is 738 or 739 stereo pairs (738.3533 average).
        let len = runner.audio_samples().len();
        assert!(
            len == 1476 || len == 1478,
            "frame produced {len} floats, expected 1476 or 1478"
        );
        for &s in runner.audio_samples() {
            assert!(s >= -1.0 && s <= 1.0, "Sample out of bounds: {s}");
        }
        if runner.audio_samples().iter().any(|&s| s.abs() > 0.0) {
            audible_frames += 1;
        }
    }

    assert!(
        audible_frames > 0,
        "In-level gameplay must generate audible audio samples"
    );

    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn level_entry_publishes_the_active_song_for_the_timer_gate() {
    let tmp_save = temp_save_path("test_audio_song_sync");
    let _ = fs::remove_file(&tmp_save);

    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    let outcome = runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert_eq!(outcome, StepOutcome::LevelEntered(0));

    let song = match &runner.state {
        SessionState::InLevel(ctx) => ctx.sim.level_song,
        _ => panic!("expected in-level state"),
    };
    assert_eq!(song, 0x01, "level 1 header music");

    // One in-level frame: the header's music request reaches the driver, and
    // the driver's sCurSong is published back for $5DCB's `ld a,[sCurSong]`
    // gate on the next frame's timer tick.
    runner.step_frame(PlayerKeys::default());
    assert_eq!(runner.sound_driver.active_song, song);
    match &runner.state {
        SessionState::InLevel(ctx) => assert_eq!(ctx.sim.current_song, song),
        _ => panic!("expected in-level state"),
    }

    let _ = fs::remove_file(&tmp_save);
}
