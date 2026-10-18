use crate::level::{LevelLayout, LAYOUT_SIZE};
use crate::objects::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn handler_0x587a1_exact_slot_branches_cadence_and_wrapping() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    for frame in [0, 1, 2, 3, 254, 255] {
        for state in [0, 1, 2, 255] {
            for y in [0u16, 0x0037, 0x0038, 0x0100, 0x12FF, 0xFFFF] {
                let mut sim = ObjectSim::new(Vec::new(), 0x3333);
                sim.fff0 = frame;
                sim.facing = 0xC7;
                sim.move_flags = 0xD9;
                sim.remove_mode = 0x6B;
                sim.money_bag = 0x99;
                sim.sfx0 = 0x71;
                sim.sfx1 = 0x72;
                sim.sfx2 = 0x73;
                let globals_before = format!("{sim:?}");
                let mut st = Staging {
                    b: std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5),
                    px: 0x1234,
                    py: 0x5678,
                };
                set_y(&mut st.b, y);
                st.b[B_FSTATE] = state;
                let mut expected = st.b;
                if frame & 1 == 0 {
                    expected[B_FLASH] ^= 0x10;
                }
                set_y(
                    &mut expected,
                    if state == 0 {
                        y.wrapping_sub(56)
                    } else {
                        y.wrapping_add(1)
                    },
                );
                if state == 0 {
                    expected[B_TYPE] = 132;
                    expected[B_FSTATE] = 1;
                }
                sim.dispatch_handler(0x587A1, &mut st, &layout, &tables);
                assert_eq!(st.b, expected, "frame={frame} state={state} y={y:#06x}");
                assert_eq!((st.px, st.py), (0x1234, 0x5678));
                assert_eq!(format!("{sim:?}"), globals_before);
            }
        }
    }
}

#[test]
fn handler_0x587a1_real_step_reward_production_promotion_init_and_fall_both_modes() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    for mode in [0, 1] {
        let mut sim = ObjectSim::new(Vec::new(), 0x3333);
        sim.configure(if mode == 0 { 0 } else { 0x0A });
        assert_eq!(sim.sprite_map_index, mode);
        sim.money_bag = 0x99;
        sim.active = 1;
        let slot = &mut sim.slots[8].b;
        set_x(slot, 0x123C);
        set_y(slot, 0x5680);
        slot[B_STATE] = 2;
        slot[B_DEF] = 10;
        slot[B_FSTATE] = 1;
        slot[B_PHASE] = 1;
        sim.slots[0].b[B_MARKER] = 0x73;
        let mario = MarioContact {
            travelling: true,
            ..MarioContact::default()
        };
        sim.step(0x1234, 0x5678, &layout, &tables, &mario, false);
        assert_eq!(sim.money_bag, 0, "mode={mode}");
        assert_eq!(sim.sfx0, 8);
        assert_eq!(sim.slots[8].b[B_STATE], 0);
        assert_eq!(
            &sim.slots[0].b[..9],
            &[0x12, 0x34, 0x73, 0x56, 0x78, 26, 0xAE, 0, 1]
        );
        assert_eq!(sim.slots[0].b[B_SPRITE], 28);
        assert_eq!(sim.standby, 1);
        assert!(sim.unported.is_empty());

        // Activation is owned by the real engine's odd/even scheduling.
        for _ in 0..3 {
            sim.step(0x1234, 0x5678, &layout, &tables, &mario, false);
            assert!(sim.unported.is_empty());
        }
        assert_eq!((sim.standby, sim.active), (0, 1));
        assert_eq!(sim.slots[0].b[B_STATE], 2);
        assert_eq!(sim.slots[0].b[B_FSTATE], 0);
        assert_eq!(y_of(&sim.slots[0].b), 0x5678);

        sim.step(0x1234, 0x5678, &layout, &tables, &mario, false);
        assert!(sim.unported.is_empty());
        assert_eq!(sim.slots[0].b[B_FSTATE], 1);
        assert_eq!(sim.slots[0].b[B_TYPE], 132);
        assert_eq!(y_of(&sim.slots[0].b), 0x5640);
        assert_eq!(x_of(&sim.slots[0].b), 0x1234);
        let initialized = sim.slots[0].b;
        for _ in 0..2 {
            sim.step(0x1234, 0x5678, &layout, &tables, &mario, false);
            assert!(sim.unported.is_empty());
        }
        assert_eq!(y_of(&sim.slots[0].b), 0x5641);
        assert_eq!(sim.slots[0].b[B_FSTATE], 1);
        assert_eq!(sim.slots[0].b[B_TYPE], 132);
        assert_eq!(sim.slots[0].b[B_FLIP], initialized[B_FLIP]);
        assert_eq!(sim.slots[0].b[B_STATE], 2);
        // The real contact pass stages star item 4 from this handler's type 132.
        let pickup = MarioContact {
            x: x_of(&sim.slots[0].b),
            y: y_of(&sim.slots[0].b).wrapping_sub(16),
            ..MarioContact::default()
        };
        sim.step(0x1234, 0x5678, &layout, &tables, &pickup, false);
        assert_eq!(sim.pending_item, 4);
        assert!(sim.unported.is_empty());
        let mut player = crate::player_sim::PlayerSim::new();
        player.x = pickup.x;
        player.y = pickup.y;
        player.apply_object_result(&mut sim, false);
        assert_eq!(sim.pending_item, 0);
        assert_eq!(player.pending_item, 4);
        player.step(
            &crate::player_sim::PlayerKeys::default(),
            false,
            false,
            &layout,
        );
        assert_eq!(player.pending_item, 0);
        assert_eq!((player.star_high, player.star_low), (4, 64));
        assert_eq!(player.music_request, 4);
    }
}
