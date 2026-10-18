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

#[test]
fn handler_0x59b0e_toggles_flip_every_eighth_frame_without_a_sprite_blink() {
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    for (fff0, flip) in [(2u8, 0x00u8), (4, 0x00), (8, 0x20), (10, 0x00), (16, 0x20)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = fff0;
        sim.mario_ref_x = 50;
        let mut st = Staging::default();
        st.b[0] = 0x01;
        st.b[1] = 0x00;
        st.b[3] = 0x00;
        st.b[4] = 0x80;
        st.b[11] = 0x10;
        st.b[15] = 1;
        sim.handler_0x59b0e(&mut st, &layout);
        assert_eq!(st.b[13], flip, "fff0={fff0}");
        assert_eq!(st.b[11], 0x10, "$5B678 does not touch the sprite");
        assert_eq!(st.b[15], 1);
    }
}
