//! Bullet Bill reset gates: native ROM bytes and enlarged-view world bounds.
use super::*;
use crate::actor::Position;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn fixture(camera: u16, x: u16, state: u8) -> (ObjectSim, Staging) {
    // Mario Zone 3's first Bullet Bill: literal level13 spawn stream bytes.
    let record = SpawnRecord {
        x: 544,
        y: 464,
        anim_index: 83,
        mode_flag: 1,
        sprite_id: 175,
        hitbox: [-11, -6, -10, 10],
        raw: [2, 32, 1, 1, 208, 83],
    };
    let mut sim = ObjectSim::new(vec![record], camera);
    sim.configure(12);
    sim.camera_position = Position { x: camera, y: 448 };
    let mut raw = [0; 32];
    raw[0..2].copy_from_slice(&x.to_be_bytes());
    raw[2..8].copy_from_slice(&[1, 1, 210, 83, 0xAB, 8]);
    raw[8] = 2;
    raw[10] = x.wrapping_sub(camera).wrapping_add(88) as u8;
    raw[11] = 175;
    raw[15] = state;
    raw[18] = 74;
    (
        sim,
        Staging {
            b: raw,
            ..Default::default()
        },
    )
}

fn expect_reset(before: [u8; 32], sprite: u8) -> [u8; 32] {
    let mut expected = before;
    expected[..5].copy_from_slice(&[2, 32, 1, 1, 208]);
    expected[8] = 1;
    expected[11] = sprite;
    expected[14..31].fill(0);
    expected
}

#[test]
fn bullet_bill_native_reset_preserves_the_rom_screen_x_gate() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&vec![0; crate::level::LAYOUT_SIZE]).unwrap();
    for state in [3, 4] {
        for screen_x in [0, 40, 183, 184, 255] {
            let (mut sim, mut st) = fixture(1024, 1136, state);
            st.b[10] = screen_x;
            let mut expected = st.b;
            if screen_x >= 184 {
                expected = expect_reset(expected, 175);
            } else {
                let x = if state == 3 { 1133u16 } else { 1139u16 };
                expected[..2].copy_from_slice(&x.to_be_bytes());
            }
            sim.dispatch_handler(0x5A9E8, &mut st, &layout, &tables);
            assert_eq!(st.b, expected, "state {state}, screen X {screen_x}");
            assert_eq!(sim.done, u8::from(screen_x >= 184));
            assert!(sim.unported.is_empty());
        }
    }
}

#[test]
fn bullet_bill_crosses_native_edges_and_wraps_until_leaving_the_enlarged_view() {
    let tables = tables();
    for state in [3, 4] {
        for (camera, margin, offset, x, reset) in [
            (1024, 112, 0, 823, true),
            (1024, 112, 0, 824, false),
            (1024, 112, 0, 935, false),  // original left edge: OAM X 255
            (1024, 112, 0, 1120, false), // original right edge: OAM X 184
            (1024, 112, 0, 1192, false), // OAM X has wrapped back to zero
            (1024, 112, 0, 1231, false),
            (1024, 112, 0, 1232, true),
            (1024, 176, 0, 1295, false),
            (1024, 176, 0, 1296, true),
            (80, 112, 112, 0, false), // camera parked at the level start
            (80, 112, 112, 399, false),
            (80, 112, 112, 400, true),
            (4016, 112, -112, 3703, true), // camera parked at the level end
            (4016, 112, -112, 3704, false),
        ] {
            let (mut sim, mut st) = fixture(camera, x, state);
            sim.view_margin = (margin, 36);
            sim.view_offset = (offset, 0);
            let mut expected = st.b;
            if reset {
                expected = expect_reset(expected, 175);
            } else {
                let x = x.wrapping_add_signed(if state == 3 { -3 } else { 3 });
                expected[..2].copy_from_slice(&x.to_be_bytes());
            }
            sim.handler_charging_enemy(&mut st, &tables);
            assert_eq!(st.b, expected, "state {state}, camera {camera}, X {x}");
            assert_eq!(sim.done, u8::from(reset));
            assert!(sim.unported.is_empty());
        }
    }
}

#[test]
fn bullet_bill_launch_distance_does_not_alias_every_256_pixels_in_enlarged_views() {
    let tables = tables();
    for enlarged in [false, true] {
        for (mario_x, launches) in [(985, false), (984, true), (768, enlarged), (1280, enlarged)] {
            let (mut sim, mut st) = fixture(1024, 1024, 5);
            sim.mario_ref_x = mario_x;
            if enlarged {
                sim.view_margin = (112, 36);
            }
            let mut expected = st.b;
            let facing_right = mario_x > 1024;
            if launches {
                expected[15] = if facing_right { 2 } else { 1 };
                expected[13] = if facing_right { 0x20 } else { 0 };
            }
            sim.handler_charging_enemy(&mut st, &tables);
            assert_eq!(st.b, expected, "Mario X {mario_x}, enlarged {enlarged}");
            assert_eq!(
                sim.facing,
                if !launches {
                    0
                } else if facing_right {
                    1
                } else {
                    255
                }
            );
        }
    }
}

#[test]
fn bullet_bill_flies_through_the_full_view_before_returning_to_its_launcher() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&vec![0; crate::level::LAYOUT_SIZE]).unwrap();
    let (mut sim, st) = fixture(544, 544, 4);
    sim.view_margin = (112, 36);
    sim.slots[0].b = st.b;
    sim.active = 1;
    sim.phase = true; // slot zero updates on the next even phase
    sim.spawned[0] = true;
    let mario = MarioContact {
        travelling: true,
        ..Default::default()
    };
    for x in (544u16..752).step_by(3) {
        assert_eq!(sim.slots[0].x(), x);
        assert_eq!(sim.slots[0].b[8], 2);
        assert_eq!(sim.slots[0].b[15], 4);
        // The source updates each established slot on alternate engine passes.
        for _ in 0..2 {
            sim.step(544, 448, &layout, &tables, &mario, false);
            assert!(!sim.slot_done[0], "must keep flying at X {x}");
        }
    }
    sim.step(544, 448, &layout, &tables, &mario, false);
    assert_eq!(&sim.slots[0].b[..5], &[2, 32, 1, 1, 208]);
    assert_eq!(sim.slots[0].b[8], 1);
    assert_eq!(sim.slots[0].b[15], 0);
    assert!(sim.slot_done[0]);
    assert!(sim.unported.is_empty());
}
