use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn staging(state: u8, x: u16, y: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 89;
    st.b[15] = state;
    st.b[0..2].copy_from_slice(&x.to_be_bytes());
    st.b[3..5].copy_from_slice(&y.to_be_bytes());
    st
}

const AIR: [u8; 0x3000] = [0x60; 0x3000];
const GROUND: [u8; 0x3000] = [0x00; 0x3000];

fn y_of(slot: &[u8; 32]) -> u16 {
    u16::from_be_bytes([slot[3], slot[4]])
}

#[test]
fn handler_0x5b06d_init_lifts_16_and_faces_mario() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&AIR).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 0x0200;
    let mut st = staging(0, 0x0100, 0x0080);
    sim.handler_0x5b06d(&mut st, &lay, &tables);
    assert_eq!(y_of(&st.b), 0x0080 - 16);
    assert_eq!((st.b[18], st.b[15], st.b[13]), (72, 2, 0x20));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5b06d_rises_on_the_74a7_curve_and_flips_to_falling() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&AIR).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[24] = 0;
    st.b[16] = 0;
    sim.handler_0x5b06d(&mut st, &lay, &tables);
    assert_eq!(
        (u16::from_be_bytes([st.b[0], st.b[1]]), y_of(&st.b)),
        (0x00FF, 0x0080 - 3)
    );
    assert_eq!((st.b[16], st.b[24]), (1, 0));

    let mut st = staging(2, 0x0100, 0x0080);
    st.b[24] = 0;
    st.b[16] = 12;
    sim.handler_0x5b06d(&mut st, &lay, &tables);
    assert_eq!((st.b[16], st.b[24]), (0, 1), "phase 12 ends the rise");
    assert_eq!(y_of(&st.b), 0x0080, "the last curve entry is zero");

    st.b[24] = 1;
    sim.handler_0x5b06d(&mut st, &lay, &tables);
    assert_eq!(
        (st.b[16], y_of(&st.b)),
        (1, 0x0080),
        "fall starts at speed 0"
    );

    st.b[16] = 12;
    sim.handler_0x5b06d(&mut st, &lay, &tables);
    assert_eq!(
        (st.b[16], y_of(&st.b)),
        (12, 0x0080 + 2),
        "fall is capped at phase 12"
    );
}

#[test]
fn handler_0x5b06d_ground_probe_hardens_and_removes() {
    let tables = tables();
    let ground = LevelLayout::from_bytes(&GROUND).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.active = 1;
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[24] = 1;
    st.b[11] = 0x10;
    sim.handler_0x5b06d(&mut st, &ground, &tables);
    assert_eq!((st.b[11], st.b[15], st.b[28]), (113, 3, 16));

    st.b[28] = 1;
    sim.handler_0x5b06d(&mut st, &ground, &tables);
    let hb = tables.hitbox_for(0, 0).unwrap();
    assert_eq!(&st.b[..19], &[0; 19]);
    assert_eq!(&st.b[19..23], &hb.map(|b| b as u8));
    assert_eq!(&st.b[23..], &[0; 9]);
    assert_eq!(sim.done, 2);
    assert_eq!(sim.active, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5b06d_unknown_state_reports_without_writes() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&AIR).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(4, 0x0100, 0x0080);
    let before = st.b;
    sim.handler_0x5b06d(&mut st, &lay, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5B06D]);
}
