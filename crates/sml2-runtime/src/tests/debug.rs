//! The desktop debug panel's runtime contract: warps, progress edits, purse and
//! golden-coin edits. Each test drives a real `Session` over the extracted
//! assets, so the state it checks is the state the play loop writes.

use crate::session::{Session, SessionAssets, SessionState};
use sml2_core::castle_sequence::{CastleSequence, GOLDEN_COIN_OFFSETS};
use sml2_core::map::progress_offset as po;

fn session() -> Session {
    Session::new(SessionAssets::load().expect("session assets must load"))
}

#[test]
fn warping_across_the_overworld_lands_on_the_requested_screen_and_node() {
    let mut session = session();
    session.debug_clear_all_levels();
    let progress = session.debug_progress();
    // The zone-door landing nodes ($3C60B's NODE_DOORS) plus the hub and castle.
    for (screen, node) in [
        (0u8, 0x14u8),
        (1, 0x28),
        (2, 0x23),
        (3, 0x32),
        (4, 0x3C),
        (5, 0x4B),
        (6, 0x50),
        (7, 0x5A),
        (8, 0x72),
    ] {
        session
            .debug_warp_map(screen, node)
            .unwrap_or_else(|e| panic!("screen {screen} node {node}: {e}"));
        match &session.state {
            SessionState::Overworld(map) => {
                assert_eq!(map.screen_id, screen);
                assert_eq!(map.node, node);
                assert_eq!(map.level_progress, progress, "progress survives the warp");
                if let Some(record) = session.assets.node_records.record(node) {
                    assert_eq!(
                        (map.cursor_x, map.cursor_y),
                        (record.cursor_x, record.cursor_y),
                        "the node record seeds the cursor on screen {screen}"
                    );
                }
                assert_eq!(map.mario_dead, 0xFF, "an arrival, not a fresh entry");
            }
            _ => panic!("screen {screen} did not land on the map"),
        }
        assert_eq!(session.last_map_screen, Some(screen));
        assert_eq!(
            session.sound_driver.music_request,
            sml2_core::map_sim::screen_music(screen),
            "each screen asks for its own song"
        );
    }
}

#[test]
fn a_warp_rejects_screens_and_nodes_the_map_cannot_reach() {
    let mut session = session();
    assert!(session.debug_warp_map(9, 0).is_err(), "screen 9 is a room");
    assert!(
        session.debug_warp_map(0, 0x82).is_err(),
        "past the node table"
    );
    assert!(session.debug_warp_map(0, 0).is_ok());
}

#[test]
fn clearing_every_stage_sets_every_level_slot_and_opens_the_gates() {
    let mut session = session();
    session.debug_wipe_progress();
    session.debug_unlock_routes();
    let slots = session.debug_level_slots();
    assert_eq!(
        slots.len(),
        31,
        "the node table reaches 31 of the 32 levels"
    );
    assert!(!slots.contains(&24), "level 24 has no node");

    let set = session.debug_clear_all_levels();
    assert_eq!(set, slots.len());
    let progress = session.debug_progress();
    for slot in &slots {
        assert_eq!(
            progress[*slot as usize] & 0x80,
            0x80,
            "slot {slot} is cleared"
        );
    }
    // The path bits share bytes with stage clears: bit 0 survives, and the
    // clear flag lands on bit 7 of the same byte.
    for offset in [
        po::TREE_PATH_1,
        po::PUMPKIN_PATH_1,
        po::PUMPKIN_PATH_2,
        po::MACRO_PATH_0,
        po::SPACE_PATH_0,
        po::TURTLE_PATH_1,
    ] {
        assert_eq!(progress[offset] & 0x01, 0x01, "path bit {offset} survives");
        assert_eq!(progress[offset] & 0x80, 0x80, "stage bit {offset} is set");
    }
    // The map's gates read those clear bits, so every gate is open.
    for gate in [
        po::GATE_MACRO,
        po::GATE_MARIO,
        po::GATE_PUMPKIN_B,
        po::GATE_PUMPKIN_C,
        po::GATE_SPACE,
    ] {
        assert_eq!(progress[gate] & 0x80, 0x80, "gate {gate} is open");
    }
    assert_eq!(
        session.debug_golden_coins(),
        [true; 6],
        "a boss stage's clear bit is its golden coin"
    );
}

#[test]
fn opening_the_secret_paths_leaves_the_stage_clears_alone() {
    let mut session = session();
    session.debug_wipe_progress();
    session.debug_unlock_routes();
    let progress = session.debug_progress();
    for offset in [
        po::TREE_PATH_1,
        po::PUMPKIN_PATH_1,
        po::PUMPKIN_PATH_2,
        po::MACRO_PATH_0,
        po::SPACE_PATH_0,
        po::TURTLE_PATH_1,
    ] {
        assert_eq!(progress[offset], 0x01, "only the path bit at {offset}");
    }
    assert_eq!(session.debug_golden_coins(), [false; 6]);
    assert_eq!(
        progress.iter().filter(|byte| **byte & 0x80 != 0).count(),
        0,
        "no stage was cleared"
    );
}

#[test]
fn golden_coins_toggle_individually_and_take_their_stage_with_them() {
    let mut session = session();
    session.debug_wipe_progress();
    assert_eq!(session.debug_golden_coins(), [false; 6]);

    session.debug_set_golden_coin(2, true);
    let mut held = [false; 6];
    held[2] = true;
    assert_eq!(session.debug_golden_coins(), held);
    assert_eq!(session.debug_progress()[14] & 0x80, 0x80);

    session.debug_set_golden_coin(2, false);
    assert_eq!(session.debug_golden_coins(), [false; 6]);
    assert_eq!(
        session.debug_progress()[14] & 0x80,
        0,
        "removing the coin un-clears its stage"
    );

    session.debug_clear_all_levels();
    session.debug_set_golden_coin(0, false);
    assert_eq!(
        session.debug_golden_coins(),
        [false, true, true, true, true, true],
        "only the removed coin goes"
    );
    assert_ne!(
        session.debug_progress()[0] & 0x80,
        0,
        "the other stages stay cleared"
    );

    // On the hub the six icons are staged from the flags at load, so an edit has
    // to restage them: `HUB_COIN_FLAGS` pairs decor slot 4+i with its offset.
    session.debug_warp_map(0, 0x14).expect("hub");
    session.debug_set_golden_coin(1, true);
    let slot = sml2_core::map::HUB_COIN_FLAGS
        .iter()
        .position(|offset| *offset == GOLDEN_COIN_OFFSETS[1] as usize)
        .expect("the hub stages every coin");
    match &session.state {
        SessionState::Overworld(map) => {
            assert_ne!(
                map.decor_slots[4 + slot].3,
                0xFF,
                "the collected coin shows on the hub map"
            );
        }
        _ => panic!("not on the map"),
    }
}

#[test]
fn the_castle_reveal_reads_the_edited_coins() {
    let mut session = session();
    session.debug_wipe_progress();
    session.begin_castle_sequence(CastleSequence::default(), 0, 0x14, 0);
    session.debug_set_golden_coin(1, true);
    let SessionState::Castle(seq) = &session.state else {
        panic!("the castle sequence did not start");
    };
    assert_eq!(seq.coins[1] & 0x80, 0x80);
    assert_eq!(seq.golden_count, 1);
    assert_eq!(session.save_record()[0x08 + 9] & 0x80, 0x80);
}

#[test]
fn warping_into_a_level_carries_the_map_progress_and_the_return_node() {
    let mut session = session();
    session.debug_set_coins(1234);
    session.debug_set_lives(7);
    session.debug_clear_all_levels();
    session.debug_warp_map(2, 0x23).expect("tree zone A");
    let progress = session.debug_progress();

    session.debug_warp_level(3).expect("level 3 loads");
    let slot = session.debug_level_slot(3).expect("level 3 has a slot");
    match &session.state {
        SessionState::InLevel(ctx) => {
            assert_eq!(ctx.manifest.level_id, 3);
            assert_eq!(ctx.sim.level_progress, progress, "the whole image comes in");
            assert_eq!(ctx.sim.level_progress_flags, progress[slot as usize]);
            assert_eq!(
                ctx.sim.map_node, 0x23,
                "the exit hands off where it started"
            );
            assert_eq!((ctx.sim.coins_low, ctx.sim.coins_high), (0x34, 0x12));
            assert_eq!(ctx.sim.lives, 0x07);
        }
        _ => panic!("level 3 did not go live"),
    }

    session.debug_warp_map(0, 0x14).expect("hub");
    assert_eq!(session.debug_coins(), 1234, "the purse survives both warps");
    assert_eq!(session.debug_lives(), 7);
    assert_eq!(session.debug_progress(), progress);
}

#[test]
fn in_level_edits_reach_the_live_player_and_the_purse_is_bcd() {
    let mut session = session();
    session.debug_warp_level(0).expect("level 1 loads");
    assert_eq!(session.debug_coins(), 0);

    session.debug_set_coins(42);
    match &session.state {
        SessionState::InLevel(ctx) => {
            assert_eq!((ctx.sim.coins_low, ctx.sim.coins_high), (0x42, 0))
        }
        _ => panic!("not in a level"),
    }
    assert_eq!(
        session.coins_low, 0x42,
        "the session mirror is kept in step"
    );

    session.debug_add_coins(100);
    assert_eq!(session.debug_coins(), 142);
    session.debug_add_coins(-1000);
    assert_eq!(session.debug_coins(), 0, "saturates at 0000");
    session.debug_set_coins(9999);
    session.debug_add_coins(10);
    assert_eq!((session.debug_coins(), session.coins_high), (9999, 0x99));

    session.debug_set_lives(5);
    session.debug_add_lives(1);
    assert_eq!(session.debug_lives(), 6);
    session.debug_set_lives(0);
    session.debug_add_lives(-1);
    assert_eq!(session.debug_lives(), 0, "saturates at 00");
}

#[test]
fn a_zone_warp_holds_the_cursor_still_until_the_player_presses_a_direction() {
    let mut session = session();
    session.debug_warp_map(2, 0x23).expect("tree zone A");
    let (cursor_x, cursor_y) = match &session.state {
        SessionState::Overworld(map) => (map.cursor_x, map.cursor_y),
        _ => panic!("not on the map"),
    };
    // `$3C32D`'s arrival auto-walk is gated on the arrival latch, so a warp must
    // not hand the cursor to the walk graph on the frames that follow it.
    for _ in 0..8 {
        session.advance(crate::session::FrameInput {
            keys: sml2_core::player_sim::PlayerKeys::default(),
            div: 0,
        });
    }
    match &session.state {
        SessionState::Overworld(map) => {
            assert_eq!((map.screen_id, map.node), (2, 0x23));
            assert_eq!((map.cursor_x, map.cursor_y), (cursor_x, cursor_y));
            assert_eq!(map.cursor_walking, 0);
        }
        _ => panic!("the warp left the map"),
    }
}

#[test]
fn a_level_warp_keeps_the_golden_coins_the_player_already_holds() {
    let mut session = session();
    session.debug_wipe_progress();
    session.debug_set_golden_coin(4, true);
    session.debug_warp_level(20).expect("level 20 loads");
    assert!(matches!(session.state, SessionState::InLevel(_)));
    assert_eq!(
        session.debug_golden_coins(),
        [false, false, false, false, true, false],
        "the coin bits travel with the level's progress image"
    );
}

/// The session's lives/purse/power-up copies only refresh on the level and map
/// hand-offs. A warp has to carry what the live state holds, not those copies.
#[test]
fn warps_carry_what_the_player_picked_up_since_the_last_hand_off() {
    let mut session = session();
    session.debug_warp_level(0).expect("level 1 loads");
    // Written the way play writes them: on the live player only.
    if let SessionState::InLevel(ctx) = &mut session.state {
        ctx.sim.coins_low = 0x37;
        ctx.sim.coins_high = 0x01;
        ctx.sim.lives = 0x08;
        ctx.sim.powerup = 1;
    }
    session.debug_warp_map(0, 0x14).expect("hub");
    match &session.state {
        SessionState::Overworld(map) => {
            assert_eq!((map.coins_low, map.coins_high, map.lives, map.powerup), (0x37, 0x01, 0x08, 1));
        }
        _ => panic!("the map warp did not land"),
    }

    // A coin door on the map spends from the map's own purse.
    if let SessionState::Overworld(map) = &mut session.state {
        map.coins_low = 0x05;
        map.coins_high = 0;
    }
    session.debug_warp_level(3).expect("level 3 loads");
    assert_eq!(session.debug_coins(), 5, "the map's spend reaches the level");
    assert_eq!(session.debug_lives(), 8);
}

/// Warping off the pause screen skips `$2756`'s `sSoundMode = 2`; the warp has
/// to hand the engine back itself.
#[test]
fn warping_from_the_pause_screen_resumes_the_sound_engine() {
    use crate::session::FrameInput;
    use sml2_core::player_sim::PlayerKeys;

    for warp_to_map in [true, false] {
        let mut session = session();
        session.debug_warp_level(0).expect("level 1 loads");
        for _ in 0..8 {
            session.advance(FrameInput { keys: PlayerKeys::default(), div: 0x3D });
        }
        let start = PlayerKeys { start: true, ..PlayerKeys::default() };
        session.advance(FrameInput { keys: start, div: 0x3D });
        assert!(session.sound_driver.sound_paused, "START paused the engine");

        if warp_to_map {
            session.debug_warp_map(0, 0x14).expect("hub");
        } else {
            session.debug_warp_level(3).expect("level 3 loads");
        }
        assert!(!session.sound_driver.sound_paused);
        for _ in 0..4 {
            session.advance(FrameInput { keys: PlayerKeys::default(), div: 0x3D });
        }
        assert!(!session.sound_driver.sound_paused, "the engine stays live");
        assert_ne!(session.sound_driver.active_song, 0, "music plays after the warp");
    }
}

/// Entering a zone from the panel replays the hub door, so it ends where a
/// played entry does. Expected values are PyBoy's (screen, node, cursor) 500
/// frames after A on each door; the warp starts from another zone so nothing
/// of the screen it left can leak through.
#[test]
fn entering_a_zone_from_the_panel_lands_where_the_rom_door_does() {
    use crate::session::FrameInput;
    use sml2_core::player_sim::PlayerKeys;

    for (screen, expect) in [
        (1u8, (1u8, 0x28u8, 83u8, 89u8)),
        (2, (2, 0x23, 48, 110)),
        (3, (3, 0x32, 18, 118)),
        (4, (4, 0x3C, 64, 100)),
        (5, (5, 0x48, 114, 90)),
        (6, (6, 0x50, 60, 112)),
        (8, (8, 0x6E, 40, 131)),
    ] {
        let mut session = session();
        session.debug_warp_map(3, 0x32).expect("start somewhere else");
        for _ in 0..20 {
            session.advance(FrameInput { keys: PlayerKeys::default(), div: 0 });
        }
        session.debug_enter_zone(screen).expect("the door opens");
        assert!(session.map_screen_transition.is_some(), "screen {screen}: the walk-in runs");
        for _ in 0..500 {
            session.advance(FrameInput { keys: PlayerKeys::default(), div: 0 });
        }
        let SessionState::Overworld(map) = &session.state else {
            panic!("screen {screen}: left the overworld");
        };
        assert_eq!(
            (map.screen_id, map.node, map.cursor_x, map.cursor_y),
            expect,
            "screen {screen}"
        );
    }
}
