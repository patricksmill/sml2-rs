use super::*;

fn tick(seq: &mut CastleSequence, keys: u8) -> SequenceOut {
    seq.tick(
        SequenceInput {
            keys_pressed: keys,
            ..Default::default()
        },
        0,
        &mut [0; 42],
    )
}

#[test]
fn door_arrival_and_departure_follow_the_two_64_frame_tables() {
    let mut seq = CastleSequence::default();
    tick(&mut seq, 0);
    tick(&mut seq, 0x49); // input cannot skip arrival
    assert_eq!(
        (seq.walk_x, seq.walk_y, seq.menu_timer, seq.sprite_id),
        (88, 167, 63, 0xBB)
    );
    for _ in 0..63 {
        tick(&mut seq, 0);
    }
    assert_eq!((seq.menu, seq.walk_y), (2, 136));
    for key in [1, 2, 8, 0x40] {
        tick(&mut seq, key);
        assert_eq!(seq.menu, 2);
    }
    tick(&mut seq, 0x81); // Down must be the exact byte
    assert_eq!(seq.menu, 2);
    tick(&mut seq, 0x80);
    assert_eq!((seq.menu, seq.menu_timer), (1, 64));
    tick(&mut seq, 0);
    assert_eq!((seq.walk_y, seq.sprite_id), (137, 0xB8));
    for _ in 0..62 {
        tick(&mut seq, 0);
    }
    assert_eq!(seq.walk_y, 168);
    assert_eq!(
        tick(&mut seq, 0).transition,
        Some(CastleTransition::ReturnToMap { walked_out: true })
    );
}

#[test]
fn six_coin_door_accepts_a_start_or_up_and_ignores_b() {
    for key in [1, 8, 0x40, 0x49] {
        let mut seq = CastleSequence::default();
        seq.enter(&[0x80; 6]);
        for _ in 0..65 {
            tick(&mut seq, 0);
        }
        tick(&mut seq, 2);
        assert_eq!(seq.menu, 2);
        tick(&mut seq, key);
        assert_eq!((seq.menu, seq.fade_low), (3, 64));
        for _ in 0..16 {
            tick(&mut seq, 0);
        }
        assert_eq!(
            (seq.fade_low, seq.bgp, seq.obp1, seq.obp2),
            (48, 0xE4, 0xD0, 0x38)
        );
        tick(&mut seq, 0);
        assert_eq!((seq.bgp, seq.obp1, seq.obp2), (0xE9, 0xE5, 0x79));
        for _ in 0..46 {
            tick(&mut seq, 0);
        }
        assert_eq!((seq.fade_low, seq.bgp), (1, 0xFF));
        assert_eq!(
            tick(&mut seq, 0).transition,
            Some(CastleTransition::EnterLevel(24))
        );
    }
}

#[test]
fn fade_borrows_from_the_high_byte_and_rows_are_consumed_once() {
    let mut seq = CastleSequence::default();
    seq.state = STATE_MENU;
    seq.menu = 1;
    seq.fade_high = 1;
    seq.fade_low = 0;
    seq.row_shift_index = 5;
    tick(&mut seq, 0);
    assert_eq!(
        (
            seq.fade_high,
            seq.fade_low,
            seq.row_shift_index,
            seq.door_open_rows
        ),
        (0, 255, 0, 1)
    );
    tick(&mut seq, 0);
    assert_eq!(seq.door_open_rows, 1);
}

#[test]
fn boss_deposit_wraps_y_without_moving_x_and_marks_only_its_coin() {
    let mut seq = CastleSequence::default();
    seq.enter(&[0; 6]);
    seq.state = STATE_TALLY;
    seq.level = 5;
    let mut progress = [0; 42];
    seq.tick(SequenceInput::default(), 5, &mut progress);
    assert_eq!((seq.walk_x, seq.walk_y, seq.level_tally), (59, 192, 1));
    for frame in 0..369u16 {
        seq.tick(
            SequenceInput {
                frame: frame as u8,
                ..Default::default()
            },
            5,
            &mut progress,
        );
    }
    assert_eq!((seq.walk_x, seq.walk_y, seq.menu), (59, 121, 0));
    seq.tick(SequenceInput::default(), 5, &mut progress);
    assert_eq!((seq.menu, seq.fade_high, seq.fade_low), (1, 1, 255));
    for _ in 0..511 {
        seq.tick(SequenceInput::default(), 5, &mut progress);
    }
    assert_eq!(
        (progress[5], seq.coins, seq.golden_count),
        (0x80, [0x80, 0, 0, 0, 0, 0], 1)
    );
}

fn flags(set: usize) -> [u8; 6] {
    let mut coins = [0u8; 6];
    for coin in coins.iter_mut().take(set) {
        *coin = 0x80;
    }
    coins
}

#[test]
fn the_six_coin_test_is_bit_seven_on_every_flag() {
    assert!(CastleSequence::all_six_coins(&flags(6)));
    assert!(!CastleSequence::all_six_coins(&flags(5)));
    assert_eq!(CastleSequence::count_coins(&flags(5)), 5);
}

/// `$45B6`: the reveal opens with the coin run set only when all six are
/// collected, and `$1B` stops the music.
#[test]
fn the_reveal_checks_the_six_coins_and_advances() {
    let mut seq = CastleSequence::default();
    seq.enter(&flags(6));
    let mut progress = [0u8; 42];
    let out = seq.tick(SequenceInput::default(), 0, &mut progress);
    assert_eq!(seq.coin_done, 0x74);
    assert_eq!(seq.state, STATE_REVEAL_B);
    assert_eq!(seq.menu_timer, 64);
    assert_eq!(out.music, Some(0xFF));

    let mut seq = CastleSequence::default();
    seq.enter(&flags(4));
    let _ = seq.tick(SequenceInput::default(), 0, &mut progress);
    assert_eq!(seq.coin_done, 0, "four coins leave the run closed");
}

/// `$441D`: the tally counts once and stages coin y=192.
#[test]
fn the_tally_state_stages_the_deposited_coin() {
    let mut seq = CastleSequence::default();
    seq.state = STATE_TALLY;
    seq.progress_flags = 0;
    let mut progress = [0u8; 42];
    seq.tick(SequenceInput::default(), 0, &mut progress);
    assert_eq!(seq.state, STATE_MENU);
    assert_eq!((seq.walk_x, seq.walk_y), (0, 192));
    assert_eq!(seq.menu, 0);
}

/// `$453B` -> `$44AC`: arrival plays music 26 with five coins, then the
/// progress bit lands and the sequence returns to the map.
#[test]
fn the_menu_walk_in_and_reveal_complete_the_progress_bit() {
    let mut seq = CastleSequence::default();
    seq.enter(&flags(5));
    seq.state = STATE_MENU;
    let mut progress = [0u8; 42];
    let out = seq.tick(
        SequenceInput {
            walk_done: true,
            ..Default::default()
        },
        0,
        &mut progress,
    );
    assert_eq!(out.music, Some(26), "$453B with five coins");
    assert_eq!(seq.menu, 1);
    assert_eq!((seq.fade_high, seq.fade_low), (1, 0xFF));

    // Burn the fade down, then the reveal marks the level.
    for _ in 0..600 {
        if seq.menu != 1 {
            break;
        }
        let _ = seq.tick(SequenceInput::default(), 0, &mut progress);
    }
    assert_eq!(seq.progress_flags & 0x80, 0x80);
    assert_eq!(seq.menu, 2, "five coins open the row shift");
    assert_eq!(seq.row_shift_count, 5);
}

/// `$44AC` marks `sLevelProgress[$44F1[sCurLevel]]` with bit 7.
#[test]
fn the_reveal_marks_the_level_progress_byte() {
    assert_eq!(LEVEL_PROGRESS_OFFSETS[9], 9);
    assert_eq!(LEVEL_PROGRESS_OFFSETS[23], 0x0E);
    let mut seq = CastleSequence::default();
    seq.enter(&flags(5));
    seq.state = STATE_MENU;
    seq.menu = 1;
    seq.fade_high = 0;
    seq.fade_low = 0;
    let mut progress = [0u8; 42];
    let _ = seq.tick(SequenceInput::default(), 9, &mut progress);
    assert_eq!(progress[9] & 0x80, 0x80);
    assert_eq!(seq.progress_flags & 0x80, 0x80);
}

/// `$453B`: the coin steps y once per even frame to `$499D[level]`.
#[test]
fn the_coin_flight_reaches_the_level_target() {
    let mut seq = CastleSequence::default();
    seq.enter(&flags(5));
    seq.level = 5; // $499D[5] = $79
    seq.walk_y = 16;
    seq.state = STATE_MENU;
    let mut progress = [0u8; 42];
    let mut frames = 0;
    for frame in 0..600u16 {
        let _ = seq.tick(
            SequenceInput {
                frame: frame as u8,
                ..Default::default()
            },
            5,
            &mut progress,
        );
        frames += 1;
        if seq.menu != 0 {
            break;
        }
    }
    assert_eq!(seq.walk_y, COIN_TARGET_Y[5]);
    assert_eq!(seq.menu, 1);
    assert!(frames < 300, "about two frames per pixel");
}

/// `$4477`: five shifts, each waiting out a 64-frame fade with SFX 2 = 10,
/// then `$44A1` hands back to the map.
#[test]
fn the_row_shift_runs_five_times_and_returns() {
    let mut seq = CastleSequence::default();
    seq.state = STATE_MENU;
    seq.menu = 2;
    seq.golden_count = 5;
    seq.row_shift_count = 5;
    let mut progress = [0u8; 42];
    let mut shifts = 0;
    for _ in 0..2000 {
        let out = seq.tick(SequenceInput::default(), 0, &mut progress);
        if out.sfx2 == Some(10) {
            shifts += 1;
        }
        if out.done {
            break;
        }
    }
    assert_eq!(shifts, 5);
    assert_eq!(seq.state, STATE_MAP);
}

/// `$21`: the fade expiring or START both end the sequence.
#[test]
fn the_start_wait_accepts_the_fade_or_start() {
    let mut seq = CastleSequence::default();
    seq.state = STATE_WAIT_START;
    seq.fade_low = 0x40;
    let mut progress = [0u8; 42];
    assert!(!seq.tick(SequenceInput::default(), 0, &mut progress).done);
    let out = seq.tick(
        SequenceInput {
            keys_pressed: 0x08,
            ..Default::default()
        },
        0,
        &mut progress,
    );
    assert!(out.done);
}
