use super::*;

#[test]
fn handler_0x596d9_init_faces_mario_and_arms_contact() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 200;
    let mut st = Staging::default();
    st.b[15] = 0;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    sim.handler_0x596d9(&mut st);
    assert_eq!(st.b[18], handler_0x596d9::CONTACT);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[13], 0x20);
    assert_eq!(sim.facing, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x596d9_spawns_child_when_timer_zero() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[15] = 1;
    st.b[28] = 0;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    st.b[3..5].copy_from_slice(&80u16.to_be_bytes());
    sim.slot_ptr_hi = 0xAD;
    sim.standby = 0;
    sim.handler_0x596d9(&mut st);
    assert_eq!(st.b[15], 3);
    assert_eq!(st.b[28], handler_0x596d9::POST_SPAWN_TIMER);
    assert_eq!(st.b[11], handler_0x596d9::SPAWN_SPRITE);
    assert_eq!(sim.standby, 1);
    assert!(sim.unported.is_empty());
}
