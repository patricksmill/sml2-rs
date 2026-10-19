use super::*;

#[test]
fn handler_0x59aad_init_sets_contact_and_facing_state() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 300;
    let mut st = Staging::default();
    st.b[15] = 0;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    sim.handler_0x59aad(&mut st);
    assert_eq!(st.b[18], handler_0x59aad::CONTACT);
    assert_eq!(st.b[15], 2);
}
