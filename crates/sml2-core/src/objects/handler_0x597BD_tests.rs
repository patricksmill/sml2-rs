use super::*;

#[test]
fn handler_0x597bd_init_sets_contact_and_leg_counters() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 200;
    let mut st = Staging::default();
    st.b[15] = 0;
    sim.handler_0x597bd(&mut st);
    assert_eq!(st.b[18], handler_0x597bd::CONTACT);
    assert_eq!((st.b[27], st.b[24]), (1, 1));
}
