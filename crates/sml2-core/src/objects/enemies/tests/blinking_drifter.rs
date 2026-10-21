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

/// `$59AD6`: leg 0 rises on `$74A7` (3, 2, 1, ...), then leg 1 falls on `$7488`.
#[test]
fn handler_0x59aad_bobs_on_the_eased_curves() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[15] = 2;
    st.b[3..5].copy_from_slice(&200u16.to_be_bytes());
    let mut ys = Vec::new();
    for _ in 0..13 {
        sim.handler_0x59aad(&mut st);
        ys.push(u16::from_be_bytes([st.b[3], st.b[4]]));
        sim.fff0 = sim.fff0.wrapping_add(2);
    }
    assert_eq!(&ys[..4], &[197, 195, 194, 192], "up 3, 2, 1, 2");
    assert_eq!(ys[11], 184, "leg 0 rises 16");
    assert_eq!((st.b[16], st.b[24]), (1, 1), "leg 1 has taken its first step");
    assert_eq!(ys[12], 184, "and that step is $7488[0] = 0");
}
