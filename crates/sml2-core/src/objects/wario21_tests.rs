use super::*;
use crate::divider::Divider;

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
    let mut st = Staging {
        b: std::array::from_fn(|i| i as u8 + 0x30),
        px: 0x1234,
        py: 0x5678,
    };
    st.b[B_FSTATE] = state;
    st.b[B_PHASE] = 0;
    st.b[B_T25] = 0;
    (sim, st)
}

fn expect_step(sim: &mut ObjectSim, st: &mut Staging, writes: &[(usize, u8)]) {
    let mut expected = st.b;
    for &(offset, value) in writes {
        expected[offset] = value;
    }
    sim.handler_wario21(st, &tables());
    assert_eq!(st.b, expected);
    assert_eq!((st.px, st.py), (0x1234, 0x5678));
}

#[test]
fn handler_0x0aaa5_init_only_changes_state() {
    let (mut sim, mut st) = fixture(0);
    sim.boss_transition = 1;
    expect_step(&mut sim, &mut st, &[(B_FSTATE, 1)]);
    assert_eq!(sim.boss_transition, 1);
}

#[test]
fn handler_0x0aaa5_movement_gate_wrap_and_threshold_before_latch() {
    for (dir, x, moved_x, next_dir) in [
        (0, 2, 0xFF, 1),
        (0, 3, 0, 0),
        (0, 0xB3, 0xB0, 1),
        (0, 0xB2, 0xAF, 0),
        (1, 0xAD, 0xB0, 0),
        (1, 0xAC, 0xAF, 1),
        (1, 0xFE, 1, 1),
    ] {
        for transition in [0, 1] {
            let (mut sim, mut st) = fixture(1);
            sim.fff0 = 4;
            sim.boss_transition = transition;
            st.b[B_T25] = dir;
            st.b[B_SCRX] = x;
            let mut writes = vec![(B_SCRX, moved_x), (B_T25, next_dir)];
            if transition != 0 {
                writes.extend([(B_FSTATE, 2), (B_TYPE, 0x40)]);
            }
            expect_step(&mut sim, &mut st, &writes);
            assert_eq!(sim.boss_transition, 0);
        }
    }
    for frame in [1, 2, 3, 0xFF] {
        let (mut sim, mut st) = fixture(1);
        sim.fff0 = frame;
        expect_step(&mut sim, &mut st, &[]);
        sim.boss_transition = 0xA5;
        expect_step(&mut sim, &mut st, &[(B_FSTATE, 2), (B_TYPE, 0x40)]);
        assert_eq!(sim.boss_transition, 0);
    }
}

#[test]
fn handler_0x0aaa5_fall_uses_cap36_and_clears_flash_and_phase() {
    let (mut sim, mut st) = fixture(2);
    sim.fff0 = 4;
    st.b[B_PHASE] = 14;
    st.b[B_SCRY] = 0xFF;
    let flash = st.b[B_FLASH];
    expect_step(
        &mut sim,
        &mut st,
        &[(B_PHASE, 15), (B_SCRY, 2), (B_FLASH, flash ^ 0x10)],
    );
    assert_eq!(st.b[B_FSTATE], 2, "B177's cap is $24, not $0F");
    sim.fff0 = 1;
    st.b[B_PHASE] = 34;
    expect_step(&mut sim, &mut st, &[(B_PHASE, 35), (B_SCRY, 4)]);
    expect_step(
        &mut sim,
        &mut st,
        &[(B_PHASE, 0), (B_SCRY, 7), (B_FLASH, 0), (B_FSTATE, 3)],
    );
    st.b[B_FSTATE] = 2;
    st.b[B_PHASE] = 36;
    expect_step(
        &mut sim,
        &mut st,
        &[(B_PHASE, 0), (B_SCRY, 10), (B_FLASH, 0), (B_FSTATE, 3)],
    );
}

#[test]
fn handler_0x0aaa5_sprite_gate_increment_wrap_and_only_new63_respawns() {
    for frame in [1, 7, 0xFF] {
        let (mut sim, mut st) = fixture(3);
        st.b[B_SPRITE] = 0x62;
        sim.fff0 = frame;
        expect_step(&mut sim, &mut st, &[]);
    }
    for sprite in [0x60, 0x61, 0x63, 0xFF] {
        let (mut sim, mut st) = fixture(3);
        sim.fff0 = 8;
        sim.div = 1;
        st.b[B_SPRITE] = sprite;
        expect_step(&mut sim, &mut st, &[(B_SPRITE, sprite.wrapping_add(1))]);
        assert_eq!(sim.div, 1);
    }
}

#[test]
fn handler_0x0aaa5_real_divider_parity_selects_exact_respawn_bytes() {
    let mut divider = Divider::new(0x00FF);
    for (advance, x, direction) in [(0, 0, 1), (1, 0xA7, 0), (256, 0, 1)] {
        divider.advance_cycles(advance);
        let (mut sim, mut st) = fixture(3);
        sim.fff0 = 8;
        sim.div = divider.sample();
        st.b[B_SPRITE] = 0x62;
        st.b[B_T25] = 0xA3;
        expect_step(
            &mut sim,
            &mut st,
            &[
                (B_SPRITE, 0x60),
                (B_SCRY, 0x38),
                (B_SCRX, x),
                (B_TYPE, 0),
                (B_FSTATE, 0),
                (B_T25, direction),
            ],
        );
    }
}

#[test]
fn handler_0x0aaa5_unknown_state_reports_without_staging_writes() {
    let (mut sim, mut st) = fixture(4);
    expect_step(&mut sim, &mut st, &[]);
    assert_eq!(sim.unported, vec![0xAAA5]);
}

#[test]
fn handler_0x0aaa5_both_anim_dispatch_entries_consume_transition_after_movement() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    for anim in [21, 24] {
        assert_eq!(tables.boss_anim_handler(anim), Some(0xAAA5));
        let mut sim = ObjectSim::new(Vec::new(), 0);
        sim.fff0 = 3; // The boss engine increments before dispatch.
        sim.boss_transition = 1;
        sim.slots[0].b[B_DEF] = anim;
        sim.slots[0].b[B_FSTATE] = 1;
        sim.slots[0].b[B_SCRX] = 2;
        sim.boss_engine(80, 72, &layout, &tables);
        assert_eq!(
            (
                sim.slots[0].b[B_SCRX],
                sim.slots[0].b[B_T25],
                sim.slots[0].b[B_FSTATE],
                sim.slots[0].b[B_TYPE]
            ),
            (0xFF, 1, 2, 0x40)
        );
        assert_eq!(sim.boss_transition, 0);
        assert!(!sim.unported.contains(&0xAAA5));
    }
}

#[test]
fn handler_0x0aaa5_spawned_child_consumes_wario20_impact_in_same_boss_pass() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(Vec::new(), 0);
    sim.spawn_boss(6);
    assert_eq!((sim.slots[0].b[B_DEF], sim.slots[1].b[B_DEF]), (20, 21));
    assert_eq!(sim.slots[1].b[B_SCRY], 0x38);
    sim.slots[0].b[B_FSTATE] = 3;
    sim.slots[0].b[B_T24] = 1;
    sim.slots[0].b[B_PHASE] = 2;
    sim.slots[0].b[B_SCRY] = 0x89;
    sim.slots[1].b[B_FSTATE] = 1;
    sim.boss_engine(80, 72, &layout, &tables);
    assert_eq!(
        (
            sim.slots[0].b[B_FSTATE],
            sim.slots[0].b[B_T28],
            sim.slots[0].b[B_PHASE],
            sim.slots[0].b[B_T24]
        ),
        (9, 0x30, 0, 0)
    );
    assert_eq!(
        (sim.camera_pan_set, sim.sfx0, sim.boss_transition),
        (Some(0x34), 0x18, 0)
    );
    assert_eq!(
        (
            sim.slots[1].b[B_FSTATE],
            sim.slots[1].b[B_TYPE],
            sim.slots[1].b[B_PHASE]
        ),
        (2, 0x40, 0)
    );
    for phase in 1..=35 {
        sim.boss_engine(80, 72, &layout, &tables);
        assert_eq!(
            (sim.slots[1].b[B_FSTATE], sim.slots[1].b[B_PHASE]),
            (2, phase)
        );
    }
    // Independent sum of ROM $B1A3..$B1C6 (the first 36 profile bytes) is 80.
    assert_eq!(sim.slots[1].b[B_SCRY], 0x85);
    sim.boss_engine(80, 72, &layout, &tables);
    assert_eq!(
        (
            sim.slots[1].b[B_FSTATE],
            sim.slots[1].b[B_PHASE],
            sim.slots[1].b[B_FLASH],
            sim.slots[1].b[B_SCRY]
        ),
        (3, 0, 0, 0x88)
    );
    assert_eq!((sim.slots[0].b[B_FSTATE], sim.slots[0].b[B_T28]), (9, 0x0C));
    assert!(sim.unported.is_empty());
}
