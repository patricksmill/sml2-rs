use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use sml2_core::map::save_record_valid;
use sml2_core::player_sim::PlayerKeys;

use crate::session::{
    AudioClock, SessionAssets, SessionRunner, SessionState, StepOutcome, MAP_ENTRY_FRAMES,
};

/// Press A on the map and run out the console's entry transition: the map
/// freezes, the palettes fade to white, and the level goes live 73 frames in.
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
fn arena_reload_discards_terrain_edits_from_the_previous_map() {
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(
        assets,
        temp_save_path("arena_terrain_reload"),
        1,
        (160, 144),
        true,
    );
    press_a_and_wait_for_level(&mut runner);
    if let SessionState::InLevel(ctx) = &mut runner.state {
        // Scene 12's first opcode is `$62`: `$25AF` calls `$0361` to
        // replace the whole decompressed map before displaying the arena.
        ctx.sim.layout_writes.push((7 * 256 + 18, 0x60));
        ctx.cam.scene_id = 12;
        ctx.scene_vm.start();
    } else {
        panic!("expected level entry");
    }
    for _ in 0..128 {
        runner.step_frame(PlayerKeys::default());
        if let SessionState::InLevel(ctx) = &runner.state {
            if ctx.sim.level_gfx_index == 2 {
                assert!(ctx.sim.layout_writes.is_empty(), "$0361 replaced the map");
                return;
            }
        }
    }
    panic!("scene did not reach its arena reload");
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

/// Walk right into level 1's first enemy and run out the death fade. Returns
/// the outcome that ended the level, with nothing rewritten mid-level.
fn die_walking_right(runner: &mut SessionRunner) -> StepOutcome {
    assert!(matches!(
        press_a_and_wait_for_level(runner),
        StepOutcome::LevelEntered(0)
    ));
    let right = PlayerKeys {
        right: true,
        ..Default::default()
    };
    for _ in 0..3000 {
        let outcome = runner.step_frame(right);
        if !matches!(outcome, StepOutcome::Continue) {
            return outcome;
        }
    }
    panic!("walking right must meet an enemy and end the level");
}

/// `$311A`: `sub 1 / daa` borrows only from 0, so the last life's death
/// continues to the map with 0 lives.
#[test]
fn the_last_life_death_continues_with_zero_lives() {
    let tmp_save = temp_save_path("test_last_life");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    runner.lives = 1;
    let outcome = die_walking_right(&mut runner);
    assert!(
        matches!(outcome, StepOutcome::LevelExited { cleared: false, .. }),
        "{outcome:?}"
    );
    assert_eq!(runner.lives, 0);
    assert!(matches!(runner.state, SessionState::Overworld(_)));
    let _ = fs::remove_file(&tmp_save);
}

/// A death at 0 lives borrows into `$27AF` (state `$05`): lives back to 5,
/// coins and money bags cleared, the golden coins folded, charged against the
/// tally and wiped, the `$2934` save, music 7. State `$07` then draws the
/// castle picture with Mario until the 255-frame fade ends and START goes to
/// state `$00`, the title.
#[test]
fn a_death_at_zero_lives_runs_game_over_to_the_title() {
    let tmp_save = temp_save_path("test_game_over");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    runner.lives = 0;
    runner.level_tally = 0x12;
    runner.money_bag = 3;
    let SessionState::Overworld(map) = &mut runner.state else {
        panic!("map")
    };
    // Golden coins 1 and 2 (`sGoldenCoin1`/`sGoldenCoin2`).
    map.level_progress[5] = 0x80;
    map.level_progress[9] = 0x80;
    let outcome = die_walking_right(&mut runner);
    assert_eq!(outcome, StepOutcome::GameOver);
    let SessionState::GameOver(screen) = &runner.state else {
        panic!("game over")
    };
    assert_eq!(screen.sim.golden_coin_flags, 0b11_0000);
    assert_eq!(
        (screen.sim.level_progress[5], screen.sim.level_progress[9]),
        (0, 0)
    );
    assert_eq!(screen.sim.fade_timer, 255);
    assert!(
        !screen.sim.map_handoff_done,
        "a borrow leaves state $05; the $14 map hand-off never runs"
    );
    assert_eq!(
        (runner.lives, runner.level_tally, runner.money_bag),
        (5, 0x10, 0)
    );
    assert_eq!((runner.coins_low, runner.coins_high), (0, 0));
    assert_eq!(
        runner.session.sound_driver.active_song, 7,
        "$27AF requests music 7"
    );
    let saved = fs::read(&tmp_save).expect("$2934 writes the save");
    assert_eq!(saved, runner.session.slots.image());
    assert_eq!(&saved[..0x50], runner.save_record().as_slice());
    assert_eq!(saved[0x43], 5, "sLives in the save");
    assert_eq!((saved[8 + 5], saved[8 + 9]), (0, 0), "golden coins wiped");

    let first = runner.render_frame();
    assert!(
        first.iter().any(|&p| p != first[0]),
        "the castle screen draws"
    );
    let start = PlayerKeys {
        start: true,
        ..Default::default()
    };
    // START is ignored while the fade runs; release it between presses.
    for frame in 0..254 {
        let keys = if frame % 2 == 0 {
            start
        } else {
            PlayerKeys::default()
        };
        runner.step_frame(keys);
        assert!(
            matches!(runner.state, SessionState::GameOver(_)),
            "frame {frame}"
        );
    }
    // The fade reaches 0 on its 255th frame, and START there hands off.
    runner.step_frame(start);
    assert!(matches!(runner.state, SessionState::Title(_)));
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
fn live_audio_keeps_a_separate_pcm_interval_for_each_catch_up_tick() {
    let mut clock = AudioClock::new(44_100);
    let samples: Vec<_> = (0..8).map(|_| clock.take()).collect();
    assert_eq!(
        samples,
        vec![735; 8],
        "eight ticks retain eight audio intervals"
    );
    let mut clock = AudioClock::new(44_100);
    assert_eq!((0..60).map(|_| clock.take()).sum::<usize>(), 44_100);
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

    // Pressing A queues the entry: the map stays on screen and its theme stops
    // ($3C1A2 requests $FF before the load), but the level is not live yet.
    let _ = runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert!(matches!(runner.state, SessionState::Overworld(_)));
    assert!(runner.pending_level.is_some(), "entry queued");
    assert_eq!(runner.sound_driver.active_song, 0, "map theme stopped");

    // `$3E9FB` waits before each palette write. By frame 48 every layer
    // must be white, including OBJ palette 1 and the status bar.
    let mut entered = None;
    for i in 1..=80 {
        let outcome = runner.step_frame(PlayerKeys::default());
        if (48..73).contains(&i) {
            assert!(runner.render_frame().iter().all(|&p| p == 255), "frame {i}");
        }
        if let StepOutcome::LevelEntered(id) = outcome {
            entered = Some((i, id));
            break;
        }
    }
    // `$3E9FB` plus the `$0B`/`$02`/`$03` tail: the level's first frame is the
    // 73rd after the press.
    assert_eq!(entered, Some((73, 0)));
    assert!(matches!(runner.state, SessionState::InLevel(_)));
    let _ = fs::remove_file(&tmp_save);
}

/// `$3573` with `sEasyMode`: the "EASY MODE" card (`$35CD`) holds for its 64
/// frames between the map fade and `$0B`, so the level goes live 64 frames
/// later, with the object engine in Easy mode.
#[test]
fn easy_mode_entry_shows_the_card_before_the_level() {
    let tmp_save = temp_save_path("test_easy_card");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    runner.easy_mode = true;
    let _ = runner.step_frame(PlayerKeys::default());
    let _ = runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    let card = {
        let font = runner
            .assets
            .ending_scene
            .as_ref()
            .and_then(|a| a.transfer_source(0x1A, 0x4656, 0x300))
            .expect("the card font");
        sml2_core::easy_mode_card::render_easy_card_output(
            font,
            &runner.assets.sprite_metas,
            (160, 144),
            sml2_core::video::OutputMode::Grayscale,
        )
    };
    assert!(card.iter().any(|&p| p != 255), "the card has ink");
    let mut entered = None;
    for i in 1..=150 {
        let outcome = runner.step_frame(PlayerKeys::default());
        let frame = runner.render_frame();
        assert_eq!(frame == card, (50..114).contains(&i), "frame {i}");
        if let StepOutcome::LevelEntered(id) = outcome {
            entered = Some((i, id));
            break;
        }
    }
    assert_eq!(entered, Some((73 + 64, 0)));
    let SessionState::InLevel(ctx) = &runner.state else {
        panic!("level")
    };
    assert!(ctx.obj.easy_mode);
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
        assert!(
            crate::session::player_behind_bg(travelling),
            "{travelling:#04x}"
        );
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
    assert_eq!(
        later.phase,
        first.phase.wrapping_add(8),
        "one call per frame"
    );
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

/// `$3C1EC`: entering a level posts `sSfxRequest0 = 22` *after* `$3C1A2`'s
/// `$2AAA` already ran `_UpdateSound`. Posting it before the same frame's
/// stop-all would let the music reset clear it unplayed; the console hears it
/// on the next frame, so the request must still be pending after that update.
#[test]
fn map_level_entry_posts_the_start_sfx_after_the_music_stop() {
    let tmp_save = temp_save_path("test_map_entry_sfx");
    let _ = fs::remove_file(&tmp_save);
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save, 1, (160, 144), true);
    runner.step_frame(PlayerKeys::default());
    assert_eq!(runner.sound_driver.active_song, 6, "hub theme");

    runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert_eq!(runner.sound_driver.active_song, 0, "the map theme stops");
    assert_eq!(
        runner.sound_driver.sfx_request_0, 22,
        "$3C1EC posts the entry sound after the update"
    );

    runner.step_frame(PlayerKeys::default());
    assert_eq!(runner.sound_driver.sfx_request_0, 0, "consumed next update");
    assert_eq!(
        runner.sound_driver.pulse_effect.active_effect, 22,
        "slot 0 starts SFX 22"
    );
}

#[test]
fn the_coin_room_draws_the_item_preview() {
    let tmp_save = temp_save_path("test_coin_preview");
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    runner.session.begin_coin_room(9);
    for _ in 0..40 {
        let _ = runner.step_frame(PlayerKeys::default());
    }
    let frame = runner.render_frame();
    let tile = match &mut runner.session.state {
        SessionState::Overworld(map) => match &mut map.room {
            Some(room) => {
                let tile = room.oam[2];
                // Any id draws, so move Mario off-screen instead.
                room.oam[1] = 0xC0;
                tile
            }
            None => panic!("the coin room is live"),
        },
        _ => panic!("the coin room is live"),
    };
    assert_eq!(tile, 0x20, "$60C7D ends the walk-in on $4C4D[0]");
    let bare = runner.render_frame();
    let diffs: Vec<(usize, usize)> = frame
        .iter()
        .zip(&bare)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| (i % 160, i / 160))
        .collect();
    assert!(!diffs.is_empty(), "the preview draws");
    assert!(
        diffs
            .iter()
            .all(|&(x, y)| (24..40).contains(&x) && (108..128).contains(&y)),
        "the walk-in ends with Mario standing at OAM x=40, y=124 (the ROM's $FFC4)"
    );
    let _ = std::fs::remove_file(&tmp_save);
}

#[test]
fn the_reveal_draws_the_walk_in() {
    let tmp_save = temp_save_path("test_reveal_walk");
    let assets = SessionAssets::load().expect("session assets must load");
    let mut runner = SessionRunner::new(assets, tmp_save.clone(), 1, (160, 144), true);
    let mut seq = sml2_core::castle_sequence::CastleSequence::default();
    seq.walk_x = 40;
    seq.walk_y = 96;
    seq.state = sml2_core::castle_sequence::STATE_REVEAL_B;
    runner.session.begin_castle_sequence(seq, 8, 0, 0);
    let drawn = runner.render_frame();
    // `$30777[0] = $BB`: tile $0D row 2 is $0F/$08. OBP $D0
    // makes its four opaque pixels black/white/white/white; the right cell
    // mirrors them. A coin sprite also changes pixels but fails this row.
    assert_eq!(
        &drawn[74 * 160 + 28..74 * 160 + 36],
        &[0, 255, 255, 255, 255, 255, 255, 0]
    );
    let mut bare_seq = sml2_core::castle_sequence::CastleSequence::default();
    bare_seq.state = sml2_core::castle_sequence::STATE_REVEAL_B;
    bare_seq.walk_x = 0; // OAM x 0 puts the sprite outside the viewport
    runner.session.begin_castle_sequence(bare_seq, 8, 0, 0);
    let bare = runner.render_frame();
    let diffs: Vec<(usize, usize)> = drawn
        .iter()
        .zip(&bare)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| (i % 160, i / 160))
        .collect();
    assert!(!diffs.is_empty(), "Mario is drawn");
    assert!(
        diffs
            .iter()
            .all(|&(x, y)| (24..=40).contains(&x) && (72..88).contains(&y)),
        "Mario's 2x2 sprite sits at (walk_x, walk_y)"
    );
    // `$30777`: native sequence state supplies the next walking pose.
    let mut slow = sml2_core::castle_sequence::CastleSequence::default();
    slow.walk_x = 40;
    slow.walk_y = 96;
    slow.state = sml2_core::castle_sequence::STATE_REVEAL_B;
    runner.session.begin_castle_sequence(slow, 8, 0, 0);
    if let SessionState::Castle(seq) = &mut runner.session.state {
        seq.sprite_id = 0xBC;
    }
    let other = runner.render_frame();
    assert_ne!(drawn, other, "the walk animation changes Mario's tile");
    let _ = std::fs::remove_file(&tmp_save);
}

/// A death runs the same `$FF9B = $14` hand-off as a clear, so its zone band
/// reloads the screen Mario died from. PyBoy: a time-out in the Tree Zone
/// level from node `$23` lands on screen 2, node `$23`, cursor 48/110.
#[test]
fn dying_in_a_zone_level_returns_to_that_zone() {
    let mut session =
        crate::session::Session::new(SessionAssets::load().expect("session assets must load"));
    session.debug_warp_map(2, 0x23).expect("tree zone");
    let tick = |session: &mut crate::session::Session, keys: PlayerKeys| {
        session
            .advance(crate::session::FrameInput { keys, div: 0 })
            .outcome
    };
    for frame in 0..400 {
        let a = frame % 20 < 2;
        tick(
            &mut session,
            PlayerKeys {
                a,
                ..PlayerKeys::default()
            },
        );
        if matches!(session.state, SessionState::InLevel(_)) {
            break;
        }
    }
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("node $23 enters its level");
    };
    ctx.sim.dead = 0xFF;
    ctx.sim.dead_fade = true;
    ctx.sim.fade_timer = 3;
    let mut exit = None;
    for _ in 0..300 {
        if let StepOutcome::LevelExited {
            node,
            screen,
            cleared,
        } = tick(&mut session, PlayerKeys::default())
        {
            exit = Some((node, screen, cleared));
            break;
        }
    }
    assert_eq!(exit, Some((0x23, 2, false)));
    let SessionState::Overworld(map) = &session.state else {
        panic!("the death lands on the map");
    };
    assert_eq!(
        (map.screen_id, map.node, map.cursor_x, map.cursor_y),
        (2, 0x23, 48, 110)
    );
}

/// `$A27C`, `$A284` and the walk timer `$A222` have no writer on the map or in
/// the level load, so a clear of Mario Zone 1 (node `$50`) carries them into
/// level 11.
#[test]
fn flutter_charge_survives_a_level_change() {
    use crate::session::{FrameInput, Session};
    let mut session = Session::new(SessionAssets::load().expect("session assets"));
    session.debug_warp_map(6, 0x50).expect("mario zone");
    let tick = |session: &mut Session, keys: PlayerKeys| {
        session.advance(FrameInput { keys, div: 0 }).outcome
    };
    let mut entered = None;
    tick(
        &mut session,
        PlayerKeys {
            a: true,
            ..PlayerKeys::default()
        },
    );
    for _ in 0..MAP_ENTRY_FRAMES + 8 {
        if let StepOutcome::LevelEntered(id) = tick(&mut session, PlayerKeys::default()) {
            entered = Some(id);
            break;
        }
    }
    assert_eq!(entered, Some(10));
    {
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("node $50 enters level 10");
        };
        ctx.sim.flutter_armed = true;
        ctx.sim.flutter_charge = 1;
        ctx.sim.walk_timer = 7;
        ctx.sim.exit_phase = true;
        ctx.sim.exit_completed = true;
        ctx.sim.fade_timer = 0;
        ctx.sim.dead = 0;
    }
    let mut exited = false;
    for _ in 0..8 {
        if let StepOutcome::LevelExited { cleared: true, .. } =
            tick(&mut session, PlayerKeys::default())
        {
            exited = true;
            break;
        }
    }
    assert!(exited, "the clear returns to the map");
    let mut walked = false;
    for _ in 0..400 {
        tick(&mut session, PlayerKeys::default());
        let SessionState::Overworld(map) = &session.state else {
            continue;
        };
        if map.cursor_walking != 0 {
            walked = true;
        }
        if walked && map.cursor_walking == 0 {
            break;
        }
    }
    assert!(walked, "clearing node $50 walks toward $51");
    let mut entered = None;
    tick(
        &mut session,
        PlayerKeys {
            a: true,
            ..PlayerKeys::default()
        },
    );
    for _ in 0..MAP_ENTRY_FRAMES + 8 {
        if let StepOutcome::LevelEntered(id) = tick(&mut session, PlayerKeys::default()) {
            entered = Some(id);
            break;
        }
    }
    assert_eq!(entered, Some(11));
    let SessionState::InLevel(ctx) = &session.state else {
        panic!("the next node enters level 11");
    };
    assert!(ctx.sim.flutter_armed, "sFlutterArmed survives the map");
    assert_eq!(ctx.sim.flutter_charge, 1, "sFlutterCharge survives the map");
    assert_eq!(
        ctx.sim.walk_timer, 7,
        "sMarioWalkAnimTimer survives the map"
    );
}

/// The reveal's coin walk (`$4485`) steps `$A222` by 8, and nothing on the
/// map or in the level load writes it again: the next level's walk cycle
/// starts from where the reveal left it.
#[test]
fn the_reveal_walk_timer_reaches_the_next_level() {
    use crate::session::{FrameInput, Session};
    let mut session = Session::new(SessionAssets::load().expect("session assets"));
    let mut seq = sml2_core::castle_sequence::CastleSequence::default();
    seq.enter(&[0x80, 0x80, 0, 0x80, 0x80, 0x80]);
    seq.level = 23;
    seq.state = sml2_core::castle_sequence::STATE_REVEAL_D;
    session.begin_castle_sequence(seq, 6, 0x50, 0);
    let tick = |session: &mut Session, keys: PlayerKeys| {
        session.advance(FrameInput { keys, div: 0 }).outcome
    };
    let mut walk_anim = 0;
    for _ in 0..4000 {
        if let SessionState::Castle(seq) = &session.state {
            walk_anim = seq.walk_anim;
        }
        tick(&mut session, PlayerKeys::default());
        if matches!(session.state, SessionState::Overworld(_)) {
            break;
        }
    }
    assert!(matches!(session.state, SessionState::Overworld(_)));
    assert_ne!(walk_anim, 0, "the coin walk stepped the timer");
    // Press A every other frame until the map accepts it and the level loads.
    let mut entered = None;
    for frame in 0..600 {
        let keys = PlayerKeys {
            a: frame % 2 == 0,
            ..PlayerKeys::default()
        };
        if let StepOutcome::LevelEntered(id) = tick(&mut session, keys) {
            entered = Some(id);
            break;
        }
    }
    assert!(entered.is_some(), "node $50 enters its level");
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!(ctx.sim.walk_timer, walk_anim);
}

/// `$022A` clears HRAM only at startup; `$EBBB` resets scratch on each load.
#[test]
fn object_clock_live_load_inherits_counter_and_resets_phase() {
    use crate::session::{FrameInput, Session};
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(10, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    assert_eq!(
        (ctx.obj.fff0, ctx.obj.phase, ctx.obj.update_phase),
        (0, false, 0)
    );
    ctx.obj.fff0 = 37;
    ctx.obj.phase = true;
    ctx.obj.update_phase = 1;
    session.begin_level(10, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    assert_eq!(
        (
            ctx.obj.fff0,
            ctx.obj.phase,
            ctx.obj.update_phase,
            ctx.obj.odd
        ),
        (37, false, 0, true)
    );
    // A normal Session caller advances the source's first and second passes.
    session.advance(FrameInput {
        keys: PlayerKeys::default(),
        div: 0,
    });
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (38, 1));
    session.advance(FrameInput {
        keys: PlayerKeys::default(),
        div: 0,
    });
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (38, 0));
    session.enter_title();
    session.begin_level(10, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (38, 0));
    ctx.obj.fff0 = 255;
    ctx.obj.phase = true;
    ctx.obj.reset_level_start();
    assert_eq!(
        (ctx.obj.fff0, ctx.obj.phase, ctx.obj.update_phase),
        (255, false, 0)
    );
    session.advance(FrameInput {
        keys: PlayerKeys::default(),
        div: 0,
    });
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!(
        (ctx.obj.fff0, ctx.obj.update_phase, ctx.obj.odd),
        (0, 1, false)
    );
}

/// `$2D7D` produces standing contact before `$8121` toggles the slot phase.
/// `$5800F..$5802B` admits odd slots on phase 1; `$5A236`/`$3CC0` carry on even FFF0.
#[test]
fn object_clock_first_live_pass_updates_odd_slot_conveyor_and_player_bytes() {
    use crate::session::Session;
    use sml2_core::actor::{Actor, ContactKind, Hitbox, Position, ScreenPosition};
    use sml2_core::level::LevelLayout;
    use sml2_core::objects::MarioContact;
    for (inherited, expected_x) in [(37, 101), (38, 100), (255, 101)] {
        let mut session = Session::new(SessionAssets::load().unwrap());
        session.begin_level(10, 0).unwrap();
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!()
        };
        ctx.obj.fff0 = inherited;
        session.begin_level(10, 0).unwrap();
        let tables = session.assets.object_tables.clone();
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!()
        };
        let mut actor = Actor::default();
        actor.activity = 2;
        actor.definition = 62;
        actor.state = 1;
        actor.contact_kind = ContactKind::from_raw(32);
        // `$C090`: x+8-(camera_x-80), y+16-(camera_y-72).
        actor.position = Position { x: 112, y: 130 };
        actor.screen = ScreenPosition { x: 100, y: 118 };
        actor.hitbox = Hitbox {
            top: 0,
            bottom: 16,
            left: 0,
            right: 16,
        };
        ctx.obj.slots[5].b = actor.to_raw_slot();
        let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
        let contact = MarioContact {
            x: 100,
            y: 100,
            hvel: 0x80,
            ..Default::default()
        };
        ctx.obj.step(100, 100, &layout, &tables, &contact, false);
        assert_eq!(
            (ctx.obj.fff0, ctx.obj.update_phase),
            (inherited.wrapping_add(1), 1)
        );
        assert!(ctx.obj.on_object);
        ctx.sim.x = 100;
        ctx.sim.y = 100;
        ctx.sim.apply_object_result(&mut ctx.obj, false);
        assert_eq!(ctx.sim.x.to_le_bytes(), [expected_x, 0]);
        assert_eq!(ctx.sim.y.to_le_bytes(), [100, 0]);
        assert!(ctx.sim.on_ground_obj);
    }
}

/// `$3F03D` runs `$3EA9A` once the hub is loaded: the map is held while BGP
/// steps `$00`, `$00`, `$40`, `$90` three frames apiece, then `$E4`. Measured
/// in recording 3 at its row 11382: `$40` at +6, `$90` at +9, `$E4` at +12.
#[test]
fn a_level_return_fades_the_hub_in() {
    use crate::session::{FrameInput, Session};
    for cleared in [false, true] {
        let mut session = Session::new(SessionAssets::load().unwrap());
        session.begin_level(0, 0).unwrap();
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!()
        };
        if cleared {
            ctx.sim.exit_phase = true;
            ctx.sim.exit_completed = true;
            ctx.sim.fade_timer = 0;
        } else {
            ctx.sim.dead = 0xFF;
            ctx.sim.dead_fade = true;
            ctx.sim.fade_timer = 1;
        }
        let mut exited = None;
        for _ in 0..100 {
            if let StepOutcome::LevelExited { screen, .. } =
                session.advance(FrameInput::default()).outcome
            {
                exited = Some(screen);
                break;
            }
        }
        assert_eq!(
            exited,
            Some(0),
            "level 0 returns to the hub (cleared {cleared})"
        );
        let mut steps = Vec::new();
        while let Some(pan) = &session.map_pan_return {
            steps.push(pan.palettes().0);
            session.advance(FrameInput::default());
        }
        assert_eq!(
            steps,
            [0, 0, 0, 0, 0, 0, 0x40, 0x40, 0x40, 0x90, 0x90, 0x90],
            "cleared {cleared}"
        );
    }
}

/// `sCastleRevealProgress` (`$A7A5`) sits inside the metatile table, so the
/// map inherits whatever the level left there, including a boss scene's
/// tileset reload (recording 3: level 13's arena leaves `$23`, not the
/// level's own `$FF`).
#[test]
fn the_map_inherits_the_live_metatile_bytes() {
    use crate::session::{FrameInput, Session};
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    ctx.manifest.metatiles.metatiles[0x1A5 / 4].tiles[0x1A5 % 4] = 0x23;
    ctx.sim.dead = 0xFF;
    ctx.sim.dead_fade = true;
    ctx.sim.fade_timer = 1;
    for _ in 0..100 {
        if matches!(
            session.advance(FrameInput::default()).outcome,
            StepOutcome::LevelExited { .. }
        ) {
            break;
        }
    }
    while session.map_pan_return.is_some() {
        session.advance(FrameInput::default());
    }
    session.advance(FrameInput::default());
    let SessionState::Overworld(map) = &session.state else {
        panic!("the death returns to the map")
    };
    assert_eq!(map.reveal_progress, 0x23);
}

#[test]
fn camera_scroll_enables_survive_death_and_map_reentry() {
    use crate::session::{FrameInput, Session};
    let enable_bytes = |camera: &sml2_core::camera::CameraSim| {
        [
            u8::from(camera.right_enabled) * 255,
            u8::from(camera.left_enabled) * 255,
        ]
    };
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    assert_eq!(enable_bytes(&ctx.cam), [0, 0]);
    // `$54A7` can set either enable without clearing the other. `$0F2A`
    // resets the screen-position latch, but does not write these bytes.
    ctx.cam.right_enabled = true;
    ctx.cam.left_enabled = true;
    ctx.sim.dead = 0xFF;
    ctx.sim.dead_fade = true;
    ctx.sim.fade_timer = 1;
    let mut exited = false;
    for _ in 0..100 {
        if matches!(
            session.advance(FrameInput::default()).outcome,
            StepOutcome::LevelExited { .. }
        ) {
            exited = true;
            break;
        }
    }
    assert!(exited);
    while session.map_pan_return.is_some() {
        session.advance(FrameInput::default());
    }
    session.advance(FrameInput {
        keys: PlayerKeys {
            a: true,
            ..Default::default()
        },
        div: 0,
    });
    for _ in 0..MAP_ENTRY_FRAMES + 8 {
        if matches!(
            session.advance(FrameInput::default()).outcome,
            StepOutcome::LevelEntered(_)
        ) {
            break;
        }
    }
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("map reentry")
    };
    assert_eq!(enable_bytes(&ctx.cam), [255, 255]);
    assert_eq!(ctx.cam.mario_screen_x, 0x40, "only the screen latch resets");
    // `$0B90`: even with Mario left of center, the inherited right enable
    // consumes his delta immediately and clears the opposite enable.
    ctx.cam.x = 0x071F;
    ctx.cam.y = 0x008E;
    ctx.cam.edge_now = Some(0);
    ctx.sim.x = 0x0710;
    ctx.sim.y = 0x0084;
    ctx.sim.x_sub = 1;
    ctx.cam.step(&mut ctx.sim);
    assert_eq!(ctx.cam.x.to_le_bytes(), [0x21, 7]);
    assert_eq!(enable_bytes(&ctx.cam), [255, 0]);

    // Demo loaders and direct replacement share the same persistent bytes.
    assert!(session.start_demo(0));
    let SessionState::Demo(demo) = &mut session.state else {
        panic!()
    };
    assert!(demo.ctx.cam.right_enabled);
    demo.ctx.cam.right_enabled = false;
    demo.ctx.cam.left_enabled = true;
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!(enable_bytes(&ctx.cam), [0, 255]);
}

#[test]
fn object_clock_survives_death_map_entry_and_demo_replacement() {
    use crate::session::{FrameInput, Session};
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    ctx.obj.fff0 = 73;
    ctx.sim.dead = 0xFF;
    ctx.sim.dead_fade = true;
    ctx.sim.fade_timer = 1;
    let mut exited = false;
    for _ in 0..100 {
        if matches!(
            session.advance(FrameInput::default()).outcome,
            StepOutcome::LevelExited { .. }
        ) {
            exited = true;
            break;
        }
    }
    assert!(exited);
    // `$3EA9A`'s hub fade-in blocks input until it ends.
    while session.map_pan_return.is_some() {
        session.advance(FrameInput::default());
    }
    session.advance(FrameInput {
        keys: PlayerKeys {
            a: true,
            ..Default::default()
        },
        div: 0,
    });
    for _ in 0..MAP_ENTRY_FRAMES + 8 {
        if matches!(
            session.advance(FrameInput::default()).outcome,
            StepOutcome::LevelEntered(_)
        ) {
            break;
        }
    }
    let SessionState::InLevel(ctx) = &session.state else {
        panic!("expected map reentry")
    };
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (73, 0));
    assert!(session.start_demo(0));
    let SessionState::Demo(demo) = &mut session.state else {
        panic!()
    };
    assert_eq!((demo.ctx.obj.fff0, demo.ctx.obj.phase), (73, false));
    demo.ctx.obj.fff0 = 117;
    session.advance(FrameInput {
        keys: PlayerKeys {
            start: true,
            ..Default::default()
        },
        div: 0,
    });
    assert!(matches!(session.state, SessionState::Title(_)));
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (117, 0));
}

#[test]
fn object_clock_boss_driver_counter_survives_next_loader() {
    use crate::session::Session;
    use sml2_core::objects::MarioContact;
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let tables = session.assets.object_tables.clone();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    ctx.obj.fff0 = 255;
    ctx.obj.spawn_boss(0);
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (255, 0));
    // `$8DA9` increments every pass, independently of the scratch phase.
    let contact = MarioContact {
        travelling: true,
        ..Default::default()
    };
    ctx.obj.step(
        ctx.cam.x,
        ctx.cam.y,
        &ctx.layout_live,
        &tables,
        &contact,
        true,
    );
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (0, 0));
    ctx.obj.step(
        ctx.cam.x,
        ctx.cam.y,
        &ctx.layout_live,
        &tables,
        &contact,
        true,
    );
    assert_eq!(ctx.obj.fff0, 1);
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!(
        (ctx.obj.fff0, ctx.obj.phase, ctx.obj.update_phase),
        (1, false, 0)
    );
}

#[test]
fn object_clock_exit_retains_the_final_engine_pass() {
    use crate::session::{FrameInput, Session};
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    ctx.obj.fff0 = 72;
    // A completed handoff is consumed after this tick's engine pass. Retain
    // that pass's result rather than the snapshot from the frame's start.
    ctx.sim.map_handoff_done = true;
    assert!(matches!(
        session.advance(FrameInput::default()).outcome,
        StepOutcome::LevelExited { .. }
    ));
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (73, 0));
}

/// `sMoneyBagCount` (`$A28D`), the HUD's enemy counter, is written only by
/// new game, save load, game over and the defeat credit. A level exit keeps
/// it for the save, and the next level load does not clear it.
#[test]
fn the_enemy_counter_carries_across_levels_and_into_the_save() {
    use crate::session::{FrameInput, Session};
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    ctx.obj.money_bag = 9;
    ctx.sim.map_handoff_done = true;
    assert!(matches!(
        session.advance(FrameInput::default()).outcome,
        StepOutcome::LevelExited { .. }
    ));
    assert_eq!(session.money_bag, 9);
    assert_eq!(session.save_record()[0x44], 9);
    session.begin_level(1, 0).unwrap();
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!(ctx.obj.money_bag, 9);
}

/// Boss scripts reload the tileset with LCD disabled and leave all palettes
/// zero. `$0EF3` restores the header palettes only after the door scroll ends.
#[test]
fn boss_entry_hides_the_tileset_reload_until_the_scroll_finishes() {
    use crate::session::{FrameInput, Session};
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(13, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    ctx.cam.scene_id = 14; // `$65 $20 $30 $60 $68 $40 $03 $FF`
    ctx.cam.transition = 1;
    ctx.cam.x = 0x6B0;
    ctx.scene_vm.start();
    let mut white_frames = 0;
    let mut restored = false;
    for _ in 0..80 {
        session.advance(FrameInput::default());
        let SessionState::InLevel(ctx) = &session.state else {
            panic!()
        };
        if ctx.scene_reload_blank {
            white_frames += 1;
            assert_eq!(
                (
                    ctx.spr.as_ref().unwrap().obp0,
                    ctx.spr.as_ref().unwrap().obp1
                ),
                (0, 0)
            );
            assert!(session.render_frame((160, 144)).iter().all(|&p| p == 255));
        } else if white_frames != 0 {
            assert_eq!(ctx.cam.transition, 0);
            assert_eq!(
                (
                    ctx.spr.as_ref().unwrap().obp0,
                    ctx.spr.as_ref().unwrap().obp1
                ),
                (
                    ctx.manifest.header.palettes[1],
                    ctx.manifest.header.palettes[2]
                )
            );
            assert!(session.render_frame((160, 144)).iter().any(|&p| p != 255));
            restored = true;
            break;
        }
    }
    assert!(white_frames > 20, "reload and door scroll stay hidden");
    assert!(restored, "the arena becomes visible when the scroll ends");
}

/// `$0669` only changes `$A299`; `$3042D` only clears it on map return.
/// The clock bytes from the preceding level must reach Pumpkin Zone 1.
#[test]
fn pumpkin_tile_animation_inherits_phase_and_map_time() {
    use crate::session::{FrameInput, Session};
    use sml2_core::tile_animation::TileAnimationClock;

    let mut session = Session::new(SessionAssets::load().expect("session assets"));
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        unreachable!()
    };
    ctx.tile_animation = TileAnimationClock {
        timer: 20,
        phase: 2,
    };

    session.debug_warp_map(1, 0x28).unwrap();
    let tick = |session: &mut Session, keys| session.advance(FrameInput { keys, div: 0 }).outcome;
    // The disabled map clock advances only on frames that run `$0300`.
    for _ in 0..10 {
        tick(&mut session, PlayerKeys::default());
    }
    let entry_frame = session.frame_n;
    tick(
        &mut session,
        PlayerKeys {
            a: true,
            ..Default::default()
        },
    );
    let mut entered = false;
    for _ in 0..MAP_ENTRY_FRAMES + 8 {
        if tick(&mut session, PlayerKeys::default()) == StepOutcome::LevelEntered(6) {
            entered = true;
            break;
        }
    }
    assert!(entered, "Pumpkin node $28 enters level 6");
    let SessionState::InLevel(ctx) = &mut session.state else {
        unreachable!()
    };
    // Ten map frames, the A frame and the entry's four `$0B`/`$02`/`$03`
    // frames tick; `$3E9FB`'s fade and the `$03` loader do not.
    assert_eq!(
        session.frame_n - entry_frame,
        1 + u64::from(MAP_ENTRY_FRAMES)
    );
    assert_eq!(
        (ctx.tile_animation.timer, ctx.tile_animation.phase),
        (20 + 10 + 1 + 4, 2)
    );
    assert_eq!(ctx.sim.tile_anim_config, 0xFF);
    assert!(ctx.tile_animation.timer >= 30);
    let expected = ctx.anim_rows[0xEC0..0xF00].to_vec();
    ctx.sim.paused = true;
    tick(&mut session, PlayerKeys::default());
    let SessionState::InLevel(ctx) = &session.state else {
        unreachable!()
    };
    // `$02FF` increments, then this frame's `$3873` copies and restarts at 0.
    assert_eq!((ctx.tile_animation.timer, ctx.tile_animation.phase), (0, 3));
    assert_eq!(&ctx.bg_live[0xA00..0xA40], expected);

    // A direct warp/reload also preserves both bytes, rather than starting
    // phase 0 and waiting another full 30 frames.
    session.begin_level(6, 0).unwrap();
    let SessionState::InLevel(ctx) = &session.state else {
        unreachable!()
    };
    assert_eq!((ctx.tile_animation.timer, ctx.tile_animation.phase), (0, 3));
}

/// `$05D5` clears the joypad while `sFadeTimer` runs, but `$5E12` writes
/// its scripted Right+B after that clear: once Wario's exit sets phase 3,
/// Mario walks even though the fade still holds the player's own keys.
#[test]
fn the_scripted_exit_walks_through_the_fade_input_lock() {
    let mut session = crate::session::Session::new(SessionAssets::load().unwrap());
    session.begin_level(24, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("Wario level")
    };
    ctx.cam.boss_fight_phase = 3;
    ctx.sim.fade_timer = 5;
    ctx.sim.x = 0x0950;
    ctx.sim.hvel = 0x80;
    session.advance(crate::session::FrameInput::default());
    let SessionState::InLevel(ctx) = &session.state else {
        panic!("Wario level")
    };
    assert!(ctx.sim.fade_timer > 0, "the fade lock was still active");
    assert!(
        ctx.sim.dir_right && ctx.sim.hvel > 0x80,
        "hvel {:02x}",
        ctx.sim.hvel
    );
}

/// State 19 draws the ending's own scene in all 20 substates:
/// the session frame is the `EndingScene` projection, never the level. The
/// Wario level is entered with the boss already beaten and Mario past the
/// `$5E12` trigger; the entry fade and sequence then run unaided.
#[test]
fn the_session_draws_the_ending_scene_through_all_twenty_substates() {
    use sml2_core::ending_entry::EndingEntryState;
    use sml2_core::video::OutputMode;
    let mut session = crate::session::Session::new(SessionAssets::load().unwrap());
    session.begin_level(24, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("Wario level")
    };
    ctx.cam.boss_fight_phase = 3;
    ctx.sim.x = 0xC0;
    ctx.sim.y = 0xE8;
    let level_frame = session.render_frame((160, 144));
    let mut seen = [false; 20];
    let mut frames = 0u32;
    for _ in 0..40_000 {
        session.advance(crate::session::FrameInput::default());
        let SessionState::InLevel(ctx) = &session.state else {
            panic!("the ending stays in the level context")
        };
        if ctx.ending_entry.state() != EndingEntryState::Sequence {
            continue;
        }
        let substate = ctx.ending_sequence.substate;
        // Compare each substate's first frame, then every 256th.
        let first = !std::mem::replace(&mut seen[usize::from(substate)], true);
        frames += 1;
        if !first && frames % 256 != 0 {
            continue;
        }
        assert_eq!(
            session.render_frame((160, 144)),
            sml2_core::ending_render::render_ending_output(
                &ctx.ending_scene,
                &ctx.ending_sequence,
                (160, 144),
                OutputMode::Grayscale
            ),
            "substate {substate}"
        );
        if substate == 19 {
            break;
        }
    }
    assert_eq!(seen, [true; 20], "every substate was drawn");
    assert_ne!(session.render_frame((160, 144)), level_frame);
}
