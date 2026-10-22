use super::*;
use crate::front_end::keys::{A, DOWN, LEFT, RIGHT, SELECT, START};
use crate::front_end::save_slots::{fresh_template, SaveSlots, SLOT_LEN};
use crate::map::SAVE_SIGNATURE;

fn valid_record(node: u8) -> [u8; SLOT_LEN] {
    let mut record = [0u8; SLOT_LEN];
    record[0] = node;
    record[0x08] = 0x80;
    record[0x40] = 3;
    record[0x41] = 1;
    record[0x42] = 0x12;
    record[0x43] = 5;
    record[0x44] = 2;
    record[0x45] = record[..0x45]
        .iter()
        .fold(0u8, |c, b| c.wrapping_add(*b));
    record[0x46..0x4A].copy_from_slice(&SAVE_SIGNATURE);
    record
}

/// Run the `$43D6` drop until the slot row is live.
fn drop_in(fs: &mut FileSelect, slots: &mut SaveSlots) -> usize {
    let mut frames = 0;
    while fs.menu_state() != 1 {
        fs.step(slots, 0, 0);
        frames += 1;
        assert!(frames < 100, "the cursor never reached the slot row");
    }
    frames
}

/// Finish the current state's timer and return the last output.
fn settle(fs: &mut FileSelect, slots: &mut SaveSlots) -> FileSelectOutput {
    for _ in 0..200 {
        let out = fs.step(slots, 0, 0);
        if fs.menu_state() == 1 {
            return out;
        }
    }
    panic!("state never returned to the slot row");
}

#[test]
fn the_loader_validates_slots_and_requests_its_sound_and_music() {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    assert_eq!(fs.take_music(), MUSIC);
    assert_eq!(fs.take_music(), 0, "the music request is one-shot");
    for slot in 0..SLOT_COUNT {
        assert_eq!(slots.slot(slot), Some(&fresh_template()));
    }
    let out = fs.step(&mut slots, 0, 0);
    assert_eq!(out.sfx0, 5, "$30000 posts SFX 0 = 5");
    assert_eq!(out.sfx2, 0);
    assert_eq!(out.action, FileSelectAction::None);
}

#[test]
fn the_cursor_drops_into_the_slot_row() {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    let frames = drop_in(&mut fs, &mut slots);
    assert_eq!(fs.cursor(), (CURSOR_START_X, SLOT_ROW_Y));
    assert_eq!(frames, 45, "16 -> 64 by 2, then by 3, then the $7C stop");
    assert_eq!(fs.menu_state(), 1);
}

#[test]
fn drop_thresholds_choose_sprite_and_priority_before_moving() {
    // `$43D6`: test current Y against $30/$7C, then add 2 and test $40.
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    for _ in 0..16 { fs.step(&mut slots, 0, 0); }
    assert_eq!(fs.cursor(), (40, 0x30));
    assert_eq!(fs.cursor_sprite(), 16);
    assert!(fs.cursor_behind_bg());
    fs.step(&mut slots, 0, 0);
    assert_eq!(fs.cursor(), (40, 0x32));
    assert_eq!(fs.cursor_sprite(), 10);
    assert!(!fs.cursor_behind_bg());
    for _ in 17..23 { fs.step(&mut slots, 0, 0); }
    assert_eq!(fs.cursor().1, 0x3E);
    fs.step(&mut slots, 0, 0);
    assert_eq!(fs.cursor().1, 0x41);
    for _ in 24..44 { fs.step(&mut slots, 0, 0); }
    assert_eq!((fs.cursor().1, fs.menu_state()), (0x7D, 0));
    fs.step(&mut slots, 0, 0);
    assert_eq!((fs.cursor().1, fs.menu_state()), (0x7C, 1));
    assert_eq!(fs.cursor_sprite(), 10);
    assert!(!fs.cursor_behind_bg());
}

#[test]
fn right_and_left_move_one_slot_over_32_frames() {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);

    fs.step(&mut slots, RIGHT, 0);
    assert_eq!(fs.menu_state(), 2, "$4398");
    assert_eq!(fs.timer(), 32);
    settle(&mut fs, &mut slots);
    assert_eq!(fs.cursor().0, 0x28 + 32, "one slot right");
    assert_eq!(fs.menu_state(), 1);

    fs.step(&mut slots, LEFT, 0);
    assert_eq!(fs.menu_state(), 3, "$43B7");
    settle(&mut fs, &mut slots);
    assert_eq!(fs.cursor().0, 0x28, "back to the left slot");
}

#[test]
fn select_toggles_easy_mode_but_not_in_erase_mode() {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);

    fs.step(&mut slots, SELECT, SELECT);
    assert!(fs.easy_mode());
    fs.step(&mut slots, SELECT, SELECT);
    assert!(!fs.easy_mode());

    // `$437D` returns early while erase mode is armed.
    enter_erase_mode(&mut fs, &mut slots);
    assert!(fs.clear_mode());
    fs.step(&mut slots, SELECT, SELECT);
    assert!(!fs.easy_mode(), "erase mode suppresses the Easy toggle");
}

/// `$302AD`'s tail: arming erase mode also writes `sEasyMode = 0`.
#[test]
fn arming_erase_mode_clears_easy_mode() {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);
    fs.step(&mut slots, SELECT, SELECT);
    assert!(fs.easy_mode());
    enter_erase_mode(&mut fs, &mut slots);
    assert!(fs.clear_mode());
    assert!(!fs.easy_mode());
}

#[test]
fn start_or_down_confirms_the_selected_slot() {
    let mut slots = SaveSlots::new();
    let record = valid_record(0x23);
    assert!(slots.write_slot(1, &record));
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);
    fs.step(&mut slots, RIGHT, 0);
    settle(&mut fs, &mut slots);
    assert_eq!(fs.cursor().0, 0x48, "slot 1");

    let out = fs.step(&mut slots, START, START);
    assert_eq!(out.sfx0, 5, "$4367");
    assert_eq!(fs.menu_state(), 5);
    assert_eq!(fs.timer(), 64);

    let mut last = FileSelectOutput::default();
    for _ in 0..64 {
        last = fs.step(&mut slots, 0, 0);
    }
    assert_eq!(last.action, FileSelectAction::Load { slot: 1, record });
    assert_eq!(fs.save_file_index, 1);

    // DOWN is the other confirm button.
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);
    fs.step(&mut slots, DOWN, DOWN);
    assert_eq!(fs.menu_state(), 5);
}

#[test]
fn confirm_is_blocked_on_the_erase_position() {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);
    move_to_erase_position(&mut fs, &mut slots);
    assert_eq!(fs.cursor().0, 0x88);

    let out = fs.step(&mut slots, START, START);
    assert_eq!(out.action, FileSelectAction::None);
    assert_eq!(fs.menu_state(), 1, "$4367's X >= $80 gate");
}

#[test]
fn a_at_the_erase_position_toggles_erase_mode_and_erases() {
    let mut slots = SaveSlots::new();
    let record = valid_record(0x40);
    assert!(slots.write_slot(2, &record));
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);
    move_to_erase_position(&mut fs, &mut slots);

    let out = fs.step(&mut slots, A, A);
    assert_eq!(out.sfx0, 1, "$4357");
    assert_eq!(fs.menu_state(), 4);
    assert_eq!(fs.timer(), 55);

    // 55 -> 43 is 12 frames; the toggle lands when the timer reaches $2B.
    let mut toggled = None;
    for _ in 0..12 {
        let out = fs.step(&mut slots, 0, 0);
        if out.sfx0 == 8 {
            toggled = Some(out.sfx0);
        }
    }
    assert_eq!(toggled, Some(8), "$4291");
    assert!(fs.clear_mode());
    settle(&mut fs, &mut slots);

    // Back to slot 2 (x = $68) and confirm the erase.
    fs.step(&mut slots, LEFT, 0);
    settle(&mut fs, &mut slots);
    assert_eq!(fs.cursor().0, 0x68);
    fs.step(&mut slots, START, START);
    let mut last = FileSelectOutput::default();
    for _ in 0..64 {
        last = fs.step(&mut slots, 0, 0);
    }
    assert_eq!(last.action, FileSelectAction::Erased { slot: 2 });
    assert_eq!(last.sfx2, 5, "$30269");
    assert_eq!(slots.slot(2), Some(&fresh_template()));
    assert_eq!(fs.menu_state(), 6);
    assert_eq!(fs.timer(), 0x80);

    settle(&mut fs, &mut slots);
    assert_eq!(fs.menu_state(), 1);
    assert!(!fs.clear_mode());
}

#[test]
fn the_erase_mode_tick_requests_sfx2_on_even_frames() {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);
    enter_erase_mode(&mut fs, &mut slots);
    settle(&mut fs, &mut slots);

    let mut ticks = Vec::new();
    for _ in 0..4 {
        ticks.push(fs.step(&mut slots, 0, 0).sfx2);
    }
    assert!(
        ticks.iter().any(|&s| s == 4),
        "the even-frame erase tick must fire: {ticks:?}"
    );
    assert!(
        ticks.iter().any(|&s| s == 0),
        "odd frames are silent: {ticks:?}"
    );
}

#[test]
fn the_cursor_walk_cycle_drives_the_sprite_ids() {
    let mut slots = SaveSlots::new();
    let mut fs = FileSelect::new(&mut slots);
    drop_in(&mut fs, &mut slots);
    fs.step(&mut slots, RIGHT, 0);
    let mut sprites = Vec::new();
    for _ in 0..8 {
        fs.step(&mut slots, 0, 0);
        sprites.push(fs.cursor_sprite());
    }
    assert!(
        sprites.iter().all(|&s| (1..=4).contains(&s)),
        "right walk frames 1..4: {sprites:?}"
    );
    assert!(
        sprites.windows(2).any(|w| w[0] != w[1]),
        "the walk timer animates the sprite"
    );
}

fn move_to_erase_position(fs: &mut FileSelect, slots: &mut SaveSlots) {
    for _ in 0..3 {
        fs.step(slots, RIGHT, 0);
        settle(fs, slots);
    }
}

fn enter_erase_mode(fs: &mut FileSelect, slots: &mut SaveSlots) {
    move_to_erase_position(fs, slots);
    fs.step(slots, A, A);
    for _ in 0..55 {
        fs.step(slots, 0, 0);
        if fs.menu_state() != 4 {
            break;
        }
    }
}
