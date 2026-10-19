use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn staging(state: u8) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 69;
    st.b[15] = state;
    st
}

#[test]
fn handler_0x594fb_init_arms_the_fuse() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0);
    st.b[18] = 0x55;
    sim.handler_0x594fb(&mut st, &tables);
    assert_eq!((st.b[15], st.b[28], st.b[18]), (1, 48, 0));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x594fb_fuse_blinks_then_arms_the_blast() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = staging(1);
    st.b[28] = 3;
    st.b[11] = 0x10;
    st.b[14] = 0;
    sim.handler_0x594fb(&mut st, &tables);
    assert_eq!((st.b[11], st.b[14], st.b[28]), (0x11, 0x10, 2));

    sim.fff0 = 1;
    sim.handler_0x594fb(&mut st, &tables);
    assert_eq!(
        (st.b[11], st.b[14], st.b[28]),
        (0x11, 0x10, 1),
        "odd frame does not blink"
    );

    sim.fff0 = 2;
    sim.handler_0x594fb(&mut st, &tables);
    assert_eq!((st.b[15], st.b[28]), (2, 0));
    assert_eq!((st.b[18], st.b[11], st.b[14]), (72, 130, 16));
    assert_eq!(sim.sfx1, 13);
}

#[test]
fn handler_0x594fb_blast_cycles_to_84_and_removes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 8;
    sim.active = 1;
    let mut st = staging(2);
    st.b[11] = 0x82;
    st.b[26] = 0;
    sim.handler_0x594fb(&mut st, &tables);
    assert_eq!(st.b[11], 0x83);
    sim.fff0 = 16;
    sim.handler_0x594fb(&mut st, &tables);
    let hb = tables.hitbox_for(0, 0).unwrap();
    assert_eq!(&st.b[..19], &[0; 19]);
    assert_eq!(&st.b[19..23], &hb.map(|b| b as u8));
    assert_eq!(&st.b[23..], &[0; 9]);
    assert_eq!(sim.done, 2);
    assert_eq!((sim.active, sim.standby), (0, 0));
}

#[test]
fn handler_0x594fb_unknown_state_reports_without_writes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(3);
    let before = st.b;
    sim.handler_0x594fb(&mut st, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x594FB]);
}
