use super::*;

#[test]
fn handler_0x59b0e_init_faces_and_arms_contact() {
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 50;
    let mut st = Staging::default();
    st.b[15] = 0;
    sim.handler_0x59b0e(&mut st, &layout);
    assert_eq!(st.b[18], handler_0x59b0e::CONTACT);
    assert_eq!(st.b[15], 2);
}
