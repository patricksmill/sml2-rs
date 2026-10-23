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
fn pig2_intro_lands_in_source_leftward_state_then_crouches() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.boss_def_index = 2;
    let mut st = Staging::default();
    st.b[B_DEF] = 11;
    st.b[B_STATE] = 6;
    st.b[B_SPRITE] = 0x20;
    st.b[B_SCRX] = 0x88;
    st.b[B_SCRY] = 0x8E;
    st.b[B_T24] = 1;
    st.b[B_FSTATE] = 3;
    for _ in 0..20 {
        obj.handler_boss_def2_pig2(&mut st, &layout, &t);
        if st.b[B_FSTATE] == 4 {
            break;
        }
    }
    assert_eq!(
        (st.b[B_SCRY], st.b[B_FSTATE], st.b[B_FLIP]),
        (0x90, 4, 0x20)
    );
    assert_eq!((st.b[B_SPRITE], st.b[B_T28]), (0x20, 8));
    let x = st.b[B_SCRX];
    for remaining in (1..=7).rev() {
        obj.handler_boss_def2_pig2(&mut st, &layout, &t);
        assert_eq!((st.b[B_T28], st.b[B_SPRITE]), (remaining, 0x28));
        assert_eq!((st.b[B_SCRX], st.b[B_SCRY]), (x, 0x90));
    }
    obj.handler_boss_def2_pig2(&mut st, &layout, &t);
    assert_eq!((st.b[B_T28], st.b[B_SPRITE]), (0, 0x20));
    assert_eq!((st.b[B_SCRX], st.b[B_SCRY]), (x - 1, 0x8D));
    assert_eq!((st.b[B_FSTATE], st.b[B_PHASE]), (4, 1));
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

    // Anim 4 state 1: ascent with b1d6
    st.b[B_PHASE] = 21; // at cap
    obj.handler_boss_anim4(&mut st, &t);
    assert_eq!(st.b[B_FSTATE], 2);
    assert_eq!(st.b[B_FLIP] & 0x40, 0x40, "flips at state transition");

    // Anim 4 state 2: descent with profile 21
    st.b[B_PHASE] = 21; // at cap
    obj.handler_boss_anim4(&mut st, &t);
    assert_eq!(st.b[B_FSTATE], 3);
    assert_eq!(st.b[B_FLIP] & 0x40, 0x00, "flips again");

    // Anim 4 state 3: rises 4 px per frame until SCRY < 8 then transitions to state 4
    st.b[B_SCRY] = 10;
    obj.boss_clear_flag = 0;
    obj.boss_clear_handoff = false;
    obj.handler_boss_anim4(&mut st, &t);
    assert_eq!(st.b[B_SCRY], 6);
    assert_eq!(st.b[B_FSTATE], 3);
    assert_eq!(obj.boss_clear_flag, 1, "win flag armed at SCRY < 8");
    assert!(obj.boss_clear_handoff, "level handoff armed at SCRY < 8");
    assert_eq!(obj.done, 1);
    // The source keeps state 3; game state 29 consumes the terminal event.
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
    obj.boss_hits_remaining = 6;

    let mut st = streamed_slot(0, 0, 0x20);
    st.b[B_DEF] = 12;
    st.b[B_FSTATE] = 4;
    st.b[B_T28] = 8;
    st.b[B_SCRX] = 0x60;
    st.b[B_SCRY] = 0x90;

    for i in (1..=7).rev() {
        obj.handler_boss_def2_controller(&mut st, &layout, &t);
        assert_eq!(
            (st.b[B_T28], st.b[B_SPRITE], st.b[B_SCRY]),
            (i as u8, 0x28, 0x90)
        );
    }
    obj.fff0 = 1;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!((st.b[B_T28], st.b[B_SPRITE], st.b[B_SCRY]), (0, 0x29, 0x90));

    obj.fff0 = 2;
    let (initial_x, initial_y) = (st.b[B_SCRX], st.b[B_SCRY]);
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!(st.b[B_SCRX], initial_x - 1);
    assert!(st.b[B_SCRY] < initial_y);

    for _ in 0..14 {
        obj.fff0 = 4;
        obj.handler_boss_def2_controller(&mut st, &layout, &t);
    }
    assert_eq!((st.b[B_T24], st.b[B_PHASE]), (1, 0));

    for _ in 0..15 {
        obj.fff0 = 6;
        obj.handler_boss_def2_controller(&mut st, &layout, &t);
    }
    assert_eq!(
        (st.b[B_SCRY], st.b[B_T24], st.b[B_T28], obj.sfx1),
        (0x90, 0, 8, 12)
    );
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

    // Hits: speed escalates (4 - hits/2)
    for (hits, fff0, exp_x) in [(6, 2, 0x4F), (4, 4, 0x4E), (2, 6, 0x4D)] {
        obj.boss_hits_remaining = hits;
        st.b[B_FSTATE] = 4;
        st.b[B_SCRX] = 0x50;
        obj.fff0 = fff0;
        obj.handler_boss_def2_controller(&mut st, &layout, &t);
        assert_eq!(st.b[B_SCRX], exp_x);
    }

    // Wall bounce left: < 0x10 flips to State 5 (moving right)
    st.b[B_FSTATE] = 4;
    st.b[B_FLIP] = 0x20;
    st.b[B_SCRX] = 0x11;
    obj.fff0 = 8;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (5, 0));

    // Wall bounce right: >= 0xA0 flips to State 4 (moving left)
    st.b[B_FSTATE] = 5;
    st.b[B_SCRX] = 0x9F;
    obj.fff0 = 10;
    obj.handler_boss_def2_controller(&mut st, &layout, &t);
    assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (4, 0x20));
}

#[test]
fn pig3_hop_matches_rom_steps_and_preserves_landing_height() {
    // Bank 2 $9E9A calls $B1EB/$B187, adds two, then shares $9DC7.
    // Literal $B214 rise and first 15 bytes of $B1A3 fall.
    let rise = [3, 3, 2, 3, 2, 1, 2, 2, 1, 2, 1, 1, 1, 0, 0];
    let fall = [0, 0, 1, 1, 1, 2, 1, 2, 2, 1, 2, 3, 2, 3, 3];
    let t = tables();
    let layout = blank_layout();
    for (state, dx) in [(4, 255u8), (5, 1)] {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.boss_hits_remaining = 6;
        let mut st = streamed_slot(0, 0, 41);
        st.b[B_DEF] = 12;
        st.b[B_FSTATE] = state;
        st.b[B_SCRX] = 88;
        st.b[B_SCRY] = 150; // Source completes by phase, with no floor clamp.
        let mut expected = st.b;
        for (leg, curve) in [(0, rise), (1, fall)] {
            for (phase, step) in curve.into_iter().enumerate() {
                obj.fff0 = 1;
                obj.handler_boss_def2_controller(&mut st, &layout, &t);
                assert_eq!(st.b, expected, "odd frames hold the hop");
                expected[B_SCRX] = expected[B_SCRX].wrapping_add(dx);
                expected[B_SCRY] = if leg == 0 {
                    expected[B_SCRY].wrapping_sub(step + 2)
                } else {
                    expected[B_SCRY].wrapping_add(step + 2)
                };
                expected[B_PHASE] = (phase as u8 + 1) % 15;
                expected[B_T24] = if phase == 14 { leg ^ 1 } else { leg };
                if leg == 1 && phase == 14 {
                    expected[B_T28] = 8;
                }
                obj.fff0 = 2;
                obj.handler_boss_def2_controller(&mut st, &layout, &t);
                assert_eq!(st.b, expected, "state {state}, leg {leg}, phase {phase}");
            }
        }
        assert_eq!((st.b[B_SCRY], obj.sfx1), (150, 12));
    }
}
