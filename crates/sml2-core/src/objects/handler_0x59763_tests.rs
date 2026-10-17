use super::*;

#[test]
fn handler_0x59763_init_arms_contact_and_faces_mario() {
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 50;
    let mut st = Staging::default();
    st.b[15] = 0;
    st.b[0..2].copy_from_slice(&100u16.to_be_bytes());
    sim.handler_0x59763(&mut st, &layout);
    assert_eq!(st.b[18], handler_0x59763::CONTACT);
    assert_eq!(st.b[15], 1);
    assert_eq!(sim.facing, 255);
}
