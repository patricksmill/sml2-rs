use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use sml2_core::map::save_record_valid;
use sml2_core::player_sim::PlayerKeys;

use crate::session::{
    AudioClock, SessionAssets, SessionRunner, SessionState, StepOutcome, MAP_ENTRY_FRAMES,
};

/// Press A on the map and run out the console's entry transition: the map
/// freezes, the palettes fade to white, and the level goes live 37 frames in.
fn press_a_and_wait_for_level(runner: &mut SessionRunner) -> StepOutcome {
    runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    let mut outcome = StepOutcome::Continue;
    for _ in 0..MAP_ENTRY_FRAMES + 8 {
        outcome = runner.step_frame(PlayerKeys::default());
        if matches!(outcome, StepOutcome::LevelEntered(_)) {
            break;
        }
    }
    outcome
}

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
    let _ = press_a_and_wait_for_level(&mut runner);

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
    let _ = press_a_and_wait_for_level(&mut runner);
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
    let _ = press_a_and_wait_for_level(&mut runner);
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

    let _ = press_a_and_wait_for_level(&mut runner);

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
    let _ = press_a_and_wait_for_level(&mut runner);

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
    let _ = press_a_and_wait_for_level(&mut runner);

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

#[test]
fn map_screen_loader_music_reaches_the_driver() {
    let tmp_save = temp_save_path("test_map_music");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);

    // The hub loader $3F156 stops music at $3F160 and ends with `ld a,6 /
    // ld [sMusicRequest],a` at $3F1FE; $3EA9C requests the same 6.
    runner.step_frame(PlayerKeys::default());
    assert_eq!(runner.sound_driver.active_song, 6, "hub theme");

    // Screen 1 is loaded by $3D208, which requests song 30.
    {
        let records = &runner.session.assets.node_records;
        match &mut runner.session.state {
            SessionState::Overworld(map) => {
                let _ = map.load_screen(1, 0, records);
            }
            _ => panic!("expected Overworld"),
        }
    }
    runner.step_frame(PlayerKeys::default());
    assert_eq!(runner.sound_driver.active_song, 30, "screen 1 loader song");

    // Screen 5 is loaded by $3D61F, which requests song 18.
    {
        let records = &runner.session.assets.node_records;
        match &mut runner.session.state {
            SessionState::Overworld(map) => {
                let _ = map.load_screen(5, 0, records);
            }
            _ => panic!("expected Overworld"),
        }
    }
    runner.step_frame(PlayerKeys::default());
    assert_eq!(runner.sound_driver.active_song, 18, "screen 5 loader song");

    // Re-loading the same screen must not restart the song.
    let before = runner.sound_driver.update_count;
    runner.step_frame(PlayerKeys::default());
    assert_eq!(runner.sound_driver.active_song, 18);
    assert_eq!(runner.sound_driver.update_count, before + 1);

    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn live_audio_clock_drops_a_stalls_surplus() {
    // 100 ms of wall clock owes 4410 samples, but a stalled frame must not push
    // them as one burst; the surplus is dropped rather than queued.
    let mut clock = AudioClock::backdated(std::time::Duration::from_millis(100));
    assert_eq!(clock.take(), 4 * 739);
    assert_eq!(clock.take(), 0);
}

#[test]
fn headless_runs_keep_the_frame_clock() {
    // The capture sweeps compare sample-for-sample, so headless sessions must
    // never take the wall-clock path.
    let tmp_save = temp_save_path("test_headless_clock");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    assert!(runner.audio_sync.is_none());
    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn map_level_entry_runs_the_console_transition() {
    let tmp_save = temp_save_path("test_map_transition");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    let _ = runner.step_frame(PlayerKeys::default());

    let plain_mean = runner.render_frame().iter().map(|&p| p as u32).sum::<u32>() / (160 * 144);

    // Pressing A queues the entry: the map stays on screen and its theme stops
    // ($3C1A2 requests $FF before the load), but the level is not live yet.
    let _ = runner.step_frame(PlayerKeys { a: true, ..Default::default() });
    assert!(matches!(runner.state, SessionState::Overworld(_)));
    assert!(runner.pending_level.is_some(), "entry queued");
    assert_eq!(runner.sound_driver.active_song, 0, "map theme stopped");

    // The first transition frame is one fade step brighter than the map; the
    // second step lands at the boundary of the hold, and the level goes live
    // MAP_ENTRY_FRAMES frames after the press.
    let faded_mean = runner.render_frame().iter().map(|&p| p as u32).sum::<u32>() / (160 * 144);
    assert!(
        faded_mean > plain_mean,
        "fade step lightens the map ({faded_mean} vs {plain_mean})"
    );

    let mut entered = None;
    for i in 1..=64 {
        if let StepOutcome::LevelEntered(id) = runner.step_frame(PlayerKeys::default()) {
            entered = Some((i, id));
            break;
        }
    }
    // Measured on the console: the last map frame is $0C, the level's first
    // frame is the 37th after the press.
    assert_eq!(entered, Some((37, 0)));
    assert!(matches!(runner.state, SessionState::InLevel(_)));
    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn pipe_travel_draws_mario_behind_the_background() {
    // $530D: `sPipeTravelDirection >= $10` clears `$FFBB`, and $52D5-$52DD then
    // sets attr bit 7 on every player OAM entry, so the pipe's BG covers Mario.
    // Arming values 1/2 and "no pipe" stay in front (a measured walk keeps OAM
    // attr $20, bit 7 clear).
    for armed in [0u8, 1, 2, 0x0E, 0x0F] {
        assert!(!crate::session::player_behind_bg(armed), "{armed:#04x}");
    }
    for travelling in [0x10u8, 0x20, 0x30, 0x40] {
        assert!(crate::session::player_behind_bg(travelling), "{travelling:#04x}");
    }
}

#[test]
fn hub_frames_advance_the_wave_strips() {
    // `$3EAFF` runs `$3E853` once per frame on the hub only; the strips must move
    // and the phase must advance with the frame count.
    let tmp_save = temp_save_path("test_wave");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    let _ = runner.step_frame(PlayerKeys::default());
    let first = runner.map_wave.clone().expect("seeded on the hub");
    for _ in 0..8 {
        let _ = runner.step_frame(PlayerKeys::default());
    }
    let later = runner.map_wave.clone().expect("still seeded");
    assert_ne!(first.strips, later.strips, "the strips rotate");
    assert_eq!(later.phase, first.phase.wrapping_add(8), "one call per frame");
    let _ = fs::remove_file(&tmp_save);
}

#[test]
fn pan_arrows_blink_on_the_console_period() {
    let tmp_save = temp_save_path("test_probe_blink");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    let _ = runner.step_frame(PlayerKeys::default());
    // Frozen camera (no keys), latch forced on: only the arrows can change.
    let mut runs: Vec<(bool, usize)> = Vec::new();
    for _ in 0..48 {
        if let SessionState::Overworld(m) = &mut runner.state {
            m.scroll_only_latch = 0xFF;
        }
        let pan = runner.render_frame();
        if let SessionState::Overworld(m) = &mut runner.state {
            m.scroll_only_latch = 0;
        }
        let ctl = runner.render_frame();
        let mut diff = 0;
        for (cx, cy) in [(72usize, 8usize), (8, 68), (144, 68), (72, 124)] {
            for y in cy..cy + 8 {
                for x in cx..cx + 8 {
                    if pan[y * 160 + x] != ctl[y * 160 + x] {
                        diff += 1;
                    }
                }
            }
        }
        let on = diff > 0;
        match runs.last_mut() {
            Some((cur, len)) if *cur == on => *len += 1,
            _ => runs.push((on, 1)),
        }
        let _ = runner.step_frame(PlayerKeys::default());
    }
    // `$3C9F4` reads `$FF97` bit 4, so the border arrows run 16 frames on and
    // 16 off; measured on the console, B+Right gave frames 30..45 of `$7E`.
    let exp: Vec<(bool, usize)> = vec![(true, 15), (false, 16), (true, 16)];
    assert_eq!(&runs[..3], &exp[..], "blink rhythm: {runs:?}");
    let _ = fs::remove_file(&tmp_save);
}
