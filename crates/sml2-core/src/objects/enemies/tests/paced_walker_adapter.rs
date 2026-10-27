use super::*;
use crate::level::{LAYOUT_HEIGHT, LAYOUT_SIZE, LAYOUT_WIDTH};

fn dirty_slot() -> [u8; 32] {
    std::array::from_fn(|index| (index as u8).wrapping_mul(37) ^ 0xA5)
}

fn empty_layout() -> LevelLayout {
    LevelLayout::from_bytes(&vec![0x60; LAYOUT_SIZE]).unwrap()
}

fn run(slot: [u8; 32], frame: u8, player_x: u16, layout: &LevelLayout) -> ([u8; 32], u8) {
    let mut sim = ObjectSim::new(Vec::new(), 0);
    sim.fff0 = frame;
    sim.mario_ref_x = player_x;
    sim.facing = 0xA5;
    let mut staging = Staging::default();
    staging.b = slot;
    sim.handler_paced_walker(&mut staging, layout);
    (staging.b, sim.facing)
}

#[test]
fn paced_walker_conversion_preserves_every_byte_including_unknown_states() {
    for index in 0..32 {
        for value in 0..=255 {
            let mut slot = dirty_slot();
            slot[index] = value;
            let mut actor = Actor::from_raw_slot(&slot);
            let walker = from_actor(&actor);
            write_actor(&walker, &mut actor);
            assert_eq!(actor.to_raw_slot(), slot, "index={index} value={value}");
        }
    }
}

#[test]
fn paced_walker_init_preserves_unowned_bytes_and_maps_facing_for_unknown_states() {
    let layout = empty_layout();
    for state in 0..=255 {
        if state == 1 || state == 2 {
            continue;
        }
        for (player_x, walk_state, horizontal, facing) in
            [(99, 1, 0, 0xFF), (100, 1, 0, 0), (101, 2, 0x20, 1)]
        {
            let mut slot = dirty_slot();
            set_x(&mut slot, 100);
            slot[15] = state;
            let mut expected = slot;
            expected[14] = 0x10;
            expected[18] = 0x4C;
            expected[15] = walk_state;
            // Preserve the pre-pilot init's handling of unrelated flip bits.
            expected[13] = (expected[13] & !0x20) | horizontal;
            expected[28] = 1;
            assert_eq!(run(slot, 1, player_x, &layout), (expected, facing));
        }
    }
}

#[test]
fn paced_walker_budget_and_pause_bytes_wrap_without_changing_other_fields() {
    let layout = empty_layout();
    for state in [1, 2] {
        for value in 0..=255u8 {
            let mut slot = dirty_slot();
            set_x(&mut slot, if state == 1 { 0 } else { u16::MAX });
            set_y(&mut slot, 200);
            slot[15] = state;
            slot[25] = value;
            slot[28] = 0;
            let mut expected = slot;
            set_x(&mut expected, if state == 1 { u16::MAX } else { 0 });
            expected[25] = value.wrapping_sub(1);
            if value == 1 {
                expected[28] = 16;
            }
            assert_eq!(run(slot, 1, 100, &layout), (expected, 0xA5));

            slot[28] = value;
            if value == 0 {
                continue;
            }
            let mut expected = slot;
            expected[28] = value - 1;
            if value == 1 {
                expected[25] = 48;
            }
            assert_eq!(run(slot, 1, 100, &layout), (expected, 0xA5));
        }
    }
}

#[test]
fn paced_walker_sprite_toggle_matches_every_frame_and_sprite_byte() {
    let layout = empty_layout();
    for frame in 0..=255u8 {
        for sprite in 0..=255u8 {
            let mut slot = dirty_slot();
            set_x(&mut slot, 100);
            set_y(&mut slot, 200);
            slot[11] = sprite;
            slot[15] = 1;
            slot[28] = 255;
            let mut expected = slot;
            expected[28] = 254;
            if frame & 3 == 0 {
                expected[11] ^= 1;
            }
            assert_eq!(run(slot, frame, 100, &layout), (expected, 0xA5));
        }
    }
}

#[test]
fn paced_walker_wall_turns_before_pause_or_budget_changes() {
    for (state, column) in [(1, 5), (2, 6)] {
        let mut tiles = vec![0x60; LAYOUT_WIDTH * LAYOUT_HEIGHT];
        tiles[11 * LAYOUT_WIDTH + column] = 0;
        let layout = LevelLayout::from_bytes(&tiles).unwrap();
        for pause in [0, 1, 255] {
            let mut slot = dirty_slot();
            set_x(&mut slot, 100);
            set_y(&mut slot, 200);
            slot[15] = state;
            slot[28] = pause;
            let mut expected = slot;
            expected[15] = state ^ 3;
            expected[13] ^= 0x20;
            assert_eq!(run(slot, 1, 100, &layout), (expected, 0xA5));
        }
    }
}
