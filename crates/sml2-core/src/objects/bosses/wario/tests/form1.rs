use crate::objects::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(Vec::new(), 0);
    sim.fff0 = 1;
    sim.boss_def_index = 6;
    sim.boss_hits_remaining = 8;
    let mut st = Staging {
        b: std::array::from_fn(|i| i as u8 + 0x30),
        px: 0x1234,
        py: 0x5678,
    };
    st.b[B_FSTATE] = state;
    for offset in [B_FLAGS, B_PHASE, B_T24, B_T25, B_ANIMFRAME] {
        st.b[offset] = 0;
    }
    (sim, st)
}

fn expect_step(sim: &mut ObjectSim, st: &mut Staging, writes: &[(usize, u8)]) {
    let mut expected = st.b;
    for &(offset, byte) in writes {
        expected[offset] = byte;
    }
    sim.handler_wario20(st, &tables());
    assert_eq!(st.b, expected);
    assert_eq!((st.px, st.py), (0x1234, 0x5678));
}

#[test]
fn handler_0x0a8ab_intro_low_byte_trigger_and_full_fade_gate() {
    let (mut sim, mut st) = fixture(0);
    sim.fade_timer = 0x0161;
    sim.fff0 = 8;
    let sprite = st.b[B_SPRITE];
    expect_step(&mut sim, &mut st, &[(B_SPRITE, sprite ^ 1)]);
    assert_eq!((sim.ending_door_active, sim.sfx1), (1, 0x0F));
    sim.fade_timer = 0x0100;
    sim.fff0 = 1;
    expect_step(&mut sim, &mut st, &[]);
    sim.fade_timer = 0;
    expect_step(
        &mut sim,
        &mut st,
        &[(B_FSTATE, 3), (B_TYPE, 0x41), (B_SPRITE, 0x4F)],
    );
    assert_eq!(sim.ending_door_active, 0);
    let (mut sim, mut st) = fixture(0);
    sim.fade_timer = 0x60;
    sim.fff0 = 8;
    expect_step(&mut sim, &mut st, &[]);
    assert_eq!((sim.ending_door_active, sim.sfx1), (0, 0));
}

#[test]
fn handler_0x0a8ab_hits_snapshot_before_reaction_and_state1_restores() {
    for stomp in [false, true] {
        let (mut sim, mut st) = fixture(4);
        st.b[B_FLAGS] = if stomp { 1 } else { 2 };
        sim.boss_timer_reload = 0x23;
        let (sprite, ty, timer) = (st.b[B_SPRITE], st.b[B_TYPE], st.b[B_T28]);
        let mut writes = vec![(B_FSTATE, 1), (B_TYPE, 0)];
        if stomp {
            writes.extend([(B_SPRITE, 0x52), (B_T28, 0x23)]);
        }
        expect_step(&mut sim, &mut st, &writes);
        assert_eq!(
            (sim.spawn_sprite, sim.saved_frame_state, sim.saved_type),
            (sprite, 4, ty)
        );
        assert_eq!(sim.boss_hits_remaining, if stomp { 6 } else { 7 });
        assert_eq!((sim.done, sim.boss_hit_latch), (1, u8::from(stomp)));
        if !stomp {
            assert_eq!(st.b[B_T28], timer);
        }
        sim.done = 0;
        st.b[B_FLAGS] = 0;
        st.b[B_T28] = 1;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_T28, 0),
                (B_FLASH, 0),
                (B_SPRITE, sprite),
                (B_FSTATE, 4),
                (B_TYPE, ty),
            ],
        );
    }
    let (mut sim, mut st) = fixture(1);
    sim.remove_mode = 1;
    sim.boss_sprite = 0x49;
    st.b[B_T28] = 0;
    expect_step(
        &mut sim,
        &mut st,
        &[(B_FSTATE, 2), (B_T28, 0x20), (B_SPRITE, 0x49)],
    );
}

#[test]
fn handler_0x0a8ab_state2_fall_threshold_and_state6_timer_wrap() {
    let (mut sim, mut st) = fixture(2);
    st.b[B_PHASE] = 2;
    st.b[B_SCRY] = 0x88;
    expect_step(&mut sim, &mut st, &[(B_PHASE, 3), (B_SCRY, 0x89)]);
    expect_step(
        &mut sim,
        &mut st,
        &[(B_PHASE, 4), (B_SCRY, 0x8A), (B_FSTATE, 6), (B_T28, 0x80)],
    );
    assert_eq!(sim.camera_pan_set, None);
    sim.fff0 = 4;
    st.b[B_T28] = 0;
    let flash = st.b[B_FLASH];
    expect_step(&mut sim, &mut st, &[(B_T28, 0xFF), (B_FLASH, flash ^ 0x10)]);
    st.b[B_T28] = 1;
    st.b[B_ANIMFRAME] = 3;
    expect_step(
        &mut sim,
        &mut st,
        &[
            (B_T28, 0),
            (B_ANIMFRAME, 0),
            (B_FLASH, 0),
            (B_FSTATE, 8),
            (B_SPRITE, 0x4C),
            (B_FLIP, 0x20),
        ],
    );
}

#[test]
fn handler_0x0a8ab_state3_rise_then_exact_fall_impact34() {
    let (mut sim, mut st) = fixture(3);
    st.b[B_SCRY] = 1;
    expect_step(&mut sim, &mut st, &[(B_PHASE, 1), (B_SCRY, 0xFE)]);
    assert_eq!(sim.camera_pan_set, None);
    st.b[B_PHASE] = 11;
    expect_step(&mut sim, &mut st, &[(B_PHASE, 0), (B_T24, 1)]);
    st.b[B_PHASE] = 2;
    st.b[B_SCRY] = 0x88;
    expect_step(&mut sim, &mut st, &[(B_PHASE, 3), (B_SCRY, 0x89)]);
    assert_eq!(
        (sim.camera_pan_set, sim.boss_transition, sim.sfx0),
        (None, 0, 0)
    );
    expect_step(
        &mut sim,
        &mut st,
        &[
            (B_PHASE, 0),
            (B_T24, 0),
            (B_SCRY, 0x8A),
            (B_FSTATE, 9),
            (B_T28, 0x30),
        ],
    );
    assert_eq!(
        (sim.camera_pan_set, sim.boss_transition, sim.sfx0),
        (Some(0x34), 1, 0x18)
    );
}

#[test]
fn handler_0x0a8ab_state4_timer_gate_and_horizontal_edges() {
    for timer in [0, 8, 0xF8] {
        let (mut sim, mut st) = fixture(4);
        sim.timer_low = timer;
        sim.fff0 = 4;
        expect_step(&mut sim, &mut st, &[(B_FSTATE, 5), (B_SPRITE, 0x4F)]);
    }
    for (dir, x, next_x, crosses) in [
        (0, 0x12, 0x10, false),
        (0, 0x11, 0x0F, true),
        (1, 0x95, 0x97, false),
        (1, 0x96, 0x98, true),
        (0, 0, 0xFE, false),
        (1, 0xFF, 1, false),
    ] {
        let (mut sim, mut st) = fixture(4);
        sim.timer_low = 1;
        sim.fff0 = 4;
        st.b[B_T25] = dir;
        st.b[B_SCRX] = x;
        let flip = st.b[B_FLIP];
        let sprite = st.b[B_SPRITE];
        let mut writes = vec![
            (B_SCRX, next_x),
            (B_ANIMFRAME, 1),
            (B_SPRITE, sprite.wrapping_add(1)),
        ];
        if crosses {
            writes.extend([(B_FLIP, flip ^ 0x20), (B_T25, dir ^ 1), (B_T28, 8)]);
        }
        expect_step(&mut sim, &mut st, &writes);
        assert_eq!(sim.camera_pan_set, None);
    }
}

#[test]
fn handler_0x0a8ab_state5_impact38_requires_completed_down_leg() {
    let (mut sim, mut st) = fixture(5);
    st.b[B_PHASE] = 20;
    expect_step(&mut sim, &mut st, &[(B_PHASE, 0), (B_T24, 1)]);
    assert_eq!(sim.camera_pan_set, None);
    st.b[B_PHASE] = 19;
    st.b[B_SCRY] = 0x80;
    expect_step(&mut sim, &mut st, &[(B_PHASE, 20), (B_SCRY, 0x82)]);
    assert_eq!(
        (sim.camera_pan_set, sim.boss_transition, sim.sfx0),
        (None, 0, 0)
    );
    expect_step(
        &mut sim,
        &mut st,
        &[
            (B_PHASE, 0),
            (B_T24, 0),
            (B_SCRY, 0x85),
            (B_FSTATE, 9),
            (B_T28, 0x38),
        ],
    );
    assert_eq!(
        (sim.camera_pan_set, sim.boss_transition, sim.sfx0),
        (Some(0x38), 1, 0x18)
    );
}

#[test]
fn handler_0x0a8ab_state7_delay_gate_and_unknown_state11() {
    let (mut sim, mut st) = fixture(7);
    sim.fff0 = 8;
    st.b[B_T28] = 1;
    expect_step(&mut sim, &mut st, &[]);
    sim.boss_delay = 1;
    st.b[B_T28] = 0;
    let sprite = st.b[B_SPRITE];
    expect_step(
        &mut sim,
        &mut st,
        &[(B_T28, 0xFF), (B_SPRITE, sprite ^ 0x1F)],
    );
    sim.fff0 = 1;
    st.b[B_T28] = 1;
    expect_step(
        &mut sim,
        &mut st,
        &[(B_T28, 0), (B_FSTATE, 11), (B_SPRITE, 0x72)],
    );
    assert_eq!(sim.boss_delay, 0);
    expect_step(&mut sim, &mut st, &[]);
    assert_eq!(sim.unported, vec![0xA8AB]);
}

#[test]
fn handler_0x0a8ab_state8_uses_verified_phase_exit() {
    let (mut sim, mut st) = fixture(8);
    st.b[B_SCRX] = 0xBE;
    sim.fade_timer = 0x1200;
    expect_step(
        &mut sim,
        &mut st,
        &(0..32).map(|i| (i, 0)).collect::<Vec<_>>(),
    );
    assert_eq!(
        (sim.done, sim.scene_phase_set, sim.fade_timer_set),
        (1, Some(1), Some(0x1205))
    );
    assert_eq!(sim.records[0].raw, [7, 0xC8, 0, 0, 0xB0, 0x1D]);
}

#[test]
fn handler_0x0a8ab_state9_wait_then_two_picker_legs() {
    let (mut sim, mut st) = fixture(9);
    st.b[B_T28] = 1;
    st.b[B_PHASE] = 11;
    expect_step(&mut sim, &mut st, &[(B_T28, 0)]);
    expect_step(
        &mut sim,
        &mut st,
        &[(B_PHASE, 0), (B_ANIMFRAME, 0), (B_SPRITE, 0x4C), (B_T24, 1)],
    );
    st.b[B_PHASE] = 11;
    st.b[B_SCRY] = 0xFF;
    expect_step(
        &mut sim,
        &mut st,
        &[
            (B_PHASE, 0),
            (B_ANIMFRAME, 0),
            (B_SPRITE, 0x4C),
            (B_T24, 0),
            (B_FSTATE, 4),
            (B_SCRY, 2),
        ],
    );
    assert_eq!(sim.camera_pan_set, None);
}
