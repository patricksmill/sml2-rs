use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.facing = 99;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.b[23] &= 0x0F;
    (sim, st)
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5a355_init_rises_36_and_picks_a_curve() {
    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[13] = 0;
    sim.mario_ref_x = 200;
    sim.handler_0x5a355(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 64);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(st.b[15], 0x10);
    assert_eq!(st.b[18], 72);
    assert_eq!(st.b[25], 20);
    assert_eq!(st.b[24], 1);
    assert_eq!(sim.facing, 1);

    let (mut sim, mut st) = fixture(0);
    sim.fff0 = 1;
    position(&mut st.b, 100, 100);
    st.b[13] = 0x20;
    sim.mario_ref_x = 40;
    sim.handler_0x5a355(&mut st);
    assert_eq!(st.b[13], 0);
    assert_eq!(st.b[15], 0x12);
    assert_eq!(st.b[25], 1);
    assert_eq!(sim.facing, 255);
}

#[test]
fn handler_0x5a355_even_frame_steps_both_axes_and_carries() {
    let (mut sim, mut st) = fixture(0x10);
    sim.fff0 = 0;
    sim.mario.x = 10;
    sim.mario.y = 20;
    position(&mut st.b, 100, 100);
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[25] = 20;
    st.b[24] = 1;
    sim.handler_0x5a355(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 103);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 101);
    assert_eq!(st.b[25], 19);
    assert_eq!(st.b[24], 2);
    assert_eq!(sim.mario_x_adjust, Some(13));
    assert_eq!(sim.mario_y_adjust, Some(21));
}

#[test]
fn handler_0x5a355_vertical_index_advances_the_high_nibble() {
    let (mut sim, mut st) = fixture(0x10);
    sim.fff0 = 2;
    position(&mut st.b, 100, 100);
    st.b[25] = 2;
    st.b[24] = 0x14;
    sim.handler_0x5a355(&mut st);
    assert_eq!(st.b[15], 0x20);
    assert_eq!(st.b[24], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a355_real_mode_b_anim67_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 67).unwrap().handler, 0x5A355);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 67;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        100,
        100,
        &layout,
        &tables,
        &MarioContact {
            x: 1000,
            ..Default::default()
        },
        false,
    );
    assert_eq!(sim.slots[1].b[18], 72);
    assert_eq!(sim.slots[1].b[15], 0x10);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a355_turns_reload_both_curve_indices_from_the_rom_helpers() {
    // $63F7/$6405/$6478/$6486 reload Y; $649F/$64B8/$64C6/$64D4 reload X.
    // The $80 bytes at indices 0 and 21 are never movement samples.
    for (state, yi, xi, next_state, next_yi, next_xi, dx, dy) in [
        (0x10, 20, 10, 0x20, 1, 9, 2, 3),
        (0x20, 20, 10, 0x30, 20, 9, 2, 0),
        (0x30, 1, 10, 0x40, 20, 9, 2, -4),
        (0x40, 1, 10, 0x10, 1, 9, 2, -1),
        (0x10, 10, 1, 0x11, 11, 1, 1, 2),
        (0x11, 10, 20, 0x12, 11, 1, -3, 2),
        (0x12, 10, 20, 0x13, 11, 20, 0, 2),
        (0x13, 10, 1, 0x10, 11, 20, 4, 2),
    ] {
        let (mut sim, mut st) = fixture(state);
        sim.fff0 = 0;
        position(&mut st.b, 488, 420);
        st.b[24] = yi;
        st.b[25] = xi;
        let mut expected = st.b;
        expected[..2].copy_from_slice(&488u16.wrapping_add_signed(dx).to_be_bytes());
        expected[3..5].copy_from_slice(&420u16.wrapping_add_signed(dy).to_be_bytes());
        expected[15] = next_state;
        expected[24] = next_yi;
        expected[25] = next_xi;
        sim.handler_0x5a355(&mut st);
        assert_eq!(
            st.b, expected,
            "state {state:02x}, Y index {yi}, X index {xi}"
        );
    }
}

#[test]
fn handler_0x5a355_completes_closed_orbits_in_both_directions() {
    // Each curve's 20 ROM samples sum to 40 px. Four quarters return to
    // the starting position and indices, with the opposite winding for Mario's side.
    for (mario_x, quarters) in [
        (
            300,
            [
                (448, 460, 0x23, 1, 20),
                (488, 500, 0x30, 20, 20),
                (528, 460, 0x41, 20, 1),
                (488, 420, 0x12, 1, 1),
            ],
        ),
        (
            600,
            [
                (528, 460, 0x21, 1, 1),
                (488, 500, 0x32, 20, 1),
                (448, 460, 0x43, 20, 20),
                (488, 420, 0x10, 1, 20),
            ],
        ),
    ] {
        let mut sim = ObjectSim::new(vec![], 488);
        sim.mario_ref_x = mario_x;
        let mut actor = Actor::default();
        actor.position = Position { x: 488, y: 456 };
        let mut st = Staging {
            b: actor.to_raw_slot(),
            ..Default::default()
        };
        sim.handler_0x5a355(&mut st);
        for _ in 0..3 {
            for (x, y, state, yi, xi) in quarters {
                for _ in 0..20 {
                    sim.handler_0x5a355(&mut st);
                }
                let x: u16 = x;
                let y: u16 = y;
                assert_eq!(&st.b[..2], &x.to_be_bytes());
                assert_eq!(&st.b[3..5], &y.to_be_bytes());
                assert_eq!([st.b[15], st.b[24], st.b[25]], [state, yi, xi]);
            }
        }
    }
}

#[test]
fn mario_zone_rotating_spike_stays_active_through_repeated_orbits_as_mario_approaches() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    for (margin, bias) in [((0, 0), 0), ((48, 0), 0), ((80, 72), 36), ((176, 72), 36)] {
        let mut sim = ObjectSim::new(vec![], 400);
        sim.configure(13);
        sim.view_margin = margin;
        sim.view_bias_y = bias;
        // Mario Zone 4's second spike: level14 stream position (488, 456), anim 67.
        let mut actor = Actor::default();
        actor.position = Position { x: 488, y: 456 };
        actor.activity = 2;
        actor.definition = 67;
        actor.sprite = 167;
        sim.slots[1].b = actor.to_raw_slot();
        for frame in 0..640 {
            let camera_x = 400 + (frame / 4).min(88);
            sim.step(
                camera_x,
                448,
                &layout,
                &tables,
                &MarioContact {
                    x: camera_x - 46,
                    y: 468,
                    ..Default::default()
                },
                false,
            );
            let slot = &sim.slots[1];
            assert_eq!(
                slot.b[8], 2,
                "frame {frame}, margin {margin:?}: spike despawned"
            );
            assert_eq!(slot.b[5], 67);
            assert_eq!(slot.b[11], 167);
            assert_eq!(slot.b[18], 72);
            assert!(
                (448..=528).contains(&slot.x()),
                "frame {frame}: X {}",
                slot.x()
            );
            assert!(
                (420..=500).contains(&slot.y()),
                "frame {frame}: Y {}",
                slot.y()
            );
            assert!(!sim.slot_done[1], "frame {frame}: spike hidden");
        }
        assert!(sim.unported.is_empty());
    }
}
