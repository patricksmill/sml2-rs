//! Mario Zone boss unit tests (Def 2: Three Piggys).

use crate::level::{LevelLayout, LAYOUT_SIZE};
use crate::object::ObjectTables;
use crate::objects::{
    ObjectSim, Staging, B_DEF, B_FLAGS, B_FLIP, B_FSTATE, B_PHASE, B_SCRX, B_SCRY, B_SPRITE,
    B_STATE, B_T24, B_T28, B_TYPE,
};

fn blank_layout() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap()
}

fn tables() -> ObjectTables {
    serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        ))
        .unwrap(),
    )
    .unwrap()
}

fn streamed_slot(x: u16, y: u16, sprite: u8) -> Staging {
    let mut st = Staging::default();
    st.b[1] = (x >> 8) as u8;
    st.b[2] = (x & 0xFF) as u8;
    st.b[3] = (y >> 8) as u8;
    st.b[4] = (y & 0xFF) as u8;
    st.b[B_SPRITE] = sprite;
    st
}

#[test]
fn part_takes_hp_and_falls_at_zero_or_borrow() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 2;
    obj.boss_sprite = 0x2A;
    obj.boss_timer_reload = 0x50;

    let mut st = streamed_slot(0, 0, 0x2C);
    st.b[B_DEF] = 0x0A;
    st.b[B_STATE] = 2;
    st.b[B_FLAGS] = 0x02; // Projectile

    obj.handler_boss_def2_part(&mut st, &layout, &t, false);
    assert_eq!(st.b[B_STATE], 1, "one HP decremented");
    assert_eq!(obj.sfx1, 9);
    assert_eq!(obj.spawn_sprite, 0x2C, "banked sprite");
    assert_eq!(st.b[B_FSTATE], 1, "hit-stun state");

    // Next point defeats it:
    st.b[B_FLAGS] = 0x02;
    obj.handler_boss_def2_part(&mut st, &layout, &t, false);
    assert_eq!(st.b[B_STATE], 0);
    assert_eq!(st.b[B_FSTATE], 2, "defeat fall");
    assert_eq!(st.b[B_T28], 0x20);
    assert_eq!(st.b[B_SPRITE], 0x2A);

    // Stomp borrow case (1 HP - 2 = $FF)
    let mut st = streamed_slot(0, 0, 0);
    st.b[B_DEF] = 0x0A;
    st.b[B_STATE] = 1;
    st.b[B_FLAGS] = 0x01; // Stomp
    obj.boss_hits_remaining = 6;
    obj.boss_slot_ptr_lo = 0;
    obj.boss_hit_latch = 0;
    obj.handler_boss_def2_part(&mut st, &layout, &t, false);
    assert_eq!(st.b[B_STATE], 0xFF, "borrow wraps");
    assert_eq!(st.b[B_FSTATE], 2, "fatal defeat fall");
}

#[test]
fn controller_waits_for_parts_then_spawns_child_anim13() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 2;
    obj.boss_def_count = 1; // 1 part alive

    let mut st = streamed_slot(0, 0, 0);
    st.b[B_DEF] = 0x0C;
    st.b[B_STATE] = 0;
    st.b[B_FLAGS] = 0x02;
    obj.boss_slot_ptr_lo = 0;
    obj.boss_hits_remaining = 6;

    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_STATE], 0, "controller has no HP");
    assert_eq!(st.b[B_FSTATE], 1, "state 1");

    // Intro holds while a part is alive
    st.b[B_FSTATE] = 0;
    st.b[B_FLAGS] = 0;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_T28], 0);

    // Count runs down when parts are clear
    obj.boss_def_count = 0;
    for _ in 0..0x31 {
        st.b[B_FSTATE] = 0;
        obj.done = 0;
        obj.handler_boss_def2_controller(&mut st, &layout, &t);
        if obj.boss_def_count != 0 {
            break;
        }
    }
    assert_eq!(obj.slots[1].b[B_DEF], 0x0D, "anim 13 spawned");
    assert_eq!(obj.slots[1].b[B_SPRITE], 0x0E);
    assert_eq!(st.b[B_FSTATE], 3);
    assert_eq!(st.b[B_TYPE], 0x45);
}

#[test]
fn spawner_anim2_requires_remove_mode_2() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 0;
    let mut st = streamed_slot(0, 0, 0);
    st.b[B_DEF] = 2;
    st.b[B_TYPE] = 0x48;

    // Normal mode: no-op
    obj.handler_boss_def2(&mut st);
    assert_eq!(st.b[B_TYPE], 0x48);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(obj.boss_def_count, 0);

    // Remove mode 2: spawns child 4
    obj.remove_mode = 2;
    obj.handler_boss_def2(&mut st);
    assert_eq!(st.b[B_TYPE], 0);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.b[B_SPRITE], 7);
    assert_eq!(obj.boss_def_count, 1);
    assert_eq!(obj.slots[1].b[B_DEF], 4);
    assert_eq!(obj.slots[1].b[B_SPRITE], 9);
}

#[test]
fn child_anim13_morphs_to_anim4_under_remove_mode_2() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = streamed_slot(0, 0, 0);
    st.b[B_DEF] = 13;
    st.b[B_FSTATE] = 3;
    st.b[B_TYPE] = 0x40;

    // Remove mode != 2: returns immediately
    obj.remove_mode = 0;
    obj.handler_boss_anim13(&mut st);
    assert_eq!(st.b[B_DEF], 13);
    assert_eq!(st.b[B_FSTATE], 3);

    // Remove mode == 2: sets screen pos and morphs to anim 4 / sprite 9
    obj.remove_mode = 2;
    obj.handler_boss_anim13(&mut st);
    assert_eq!(st.b[B_SCRY], 0x90);
    assert_eq!(st.b[B_SCRX], 0x58);
    assert_eq!(st.b[B_DEF], 4);
    assert_eq!(st.b[B_SPRITE], 9);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(st.b[B_PHASE], 0);
    assert_eq!(st.b[B_TYPE], 0);
}

#[test]
fn pig2_hops_left_preserves_state5_and_turns_at_boundary() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 2;

    let mut st = streamed_slot(0, 0, 0x20);
    st.b[B_DEF] = 11;
    st.b[B_STATE] = 6;
    st.b[B_SCRX] = 0x88;
    st.b[B_SCRY] = 0x8E;
    st.b[B_T24] = 1;
    st.b[B_FSTATE] = 3; // intro drop from chimney
    // Step until descent completes at 0x90: lands into state 5 (facing left) with crouch pose
    for _ in 0..20 {
        obj.handler_boss_def2_pig2(&mut st, &layout, &t);
        if st.b[B_SCRY] >= 0x90 {
            break;
        }
    }
    assert_eq!(st.b[B_SCRY], 0x90);
    assert_eq!(st.b[B_FSTATE], 5, "lands into leftward hop state");
    assert_eq!(st.b[B_FLIP], 0, "facing left");
    assert_eq!(st.b[B_SPRITE], 0x28, "crouch pose");
    assert_eq!(st.b[B_T28], 8, "crouch countdown reload");

    // Spend the 8-frame crouch:
    for i in (1..=7).rev() {
        obj.handler_boss_def2_pig2(&mut st, &layout, &t);
        assert_eq!(st.b[B_T28], i as u8);
        assert_eq!(st.b[B_SPRITE], 0x28, "crouching pose");
        assert_eq!(st.b[B_SCRY], 0x90, "stays grounded while crouching");
    }
    // Next call: B_T28 reaches 0, sprite swaps to 0x20, and jump begins
    obj.handler_boss_def2_pig2(&mut st, &layout, &t);
    assert_eq!(st.b[B_T28], 0);
    assert_eq!(st.b[B_SPRITE], 0x20, "swaps to jump sprite");
    assert_eq!(st.b[B_FSTATE], 5, "still in state 5 (moving left)");

    // Complete the hop and verify that landing does NOT force state 4:
    st.b[B_T24] = 1;
    st.b[B_SCRY] = 0x8E;
    for _ in 0..20 {
        obj.handler_boss_def2_pig2(&mut st, &layout, &t);
        if st.b[B_SCRY] >= 0x90 {
            break;
        }
    }
    assert_eq!(st.b[B_SCRY], 0x90);
    assert_eq!(st.b[B_FSTATE], 5, "landing preserves state 5 (leftward)");
    assert_eq!(st.b[B_FLIP], 0, "preserves leftward flip");
    assert_eq!(st.b[B_T28], 8, "reloads crouch countdown for next hop");
    assert_eq!(st.b[B_SPRITE], 0x28);

    // When hitting left arena boundary, flips to state 4 (moving right):
    st.b[B_SCRX] = 0x10;
    st.b[B_T28] = 0; // in air
    obj.handler_boss_def2_pig2(&mut st, &layout, &t);
    assert_eq!(st.b[B_FSTATE], 4, "flipped to rightward hop");
    assert_eq!(st.b[B_FLIP], 0x20, "flipped to right-facing");
}

#[test]
fn anim4_spawns_anim5_and_chain_reaches_win() {
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 1;

    let mut st = streamed_slot(0, 0, 9);
    st.b[B_DEF] = 4;
    st.b[B_FSTATE] = 0;
    st.b[B_SCRY] = 0x90;
    st.b[B_SCRX] = 0x58;

    // State 0: spawns anim 5 and transitions to state 1
    obj.handler_boss_anim4(&mut st, &t);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(obj.boss_def_count, 2);
    assert_eq!(obj.slots[2].b[B_DEF], 5);
    assert_eq!(obj.slots[2].b[B_SPRITE], 0x0C);

    // Anim 4 state 1: descent with b1d6
    st.b[B_PHASE] = 21; // at cap
    obj.handler_boss_anim4(&mut st, &t);
    assert_eq!(st.b[B_FSTATE], 2);
    assert_eq!(st.b[B_FLIP] & 0x40, 0x40, "flips at state transition");

    // Anim 4 state 2: descent with profile 21
    st.b[B_PHASE] = 21; // at cap
    obj.handler_boss_anim4(&mut st, &t);
    assert_eq!(st.b[B_FSTATE], 3);
    assert_eq!(st.b[B_FLIP] & 0x40, 0x00, "flips again");

    // Anim 4 state 3: rises 4 px per frame until SCRY < 8
    st.b[B_SCRY] = 10;
    obj.boss_clear_flag = 0;
    obj.handler_boss_anim4(&mut st, &t);
    assert_eq!(st.b[B_SCRY], 6);
    assert_eq!(obj.boss_clear_flag, 1, "win flag armed at SCRY < 8");
}

#[test]
fn anim5_waits_8_frames_then_descends() {
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = streamed_slot(0, 0, 0x0C);
    st.b[B_DEF] = 5;
    st.b[B_FSTATE] = 0;
    st.b[B_SCRY] = 0x50;

    for _ in 0..7 {
        obj.handler_boss_anim5(&mut st, &t);
        assert_eq!(st.b[B_FSTATE], 0);
        assert_eq!(st.b[B_SCRY], 0x50);
    }
    // 8th frame: SCRY += 0x18 and state 1
    obj.handler_boss_anim5(&mut st, &t);
    assert_eq!(st.b[B_SCRY], 0x68);
    assert_eq!(st.b[B_FSTATE], 1);
}

#[test]
fn pig3_crouches_then_leaps_high_and_lands_into_crouch() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 2;
    obj.boss_hits_remaining = 6; // full HP: speed = 1

    let mut st = streamed_slot(0, 0, 0x20);
    st.b[B_DEF] = 12;
    st.b[B_FSTATE] = 4; // landed into state 4 (moving left)
    st.b[B_T28] = 8;    // 8-frame crouch
    st.b[B_SCRX] = 0x60;
    st.b[B_SCRY] = 0x90;

    // First 7 frames: crouched in pose 0x28
    for i in (1..=7).rev() {
        obj.handler_boss_def2_controller(&mut st, &layout, &t);
        assert_eq!(st.b[B_T28], i as u8);
        assert_eq!(st.b[B_SPRITE], 0x28, "crouching pose");
        assert_eq!(st.b[B_SCRY], 0x90, "grounded");
    }
    // 8th call: B_T28 reaches 0, swaps to jump sprite 0x29
    obj.fff0 = 1; // odd frame -> returns without moving
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_T28], 0);
    assert_eq!(st.b[B_SPRITE], 0x29, "jump pose armed");
    assert_eq!(st.b[B_SCRY], 0x90);

    // On even frame: starts high ascent (step + 2)
    obj.fff0 = 2;
    let initial_y = st.b[B_SCRY];
    let initial_x = st.b[B_SCRX];
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_SCRX], initial_x - 1, "speed 1 leftward");
    assert!(st.b[B_SCRY] < initial_y, "ascended upward");

    // Step through remaining ascent frames (14 more even frames)
    for _ in 0..14 {
        obj.fff0 = 4;
        obj.handler_boss_def2_controller(&mut st, &layout, &t);
    }
    // Reached apex: phase reset to 0, T24 set to 1 (descent)
    assert_eq!(st.b[B_T24], 1, "toggled to descent");
    assert_eq!(st.b[B_PHASE], 0, "phase reset for descent");

    // Step through descent (15 even frames)
    for _ in 0..15 {
        obj.fff0 = 6;
        obj.handler_boss_def2_controller(&mut st, &layout, &t);
    }
    // Landed!
    assert_eq!(st.b[B_SCRY], 0x90, "landed at floor 0x90");
    assert_eq!(st.b[B_T24], 0, "descent complete");
    assert_eq!(st.b[B_T28], 8, "crouch reload on landing");
    assert_eq!(obj.sfx1, 12, "landing sfx");
}

#[test]
fn pig3_speed_escalates_and_bounces_between_walls() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 2;

    let mut st = streamed_slot(0, 0, 0x29);
    st.b[B_DEF] = 12;
    st.b[B_T28] = 0; // mid-air jump

    // Hit 0 (6 hits left): speed = 4 - 3 = 1
    obj.boss_hits_remaining = 6;
    st.b[B_FSTATE] = 4; // left
    st.b[B_SCRX] = 0x50;
    obj.fff0 = 2;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_SCRX], 0x4F, "speed 1 at 6 hits");

    // Hit 1 (4 hits left): speed = 4 - 2 = 2
    obj.boss_hits_remaining = 4;
    st.b[B_SCRX] = 0x50;
    obj.fff0 = 4;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_SCRX], 0x4E, "speed 2 at 4 hits");

    // Hit 2 (2 hits left): speed = 4 - 1 = 3
    obj.boss_hits_remaining = 2;
    st.b[B_SCRX] = 0x50;
    obj.fff0 = 6;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_SCRX], 0x4D, "speed 3 at 2 hits");

    // Wall bounce left: hitting < 0x10 flips to State 5 (moving right)
    st.b[B_FSTATE] = 4;
    st.b[B_FLIP] = 0x20; // facing left
    st.b[B_SCRX] = 0x11;
    obj.fff0 = 8;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_FSTATE], 5, "flipped to rightward state");
    assert_eq!(st.b[B_FLIP], 0, "flipped facing right");

    // Wall bounce right: hitting >= 0xA0 flips to State 4 (moving left)
    st.b[B_FSTATE] = 5;
    st.b[B_SCRX] = 0x9F;
    obj.fff0 = 10;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_FSTATE], 4, "flipped to leftward state");
    assert_eq!(st.b[B_FLIP], 0x20, "flipped facing left");
}

