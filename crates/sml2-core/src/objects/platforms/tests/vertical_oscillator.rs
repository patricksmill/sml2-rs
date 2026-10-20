use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

fn y_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[3], slot[4]])
}

#[test]
fn handler_0x595fe_init_arms_contact_and_leg_counters() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(11));
    st.b[15] = 0;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    st.b[3..5].copy_from_slice(&200u16.to_be_bytes());
    let mut expected = st.b;
    expected[0..2].copy_from_slice(&96u16.to_be_bytes());
    expected[15] = 1;
    expected[18] = handler_0x595fe::CONTACT;
    expected[24] = handler_0x595fe::LEG;
    expected[28] = handler_0x595fe::SPRITE_TIMER;
    sim.handler_0x595fe(&mut st);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x595fe_alternates_vertical_legs_and_toggles_state() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = Staging::default();
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.state = 1;
    actor.motion_leg = 2;
    actor.timer = 0;
    actor.contact_kind = crate::actor::ContactKind::from_raw(handler_0x595fe::CONTACT);
    st.b = actor.to_raw_slot();
    sim.handler_0x595fe(&mut st);
    assert_eq!(
        (y_of(&st.b), st.b[24], st.b[15]),
        (102, handler_0x595fe::LEG, 2)
    );
    assert_eq!(st.b[28], handler_0x595fe::SPRITE_TIMER);
    st.b[28] = 0;
    st.b[24] = 2;
    sim.handler_0x595fe(&mut st);
    assert_eq!((y_of(&st.b), st.b[15]), (100, 1));
}

#[test]
fn handler_0x595fe_carries_stood_on_rider_on_y() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario.y = 300;
    let mut st = Staging::default();
    st.b[15] = 1;
    st.b[24] = 2;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[3..5].copy_from_slice(&100u16.to_be_bytes());
    sim.handler_0x595fe(&mut st);
    assert_eq!(sim.mario_y_adjust, Some(302));
}
