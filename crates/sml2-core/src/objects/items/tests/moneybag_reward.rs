use crate::actor::Position;
use crate::objects::*;

#[test]
fn helper_0x5b346_first_free_exact_bytes_and_count_wrap() {
    for free in [0, 3, 8, 15] {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        for (i, slot) in sim.slots.iter_mut().enumerate() {
            slot.b = std::array::from_fn(|byte| (i * 13 + byte * 7) as u8);
            slot.b[B_STATE] = if i < free { 2 } else { 0 };
        }
        let before = sim.slots.clone();
        sim.camera_position = Position {
            x: 0x1234,
            y: 0x5678,
        };
        sim.prev_cam_x = 0x9999;
        sim.slot_ptr_hi = if free == 0 { 0 } else { 0xAE };
        sim.boss_slot_ptr_lo = 0xB9;
        sim.standby = 255;
        sim.spawn_anim = 26;
        sim.spawn_sprite = 28;
        sim.spawn_attr = 0xA5;
        assert_eq!(sim.spawn_reward_from_camera(), Some(free));
        let mut expected = before[free].b;
        expected[0..2].copy_from_slice(&[0x12, 0x34]);
        expected[3..9].copy_from_slice(&[0x56, 0x78, 26, sim.slot_ptr_hi, 0xB0, 1]);
        expected[B_SPRITE] = 28;
        expected[B_FLIP] = 0xA5;
        for i in 0..SLOT_COUNT {
            assert_eq!(
                sim.slots[i].b,
                if i == free { expected } else { before[i].b }
            );
        }
        assert_eq!(sim.standby, 0);
        assert_eq!(
            (sim.spawn_anim, sim.spawn_sprite, sim.spawn_attr),
            (0, 0, 0)
        );
    }
}

#[test]
fn helper_0x5b31d_overflow_nonoverflow_invalid_decimal_and_capacity_sound() {
    for (initial, expected, reward) in [(0x98, 0x99, false), (0x99, 0, true), (0xF9, 0x60, false)] {
        for full in [false, true] {
            let mut sim = ObjectSim::new(Vec::new(), 0);
            sim.money_bag = initial;
            sim.sfx0 = 0x41;
            sim.spawn_anim = 7;
            sim.spawn_sprite = 9;
            sim.spawn_attr = 0x65;
            sim.standby = 19;
            for slot in &mut sim.slots {
                slot.b[B_STATE] = u8::from(full) * 2;
            }
            let before = sim.slots.clone();
            sim.add_money_bag_reward();
            assert_eq!(sim.money_bag, expected);
            assert_eq!(sim.sfx0, if reward { 8 } else { 0x41 });
            assert_eq!(sim.standby, if reward && !full { 20 } else { 19 });
            if reward {
                assert_eq!(
                    (sim.spawn_anim, sim.spawn_sprite, sim.spawn_attr),
                    (0, 0, 0)
                );
                if !full {
                    assert_eq!(sim.slots[0].b[B_DEF], 26);
                    assert_eq!(sim.slots[0].b[B_SPRITE], 28);
                    assert_eq!(sim.slots[0].b[B_FLIP], 0x65);
                }
            } else {
                assert_eq!(
                    (sim.spawn_anim, sim.spawn_sprite, sim.spawn_attr),
                    (7, 9, 0x65)
                );
            }
            if full || !reward {
                for i in 0..SLOT_COUNT {
                    assert_eq!(sim.slots[i].b, before[i].b);
                }
            }
        }
    }
}

#[test]
fn helper_0x5b31d_step_squashed_expiry_produces_camera_reward_then_promotes() {
    use crate::level::{LevelLayout, LAYOUT_SIZE};
    use crate::object::AnimProfile;
    let mut tables = tables();
    tables.anim_profiles_a[1] = AnimProfile {
        handler: 0x582A9,
        type_sets: Vec::new(),
        states: Vec::new(),
    };
    // This test proves production and activation; the reward consumer has its own ticket.
    tables.anim_profiles_a[26].handler = 0x58100;
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    let mut sim = ObjectSim::new(Vec::new(), 0x3333);
    sim.money_bag = 0x99;
    sim.active = 1;
    let slot = &mut sim.slots[8].b;
    set_x(slot, 0x123C);
    set_y(slot, 0x5680);
    slot[B_STATE] = 2;
    slot[B_DEF] = 1;
    slot[B_FSTATE] = 1;
    slot[B_PHASE] = 1;
    sim.slots[0].b[B_MARKER] = 0x73;
    let mario = MarioContact {
        travelling: true,
        ..MarioContact::default()
    };
    sim.step(0x1234, 0x5678, &layout, &tables, &mario, false);
    assert_eq!(sim.money_bag, 0);
    assert_eq!(sim.sfx0, 8);
    assert_eq!(sim.slots[8].b[B_STATE], 0);
    assert_eq!(
        &sim.slots[0].b[..9],
        &[0x12, 0x34, 0x73, 0x56, 0x78, 26, 0xAE, 0, 1]
    );
    assert_eq!(sim.slots[0].b[B_SPRITE], 28);
    assert_eq!(sim.standby, 1);
    // The engine increments its odd/even clock every second phase.
    for _ in 0..3 {
        sim.step(0x1234, 0x5678, &layout, &tables, &mario, false);
    }
    assert_eq!(sim.slots[0].b[B_STATE], 2);
    assert_eq!((sim.standby, sim.active), (0, 1));
    assert!(sim.unported.is_empty());
}

fn tables() -> ObjectTables {
    ObjectTables {
        schema: String::new(),
        notes: Vec::new(),
        anim_to_sprite_a: vec![0; 96],
        anim_to_sprite_b: Vec::new(),
        hitbox_records_a: vec![[0; 4]],
        hitbox_records_b: Vec::new(),
        hitbox_index_a: vec![0; 96],
        hitbox_index_b: Vec::new(),
        boss_anim_handlers: Vec::new(),
        boss_picker_profile: Vec::new(),
        boss_picker_profile_15: Vec::new(),
        boss_sweep_profile: Vec::new(),
        draw_records: Vec::new(),
        draw_index: Vec::new(),
        sprite_mappings_a: Vec::new(),
        sprite_mappings_b: Vec::new(),
        anim_profiles_a: vec![
            crate::object::AnimProfile {
                handler: 0,
                type_sets: Vec::new(),
                states: Vec::new()
            };
            96
        ],
        anim_profiles_b: Vec::new(),
        player_mappings: Vec::new(),
        // 118 empty lists: the boss render pass must find a list for any
        // sprite id a def installs, so the synthetic table keeps the shape
        // of the real one (see `boss_oam_tables` for a populated one).
        boss_oam_lists: vec![vec![0x80]; 118],
    }
}
