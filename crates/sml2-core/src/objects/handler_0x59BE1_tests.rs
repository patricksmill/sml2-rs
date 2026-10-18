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
    st.b[5] = 89;
    st.b[15] = state;
    st
}

#[test]
fn handler_0x59be1_init_arms_the_flash_and_timer() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0);
    sim.handler_0x59be1(&mut st, &tables);
    assert_eq!((st.b[14], st.b[15], st.b[28]), (16, 1, 32));
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59be1_counts_down_then_removes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.active = 1;
    let mut st = staging(1);
    st.b[28] = 2;
    sim.handler_0x59be1(&mut st, &tables);
    assert_eq!(st.b[28], 1);
    assert_eq!(sim.done, 0);
    sim.handler_0x59be1(&mut st, &tables);
    let hb = tables.hitbox_for(0, 0).unwrap();
    assert_eq!(&st.b[..19], &[0; 19]);
    assert_eq!(&st.b[19..23], &hb.map(|b| b as u8));
    assert_eq!(&st.b[23..], &[0; 9]);
    assert_eq!(sim.done, 2);
    assert_eq!((sim.active, sim.standby), (0, 0));
}

#[test]
fn handler_0x59be1_unknown_state_reports_without_writes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(2);
    let before = st.b;
    sim.handler_0x59be1(&mut st, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x59BE1]);
}
