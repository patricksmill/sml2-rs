use super::*;

#[test]
fn handler_0x599a6_zero_state_arms_contact_and_state_five() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[15] = 0;
    sim.handler_0x599a6(&mut st);
    assert_eq!((st.b[18], st.b[15]), (handler_0x599a6::CONTACT, 5));
}
