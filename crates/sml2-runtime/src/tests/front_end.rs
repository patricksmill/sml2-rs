//! Runtime wiring for the ROM boot flow: title -> file select -> overworld.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use sml2_core::front_end::keys::{START, SELECT};
use sml2_core::front_end::save_slots::fresh_template;
use sml2_core::map::save_record_valid;
use sml2_core::player_sim::PlayerKeys;

use crate::session::{Boot, SessionAssets, SessionRunner, SessionState};

fn temp_save_path(name: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("sml2_fe_{name}_{id}_{}.sav", std::process::id()))
}

fn runner_with_boot(name: &str) -> SessionRunner {
    let save = temp_save_path(name);
    let _ = fs::remove_file(&save);
    let assets = SessionAssets::load().expect("session assets must load");
    SessionRunner::new_with_boot(assets, save, 1, (160, 144), true, Boot::Title)
}

fn press(keys: u8) -> PlayerKeys {
    PlayerKeys {
        right: keys & 0x10 != 0,
        left: keys & 0x20 != 0,
        up: keys & 0x40 != 0,
        down: keys & 0x80 != 0,
        a: keys & 0x01 != 0,
        b: keys & 0x02 != 0,
        select: keys & 0x04 != 0,
        start: keys & 0x08 != 0,
    }
}

fn press_once(runner: &mut SessionRunner, keys: u8) {
    runner.step_frame(press(keys));
}

fn finish_file_select_loading(runner: &mut SessionRunner) {
    if !matches!(runner.state, SessionState::FileSelectLoading(_)) { return; }
    for _ in 0..20 {
        runner.step_frame(PlayerKeys::default());
        if !matches!(runner.state, SessionState::FileSelectLoading(_)) { return; }
    }
    panic!("file select loader did not complete");
}

#[test]
fn boot_title_then_start_opens_the_file_select() {
    let mut runner = runner_with_boot("title_to_fs");
    assert!(matches!(runner.state, SessionState::Title(_)));
    assert!(!runner.saveable(), "the title has nothing to persist");

    // `$14043`: START is ignored until the countdown's low byte drops below
    // $40; 64 idle frames is enough.
    for _ in 0..64 {
        runner.step_frame(PlayerKeys::default());
    }
    let title = runner.render_frame();
    press_once(&mut runner, START);
    assert!(matches!(runner.state, SessionState::FileSelectLoading(_)));
    assert_eq!(runner.render_frame(), title, "pending $19 retains title VRAM");
    finish_file_select_loading(&mut runner);
    assert!(matches!(runner.state, SessionState::FileSelect(_)));
    assert_eq!(runner.sound_driver.active_song, 13, "$30000 requests song 13");
}

#[test]
fn the_file_select_loads_a_blank_slot_into_the_overworld() {
    let mut runner = runner_with_boot("fs_load");
    for _ in 0..64 {
        runner.step_frame(PlayerKeys::default());
    }
    press_once(&mut runner, START);
    finish_file_select_loading(&mut runner);
    assert!(matches!(runner.state, SessionState::FileSelect(_)));

    // `$43D6`: 45 frames until the slot row is live; then START confirms.
    for _ in 0..45 {
        runner.step_frame(PlayerKeys::default());
    }
    press_once(&mut runner, START);
    // `$41C6`: the 64-frame drop, then the load body.
    for _ in 0..64 {
        runner.step_frame(PlayerKeys::default());
    }

    match &runner.state {
        SessionState::Overworld(map) => {
            assert_eq!(map.screen_id, 0);
            assert_eq!(map.node, 0);
        }
        _ => panic!("the blank slot must start a new game"),
    }
    assert_eq!(runner.active_slot, Some(0));
    assert_eq!(runner.lives, 5);
    assert!(
        save_record_valid(&runner.save_record()),
        "the synthesized new-game record is valid"
    );
}

#[test]
fn select_marks_easy_mode_without_touching_the_slots() {
    let mut runner = runner_with_boot("fs_easy");
    for _ in 0..64 {
        runner.step_frame(PlayerKeys::default());
    }
    press_once(&mut runner, START);
    finish_file_select_loading(&mut runner);
    for _ in 0..45 {
        runner.step_frame(PlayerKeys::default());
    }
    let before = runner.slots.clone();
    press_once(&mut runner, SELECT);
    match &runner.state {
        SessionState::FileSelect(fs) => assert!(fs.easy_mode()),
        _ => panic!("still in the file select"),
    }
    assert_eq!(runner.slots, before);
}

#[test]
fn a_damaged_slot_start_a_fresh_game() {
    let mut runner = runner_with_boot("fs_damaged");
    // Make every slot hold a template (the loader validates them anyway) and
    // then corrupt the record the file select will read.
    runner.slots.write_slot(0, &fresh_template());
    assert!(!save_record_valid(&fresh_template()));

    for _ in 0..64 {
        runner.step_frame(PlayerKeys::default());
    }
    press_once(&mut runner, START);
    finish_file_select_loading(&mut runner);
    for _ in 0..45 {
        runner.step_frame(PlayerKeys::default());
    }
    press_once(&mut runner, START);
    for _ in 0..64 {
        runner.step_frame(PlayerKeys::default());
    }
    assert!(matches!(runner.state, SessionState::Overworld(_)));
    assert_eq!(runner.active_slot, Some(0));
}

fn demo_position(name: &str, live: PlayerKeys, frames: usize) -> (u16, u16) {
    let mut runner = runner_with_boot(name);
    assert!(runner.start_demo(0), "demo 0 loads");
    for _ in 0..frames {
        runner.step_frame(live);
    }
    match &runner.state {
        SessionState::Demo(demo) => (demo.ctx.sim.x, demo.ctx.sim.y),
        _ => panic!("the demo must still be running"),
    }
}

#[test]
fn the_demo_pages_are_the_rom_slices() {
    let assets = SessionAssets::load().expect("session assets must load");
    assert_eq!(assets.front_end.demo_tapes().len(), 0x400);
    let tape0 = assets.front_end.demo_tape(0);
    assert_eq!(tape0, &assets.front_end.demo_tapes()[0x300..]);
    // Page 0 opens with two skipped pairs, then SELECT+UP for 3 frames.
    assert_eq!(&tape0[..6], &[0x00, 0x00, 0x44, 0x03, 0x40, 0x08]);
}

#[test]
fn the_title_countdown_starts_the_first_attract_demo() {
    let mut runner = runner_with_boot("demo_countdown");
    for _ in 0..1090 {
        runner.step_frame(PlayerKeys::default());
    }
    match &runner.state {
        SessionState::Demo(demo) => {
            assert_eq!(demo.index, 0);
            assert_eq!(demo.level_id, 0);
        }
        _ => panic!("the countdown must start demo 0"),
    }
}

#[test]
fn the_demo_tape_drives_the_player_and_live_input_does_not_leak() {
    let idle = PlayerKeys::default();
    let left = PlayerKeys {
        left: true,
        ..PlayerKeys::default()
    };
    let right = PlayerKeys {
        right: true,
        ..PlayerKeys::default()
    };
    let start = demo_position("demo_start", idle, 0);
    let held_left = demo_position("demo_left", left, 30);
    let held_right = demo_position("demo_right", right, 30);
    assert_eq!(
        held_left, held_right,
        "live input must not change the demo"
    );
    assert_ne!(start, held_left, "the tape must move Mario");
}

#[test]
fn start_ends_the_demo_and_advances_the_title_index() {
    let mut runner = runner_with_boot("demo_start");
    assert!(runner.start_demo(0));
    runner.step_frame(PlayerKeys::default());
    runner.step_frame(PlayerKeys {
        start: true,
        ..PlayerKeys::default()
    });
    match &runner.state {
        SessionState::Title(title) => {
            assert_eq!(title.demo_index(), 1);
            assert_eq!(title.timer(), 1344, "Init resets the attract countdown");
        }
        _ => panic!("START must return to the title"),
    }
}

#[test]
fn the_tape_terminator_returns_to_the_title() {
    let mut runner = runner_with_boot("demo_tape_end");
    assert!(runner.start_demo(0));
    for _ in 0..4000 {
        runner.step_frame(PlayerKeys::default());
        if matches!(runner.state, SessionState::Title(_)) {
            break;
        }
    }
    match &runner.state {
        SessionState::Title(title) => assert_eq!(title.demo_index(), 1),
        _ => panic!("the $FF terminator must return to the title"),
    }
}
