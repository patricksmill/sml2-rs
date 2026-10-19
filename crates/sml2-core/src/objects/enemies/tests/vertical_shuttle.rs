use super::*;

#[test]
fn handler_0x59774_zero_state_arms_contact_and_state_three() {
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[15] = 0;
    sim.handler_0x59774(&mut st, &layout);
    assert_eq!((st.b[18], st.b[15]), (handler_0x59774::CONTACT, 3));
}
