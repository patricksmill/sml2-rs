use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn staged(anim: u8, ty: u8) -> Staging {
    let mut st = Staging {
        b: std::array::from_fn(|i| 0x40 + i as u8),
        px: 0x1234,
        py: 0x5678,
    };
    st.b[B_DEF] = anim;
    st.b[B_RECHI] = 0xAB;
    st.b[B_TYPE] = ty;
    st.b[B_FLAGS] = 1;
    st
}

#[test]
fn stomp_morph_dispatch_both_source_entries_request_channel_zero_and_reset_slot() {
    let tables = tables();
    for (anim, ty, next, sprite) in [(53, 0x52, 54, 110), (54, 0x53, 53, 108)] {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        (sim.sfx0, sim.sfx1, sim.sfx2) = (0x71, 0x82, 0x93);
        let mut st = staged(anim, ty);
        let mut expected = st.b;
        expected[B_DEF] = next;
        expected[B_SPRITE] = sprite;
        expected[14..31].fill(0);

        sim.interaction_dispatch(&mut st, &tables);

        assert_eq!(st.b, expected, "source morph for type {ty:02x}");
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert_eq!((sim.sfx0, sim.sfx1, sim.sfx2), (3, 0x82, 0x93));
        assert_eq!(sim.done, 1);
        assert_eq!(sim.money_bag, 0);
        assert!(!sim.clear_stage_flags);
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn stomp_morph_defeat_gates_preserve_sound_except_moneybag_overflow() {
    let tables = tables();
    for (spin, ty) in [(true, 0x52), (false, 0x50)] {
        for overflow in [false, true] {
            let mut sim = ObjectSim::new(Vec::new(), 0);
            sim.mario.spin_jump = spin;
            sim.money_bag = if overflow { 0x99 } else { 0x12 };
            (sim.sfx0, sim.sfx1, sim.sfx2) = (0x71, 0x82, 0x93);
            let mut st = staged(53, ty);
            let mut expected = st.b;
            expected[B_DEF] = 0;
            expected[B_FSTATE] = 0;
            expected[B_DEFEAT] = 0;
            expected[B_TYPE] = 0;

            sim.interaction_dispatch(&mut st, &tables);

            assert_eq!(st.b, expected);
            assert_eq!((st.px, st.py), (0x1234, 0x5678));
            assert_eq!(
                (sim.sfx0, sim.sfx1, sim.sfx2),
                (if overflow { 8 } else { 0x71 }, 0x82, 0x93)
            );
            assert_eq!(sim.money_bag, if overflow { 0 } else { 0x13 });
            assert_eq!(sim.done, 1);
            assert!(sim.clear_stage_flags);
        }
    }
}

#[test]
fn stomp_morph_step_collision_produces_anim54_and_delays_its_handler() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(Vec::new(), 0);
    sim.active = 1;
    (sim.sfx0, sim.sfx1, sim.sfx2) = (0x71, 0x82, 0x93);
    let mut initial = staged(53, 0x52).b;
    set_x(&mut initial, 0x108);
    set_y(&mut initial, 0x110);
    initial[B_STATE] = 2;
    initial[B_FLAGS] = 0;
    initial[B_SCRX] = 0x60;
    initial[B_SCRY] = 0x70;
    initial[B_HB_TOP..=B_HB_RIGHT].fill(0);
    sim.slots[0].b = initial;
    let mario = MarioContact {
        x: 0x100,
        y: 0x100,
        ..MarioContact::default()
    };
    let mut expected = initial;
    expected[B_DEF] = 54;
    expected[B_SPRITE] = 110;
    expected[14..31].fill(0);
    expected[B_SCRX] = 0x60;
    expected[B_SCRY] = 0x68;
    expected[B_HB_TOP..=B_HB_RIGHT].copy_from_slice(&[0xF3, 0xFD, 0xFA, 6]);

    sim.step(0x100, 0x100, &layout, &tables, &mario, false);

    assert!(
        sim.bounce && sim.stomp_jump,
        "collision must generate the stomp"
    );
    assert_eq!(sim.slots[0].b, expected);
    assert_eq!((sim.sfx0, sim.sfx1, sim.sfx2), (3, 0x82, 0x93));
    assert!(
        sim.slot_done[0],
        "morph ends this slot's frame before anim54 runs"
    );
    assert_eq!(sim.money_bag, 0);
    assert!(sim.unported.is_empty());
}
