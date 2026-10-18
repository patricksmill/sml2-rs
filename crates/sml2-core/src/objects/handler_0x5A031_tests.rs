use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn layout(metatile: u8) -> LevelLayout {
    LevelLayout::from_bytes(&[metatile; 0x3000]).unwrap()
}

fn layout_at(cells: &[(usize, usize, u8)]) -> LevelLayout {
    let mut bytes = [0x60u8; 0x3000];
    for &(x, y, value) in cells {
        bytes[y * 256 + x] = value;
    }
    LevelLayout::from_bytes(&bytes).unwrap()
}

fn staging(state: u8, x: u16, y: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 36;
    st.b[15] = state;
    st.b[0..2].copy_from_slice(&x.to_be_bytes());
    st.b[3..5].copy_from_slice(&y.to_be_bytes());
    st
}

fn x_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[0], slot[1]])
}

fn y_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[3], slot[4]])
}

#[test]
fn handler_0x5a031_init_arms_contact_and_snaps_16_px() {
    for (flip, state, x) in [
        (0x00u8, 1u8, 0x00F0u16),
        (0x20, 2, 0x0110),
        (0x40, 1, 0x0110), // bit 5 picks the state; any flip bit snaps right
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1; // odd: no sprite cycle
        let mut st = staging(0, 0x0100, 0x0080);
        st.b[13] = flip;
        sim.handler_0x5a031(&mut st, &layout(0x60), &tables());
        assert_eq!(st.b[18], 72, "flip={flip:#04x}");
        assert_eq!(st.b[15], state);
        assert_eq!(x_of(&st.b), x);
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x5a031_sprite_cycle_runs_on_even_frames() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[11] = 10;
    st.b[26] = 0;
    for (frame, sprite, phase) in [(1u8, 11u8, 1u8), (2, 12, 2), (0, 10, 0)] {
        sim.handler_0x5a031(&mut st, &layout(0x60), &tables());
        assert_eq!((st.b[11], st.b[26]), (sprite, phase), "even frame {frame}");
    }
    sim.fff0 = 1;
    let before = st.b;
    sim.handler_0x5a031(&mut st, &layout(0x60), &tables());
    assert_eq!((st.b[11], st.b[26]), (before[11], before[26]));
}

#[test]
fn handler_0x5a031_walks_and_removes_on_the_leading_wall() {
    let tables = tables();
    for (state, metatile, want_x) in [(1u8, 0x60u8, 0x00FFu16), (2, 0x60, 0x0101)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1;
        let mut st = staging(state, 0x0100, 0x0080);
        sim.handler_0x5a031(&mut st, &layout(metatile), &tables);
        assert_eq!(x_of(&st.b), want_x, "state={state}");

        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1;
        sim.active = 1;
        let mut st = staging(state, 0x0100, 0x0080);
        sim.handler_0x5a031(&mut st, &layout(0x00), &tables);
        let hb = tables.hitbox_for(0, 0).unwrap();
        assert_eq!(&st.b[..19], &[0; 19], "wall removes state {state}");
        assert_eq!(&st.b[19..23], &hb.map(|b| b as u8));
        assert_eq!(&st.b[23..], &[0; 9]);
        assert_eq!(sim.done, 2);
        assert_eq!((sim.active, sim.standby), (0, 0));
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x5a031_rise_leg_uses_the_eight_step_curve() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[16] = 0; // phase 0 selects 2
    st.b[24] = 0; // rise leg
    sim.handler_0x5a031(&mut st, &layout(0x60), &tables());
    assert_eq!(y_of(&st.b), 0x0080 - 2);
    assert_eq!((st.b[16], st.b[24]), (1, 0));

    st.b[16] = 7; // phase 7 selects 0 and postincrements to the cap
    sim.handler_0x5a031(&mut st, &layout(0x60), &tables());
    assert_eq!(y_of(&st.b), 0x0080 - 2);
    assert_eq!((st.b[16], st.b[24]), (0, 1), "rise ends into the fall leg");
}

#[test]
fn handler_0x5a031_fall_leg_moves_on_the_7488_curve_and_lands() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[16] = 5; // FALL_SPEED[5] = 2
    st.b[24] = 1;
    sim.handler_0x5a031(&mut st, &layout(0x60), &tables());
    assert_eq!(y_of(&st.b), 0x0080 + 2);
    assert_eq!((st.b[16], st.b[24]), (6, 1));

    st.b[16] = 5;
    sim.handler_0x5a031(&mut st, &layout(0x40), &tables());
    assert_eq!(y_of(&st.b), 0x0080 + 2, "grounded: no fall step");
    assert_eq!((st.b[16], st.b[24]), (0, 0), "landing toggles the leg");

    st.b[24] = 1;
    st.b[16] = 8; // $5B441's cap is 8, unlike the 21-cap fall consumers
    sim.handler_0x5a031(&mut st, &layout(0x60), &tables());
    assert_eq!((st.b[16], y_of(&st.b)), (8, 0x0080 + 2 + 2));
}

#[test]
fn handler_0x5a031_only_the_leading_side_wall_removes() {
    let tables = tables();
    let left = layout_at(&[(0x0F, 7, 0x00)]);
    let right = layout_at(&[(0x10, 7, 0x00)]);
    for (state, lay, removed) in [(1u8, &left, true), (1, &right, false), (2, &right, true), (2, &left, false)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1;
        sim.active = 1;
        let mut st = staging(state, 0x0100, 0x0080);
        sim.handler_0x5a031(&mut st, lay, &tables);
        assert_eq!(sim.done == 2, removed, "state={state}");
        if !removed {
            assert_eq!(st.b[15], state);
            assert_eq!(x_of(&st.b), if state == 1 { 0x00FF } else { 0x0101 });
        }
    }
}

#[test]
fn handler_0x5a031_unknown_state_reports_without_writes() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    let mut st = staging(3, 0x0100, 0x0080);
    let before = st.b;
    sim.handler_0x5a031(&mut st, &layout(0x60), &tables());
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A031]);
}

#[test]
fn handler_0x5a031_real_mode_b_anim36_step() {
    let tables = tables();
    let lay = layout(0x60);
    assert_eq!(tables.profile_for(0x0A, 36).unwrap().handler, 0x5A031);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 0x0100, y: 0x0080 };
    actor.activity = 2;
    actor.definition = 36;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(0x0080, 0x0080, &lay, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 72);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(x_of(&sim.slots[1].b), 0x00F0);
    assert!(sim.unported.is_empty());
}
