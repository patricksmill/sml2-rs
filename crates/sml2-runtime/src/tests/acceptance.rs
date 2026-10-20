//! FE-8 acceptance tapes: fixed inputs through the live session, byte asserts.

use sml2_core::front_end::save_slots::fresh_template;
use sml2_core::front_end::{SaveSlots, SAVE_IMAGE_LEN, SLOT_LEN};
use sml2_core::map::{save_record_valid, SAVE_SIGNATURE};
use sml2_core::map_sim::MapSim;
use sml2_core::player_sim::PlayerKeys;

use crate::session::{FrameInput, Session, SessionAssets, SessionState};

fn session() -> Session {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    session.enter_title();
    session
}

fn step(session: &mut Session, keys: PlayerKeys) {
    session.advance(FrameInput { keys, div: 0 });
}

fn idle(session: &mut Session, frames: usize) {
    for _ in 0..frames {
        step(session, PlayerKeys::default());
    }
}

fn press(session: &mut Session, keys: PlayerKeys) {
    step(session, keys);
}

fn to_file_select(session: &mut Session) {
    idle(session, 64);
    press(
        session,
        PlayerKeys {
            start: true,
            ..PlayerKeys::default()
        },
    );
    assert!(matches!(session.state, SessionState::FileSelect(_)));
}

fn drop_in(session: &mut Session) {
    idle(session, 45);
    assert_eq!(file_select(session).menu_state(), 1);
}

fn move_right(session: &mut Session) {
    press(
        session,
        PlayerKeys {
            right: true,
            ..PlayerKeys::default()
        },
    );
    idle(session, 32);
}

fn move_left(session: &mut Session) {
    press(
        session,
        PlayerKeys {
            left: true,
            ..PlayerKeys::default()
        },
    );
    idle(session, 32);
}

fn start_confirm(session: &mut Session) {
    press(
        session,
        PlayerKeys {
            start: true,
            ..PlayerKeys::default()
        },
    );
    assert_eq!(file_select(session).menu_state(), 5);
    assert_eq!(file_select(session).timer(), 64);
}

fn file_select(session: &Session) -> &sml2_core::front_end::FileSelect {
    match &session.state {
        SessionState::FileSelect(fs) => fs,
        _ => panic!("expected the file select"),
    }
}

fn overworld(session: &Session) -> &MapSim {
    match &session.state {
        SessionState::Overworld(map) => map,
        _ => panic!("expected the overworld"),
    }
}

fn record(node: u8, coins_low: u8, coins_high: u8, tally: u8, lives: u8, money: u8) -> [u8; SLOT_LEN] {
    let mut record = [0u8; SLOT_LEN];
    record[0] = node;
    record[0x08] = 0x80;
    record[0x40] = coins_low;
    record[0x41] = coins_high;
    record[0x42] = tally;
    record[0x43] = lives;
    record[0x44] = money;
    record[0x45] = record[..0x45]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    record[0x46..0x4A].copy_from_slice(&SAVE_SIGNATURE);
    record
}

fn seed_slot(session: &mut Session, slot: usize, record: &[u8; SLOT_LEN]) {
    let mut slots = SaveSlots::new();
    assert!(slots.write_slot(slot, record));
    session.slots = slots;
}

fn confirm_and_load(session: &mut Session) {
    start_confirm(session);
    idle(session, 64);
}

#[test]
fn acceptance_1_empty_sram_new_game_reaches_the_hub() {
    let mut session = session();
    to_file_select(&mut session);
    drop_in(&mut session);
    confirm_and_load(&mut session);

    let map = overworld(&session);
    assert_eq!(map.screen_id, 0);
    assert_eq!(map.node, 0);
    assert_eq!(session.lives, 5);
    assert_eq!(session.coins_low, 0);
    assert_eq!(session.coins_high, 0);
    assert_eq!(session.level_tally, 0);
    assert_eq!(session.money_bag, 0);
    assert_eq!(session.active_slot, Some(0));

    let record = session.save_record();
    assert_eq!(record[0], 0, "new game starts at node 0");
    assert_eq!(&record[0x01..0x08], &[0u8; 7], "the progress block is empty");
    assert_eq!(record[0x42], 0);
    assert_eq!(record[0x43], 5);
    assert!(save_record_valid(&record));
}

#[test]
fn acceptance_2_continue_uses_pipe_c_and_leaves_the_others_alone() {
    let mut session = session();
    let stored = record(0, 0x12, 0x03, 0x07, 8, 2);
    seed_slot(&mut session, 2, &stored);

    to_file_select(&mut session);
    drop_in(&mut session);
    move_right(&mut session);
    move_right(&mut session);
    confirm_and_load(&mut session);

    let map = overworld(&session);
    assert_eq!(map.screen_id, 0);
    assert_eq!(map.node, 0);
    assert_eq!(session.lives, 8);
    assert_eq!(session.coins_low, 0x12);
    assert_eq!(session.coins_high, 0x03);
    assert_eq!(session.level_tally, 0x07);
    assert_eq!(session.money_bag, 2);
    assert_eq!(session.active_slot, Some(2));
    assert_eq!(session.slots.load_slot(2), Some(stored));
    assert_eq!(session.slots.slot(0), Some(&fresh_template()));
    assert_eq!(session.slots.slot(1), Some(&fresh_template()));
}

#[test]
fn acceptance_3_save_round_trip_reloads_pipe_c() {
    let mut session = session();
    let stored = record(0, 0x12, 0x03, 0x07, 8, 2);
    seed_slot(&mut session, 2, &stored);
    to_file_select(&mut session);
    drop_in(&mut session);
    move_right(&mut session);
    move_right(&mut session);
    confirm_and_load(&mut session);

    // Change the overworld state, then let the map's real save trigger run.
    if let SessionState::Overworld(map) = &mut session.state {
        map.pending_save = true;
        map.coins_low = 0x15;
    }
    session.level_tally = 9;
    step(&mut session, PlayerKeys::default());

    let saved = session
        .slots
        .load_slot(2)
        .expect("pipe C must hold the saved record");
    assert!(save_record_valid(&saved));
    assert_eq!(saved[0x40], 0x15, "the changed coin count");
    assert_eq!(saved[0x42], 9, "the changed tally");

    // Relaunch from the written image: pipe C alone survives.
    let image = session.slots.image();
    let mut relaunch = session_with_slots(&image);
    let valid = relaunch.slots.validate_and_init();
    assert_eq!(valid, [false, false, true]);
    assert_eq!(relaunch.slots.load_slot(2), Some(saved));
    assert_eq!(relaunch.slots.slot(0), Some(&fresh_template()));
    assert_eq!(relaunch.slots.slot(1), Some(&fresh_template()));
}

fn session_with_slots(image: &[u8]) -> Session {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    session.slots = SaveSlots::from_image(image);
    session.enter_title();
    session
}

#[test]
fn acceptance_4_erase_clears_pipe_c_and_plays_the_post_erase_animation() {
    let mut session = session();
    let stored = record(0, 0x12, 0x03, 0x07, 8, 2);
    seed_slot(&mut session, 2, &stored);
    to_file_select(&mut session);
    drop_in(&mut session);
    for _ in 0..3 {
        move_right(&mut session);
    }
    assert_eq!(file_select(&session).cursor().0, 0x88);

    press(
        &mut session,
        PlayerKeys {
            a: true,
            ..PlayerKeys::default()
        },
    );
    assert_eq!(file_select(&session).menu_state(), 4);
    assert_eq!(file_select(&session).timer(), 55);
    idle(&mut session, 12);
    assert!(file_select(&session).clear_mode());
    idle(&mut session, 43);
    assert_eq!(file_select(&session).menu_state(), 1);

    move_left(&mut session);
    assert_eq!(file_select(&session).cursor().0, 0x68);
    start_confirm(&mut session);
    idle(&mut session, 64);

    assert_eq!(session.slots.slot(2), Some(&fresh_template()));
    assert!(!session.slots.valid(2));
    assert_eq!(file_select(&session).menu_state(), 6);
    assert_eq!(file_select(&session).timer(), 0x80);

    idle(&mut session, 128);
    assert_eq!(file_select(&session).menu_state(), 1);
    assert!(!file_select(&session).clear_mode());
}

#[test]
fn acceptance_5_confirm_lands_exactly_64_frames_later_and_b_does_not_cancel() {
    let mut session = session();
    to_file_select(&mut session);
    drop_in(&mut session);
    start_confirm(&mut session);

    let b = PlayerKeys {
        b: true,
        ..PlayerKeys::default()
    };
    for frame in 0..63 {
        step(&mut session, b);
        assert!(
            matches!(session.state, SessionState::FileSelect(_)),
            "the commit must not land before frame 64 (frame {frame})"
        );
        assert_eq!(file_select(&session).menu_state(), 5);
    }
    step(&mut session, b);
    assert!(matches!(session.state, SessionState::Overworld(_)));

    // DOWN is the other arming button.
    let mut down = session_with_slots(&SaveSlots::new().image());
    to_file_select(&mut down);
    drop_in(&mut down);
    press(
        &mut down,
        PlayerKeys {
            down: true,
            ..PlayerKeys::default()
        },
    );
    assert_eq!(file_select(&down).menu_state(), 5);
}

#[test]
fn acceptance_6_one_corrupt_slot_is_reinitialized_alone() {
    let mut slots = SaveSlots::new();
    let first = record(0, 1, 2, 3, 4, 5);
    let third = record(0, 6, 7, 8, 9, 10);
    assert!(slots.write_slot(0, &first));
    assert!(slots.write_slot(2, &third));
    let mut image = slots.image();
    image[SLOT_LEN + 0x46] ^= 0xFF; // break pipe B's signature

    let mut session = session_with_slots(&image);
    let valid = session.slots.validate_and_init();
    assert_eq!(valid, [true, false, true]);
    assert_eq!(session.slots.load_slot(0), Some(first));
    assert_eq!(session.slots.load_slot(2), Some(third));
    assert_eq!(session.slots.slot(1), Some(&fresh_template()));

    // Entering the file select validates again without touching the valid rows.
    session.enter_file_select();
    assert_eq!(session.slots.load_slot(0), Some(first));
    assert_eq!(session.slots.slot(1), Some(&fresh_template()));
    assert_eq!(session.slots.load_slot(2), Some(third));
}

#[test]
fn acceptance_7_easy_mode_is_ui_state_and_erase_clears_it() {
    let mut session = session();
    to_file_select(&mut session);
    drop_in(&mut session);
    press(
        &mut session,
        PlayerKeys {
            select: true,
            ..PlayerKeys::default()
        },
    );
    assert!(file_select(&session).easy_mode());

    confirm_and_load(&mut session);
    assert!(matches!(session.state, SessionState::Overworld(_)));
    session.enter_file_select();
    assert!(!file_select(&session).easy_mode(), "Easy is not saved");

    // Erase mode clears Easy on its toggle frame.
    let mut erase = session_with_slots(&SaveSlots::new().image());
    to_file_select(&mut erase);
    drop_in(&mut erase);
    press(
        &mut erase,
        PlayerKeys {
            select: true,
            ..PlayerKeys::default()
        },
    );
    assert!(file_select(&erase).easy_mode());
    for _ in 0..3 {
        move_right(&mut erase);
    }
    press(
        &mut erase,
        PlayerKeys {
            a: true,
            ..PlayerKeys::default()
        },
    );
    idle(&mut erase, 12);
    assert!(file_select(&erase).clear_mode());
    assert!(!file_select(&erase).easy_mode());
}

#[test]
fn acceptance_8_attract_demo_starts_and_start_interrupts_it() {
    let mut session = session();
    idle(&mut session, 1090);
    match &session.state {
        SessionState::Demo(demo) => {
            assert_eq!(demo.index, 0);
            assert_eq!(demo.level_id, 0);
        }
        _ => panic!("the countdown must start demo 0"),
    }
    press(
        &mut session,
        PlayerKeys {
            start: true,
            ..PlayerKeys::default()
        },
    );
    match &session.state {
        SessionState::Title(title) => assert_eq!(title.demo_index(), 1),
        _ => panic!("START must interrupt the demo"),
    }
}

#[test]
fn acceptance_image_length_is_the_three_pipe_format() {
    assert_eq!(SAVE_IMAGE_LEN, 3 * SLOT_LEN);
}

/// ME-7: a coin door loads its room, the room's script runs to state 3 and
/// `$3F4A0` lands the castle screen again.
#[test]
fn coin_door_room_plays_and_returns_to_the_castle() {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    {
        let SessionState::Overworld(map) = &mut session.state else {
            panic!("a fresh session starts on the map");
        };
        map.screen_id = 0;
        map.node = 0x70; // the $0200 coin door, screen 10
        map.coins_low = 0x00;
        map.coins_high = 0x20;
        map.cursor_walking = 0;
    }
    press(
        &mut session,
        PlayerKeys {
            a: true,
            ..PlayerKeys::default()
        },
    );
    {
        let SessionState::Overworld(map) = &mut session.state else {
            panic!("the door keeps the overworld");
        };
        assert_eq!(map.screen_id, 10, "$3F3F0 loads screen 10");
        let room = map.room.as_mut().expect("the door staged a coin room");
        // The intro page counts up to 7 over ~100 frames; skip that gate so
        // the tape is about the room, not the wait.
        room.set_sprite(0xA6BD, 7);
    }
    idle(&mut session, 2);
    press(
        &mut session,
        PlayerKeys {
            a: true,
            ..PlayerKeys::default()
        },
    );
    idle(&mut session, 2);
    let mut frames = 0;
    loop {
        idle(&mut session, 1);
        frames += 1;
        match &session.state {
            SessionState::Overworld(map) if map.screen_id == 8 => break,
            _ if frames > 4000 => panic!("the coin room never returned"),
            _ => {}
        }
    }
    let SessionState::Overworld(map) = &session.state else {
        panic!("the castle is the overworld");
    };
    assert!(map.room.is_none(), "the room is torn down");
    assert!(frames < 2000, "the room returns in {frames} frames");
}

/// ME-7: the six-coin reveal runs its walk-in and menu, marks the level's
/// progress byte and lands the map.
#[test]
fn castle_reveal_runs_and_lands_the_map() {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    let mut seq = sml2_core::castle_sequence::CastleSequence::default();
    seq.enter(&[0x80; 6]);
    seq.level = 23; // $44F1[23] = $0E
    seq.state = sml2_core::castle_sequence::STATE_REVEAL_A;
    session.begin_castle_sequence(seq, 0, 0x50, 0);
    assert!(matches!(session.state, SessionState::Castle(_)));
    for _ in 0..4000 {
        idle(&mut session, 1);
        if matches!(session.state, SessionState::Overworld(_)) {
            break;
        }
    }
    let SessionState::Overworld(map) = &session.state else {
        panic!("the reveal lands the map");
    };
    assert_eq!(map.screen_id, 0);
    assert_eq!(map.node, 0x50);
    assert_eq!(
        map.level_progress[0x0E] & 0x80,
        0x80,
        "$44AC marks sLevelProgress[$44F1[23]]"
    );
}

/// ME-7: the bell room runs its walk-in and bell sequence and returns to the
/// map once the script writes state `$0A`.
#[test]
fn bell_room_plays_and_returns_to_the_map() {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    session.begin_bell_room(13, 0x50, 0);
    assert!(matches!(session.state, SessionState::Overworld(_)));
    for frame in 0..6000 {
        let keys = if frame % 8 == 0 {
            PlayerKeys {
                a: true,
                ..PlayerKeys::default()
            }
        } else {
            PlayerKeys::default()
        };
        step(&mut session, keys);
        if let SessionState::Overworld(map) = &session.state {
            if map.screen_id == 0 && map.room.is_none() {
                break;
            }
        }
    }
    let SessionState::Overworld(map) = &session.state else {
        panic!("the bell room returns to the map");
    };
    assert_eq!(map.screen_id, 0, "the deferred exit lands the map");
    assert!(map.room.is_none());
}

/// ME-7: the save image written mid-reveal reloads into a fresh session.
#[test]
fn the_castle_reveal_saves_and_reloads() {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    let mut seq = sml2_core::castle_sequence::CastleSequence::default();
    seq.enter(&[0x80; 6]);
    seq.level = 23;
    seq.state = sml2_core::castle_sequence::STATE_REVEAL_A;
    session.begin_castle_sequence(seq, 0, 0x50, 0);
    idle(&mut session, 10);
    let record = session.save_record();
    assert!(save_record_valid(&record), "the mid-chain record is valid");

    let mut other = Session::new(SessionAssets::load().expect("session assets must load"));
    assert!(other.apply_save_record(&record), "the record reloads");
    assert!(matches!(other.state, SessionState::Overworld(_)));
}

/// ME-7: six golden coins, the final castle's level 24 and a Wario clear run
/// the ending entry into the credits sequence.
#[test]
fn six_coins_wario_and_ending_tape() {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    if let SessionState::Overworld(map) = &mut session.state {
        for offset in [5usize, 9, 14, 17, 23, 29] {
            map.level_progress[offset] = 0x80;
        }
    }
    session.begin_level(24, 0x80).expect("level 24 loads");
    idle(&mut session, 60);
    {
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level 24 is live");
        };
        // The fight's own handlers are covered by `player/tests/boss_clear.rs`;
        // this tape starts at its terminal state.
        ctx.sim.boss_clear = true;
        ctx.sim.fade_timer = 2;
        ctx.cam.boss_fight_phase = 3; // the fight's terminal phase
    }
    // The ending scene's own assets (`extracted/ending/scene.json`); substate 4
    // holds until the scene is loaded.
    let ending_assets = sml2_core::ending_scene::EndingAssets::from_json(include_str!(
        "../../../../extracted/ending/scene.json"
    ))
    .expect("ending scene asset");
    {
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level 24 is live");
        };
        let mut state = 4;
        let mut fade = 0x0100;
        ctx.ending_scene.load(ending_assets, &mut state, &mut fade);
    }
    let mut frames = 0;
    let mut substate = 0;
    for _ in 0..30000 {
        idle(&mut session, 1);
        frames += 1;
        substate = match &session.state {
            SessionState::InLevel(ctx) => ctx.ending_sequence.substate,
            _ => substate,
        };
        if !matches!(session.state, SessionState::InLevel(_)) || substate >= 18 {
            break;
        }
    }
    let entry_state = match &session.state {
        SessionState::InLevel(ctx) => ctx.ending_entry.state(),
        _ => sml2_core::ending_entry::EndingEntryState::Sequence,
    };
    assert!(
        entry_state == sml2_core::ending_entry::EndingEntryState::Sequence || substate > 0,
        "the ending entry reached its sequence (substate {substate}, {frames} frames)"
    );
    assert!(
        substate >= 18,
        "the credits page is streaming (substate {substate}, {frames} frames)"
    );
}
