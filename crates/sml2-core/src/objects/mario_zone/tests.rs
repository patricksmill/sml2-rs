//! Mario Zone boss unit tests (Def 2: Three Piggys).

use crate::level::{LevelLayout, LAYOUT_SIZE};
use crate::object::ObjectTables;
use crate::objects::{
    ObjectSim, Staging, B_DEF, B_FLAGS, B_FSTATE, B_PHASE, B_SCRX, B_SCRY, B_SPRITE, B_STATE,
    B_T28, B_TYPE,
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
