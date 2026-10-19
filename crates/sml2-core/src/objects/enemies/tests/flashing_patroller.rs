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
    st.b[5] = 80;
    st.b[15] = state;
    st.b[0..2].copy_from_slice(&x.to_be_bytes());
    st.b[3..5].copy_from_slice(&y.to_be_bytes());
    st
}

#[test]
fn handler_0x5966c_init_faces_and_arms_the_leg() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 0x0200;
    let mut st = staging(0, 0x0100, 0x0080);
    sim.handler_0x5966c(&mut st);
    assert_eq!((st.b[18], st.b[15], st.b[14]), (72, 2, 16));
    assert_eq!((st.b[25], st.b[13]), (48, 0x20));
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x0080 - 4);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5966c_legs_walk_and_swap_at_the_leg_end() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1;
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[25] = 2;
    sim.handler_0x5966c(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x00FF);
    assert_eq!((st.b[25], st.b[15]), (1, 1));

    st.b[25] = 1;
    sim.handler_0x5966c(&mut st);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x00FE);
    assert_eq!((st.b[25], st.b[28], st.b[15]), (96, 16, 2));

    sim.handler_0x5966c(&mut st);
    assert_eq!(
        u16::from_be_bytes([st.b[0], st.b[1]]),
        0x00FF,
        "state 2 walks right"
    );

    st.b[25] = 1;
    sim.handler_0x5966c(&mut st);
    assert_eq!(
        (st.b[25], st.b[28], st.b[15]),
        (96, 16, 1),
        "state 2 leg end flips back"
    );
}

#[test]
fn handler_0x5966c_pause_blinks_then_restores_flash() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[28] = 1;
    st.b[14] = 0;
    sim.handler_0x5966c(&mut st);
    assert_eq!((st.b[14], st.b[28]), (16, 0));
}

#[test]
fn handler_0x5966c_unknown_state_reports_without_writes() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(3, 0x0100, 0x0080);
    let before = st.b;
    sim.handler_0x5966c(&mut st);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5966C]);
}

#[test]
fn handler_0x5966c_real_mode_a_anim80_step() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x00, 80).unwrap().handler, 0x5966C);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x00);
    sim.mario_ref_x = 0x0050;
    let mut actor = crate::actor::Actor::default();
    actor.position = crate::actor::Position {
        x: 0x0100,
        y: 0x0080,
    };
    actor.activity = 2;
    actor.definition = 80;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(
        0x0080,
        0x0080,
        &lay,
        &tables,
        &MarioContact::default(),
        false,
    );
    assert_eq!((sim.slots[1].b[18], sim.slots[1].b[15]), (72, 1));
    assert!(sim.unported.is_empty());
}
