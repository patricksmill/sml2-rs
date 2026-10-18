use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn staging(state: u8, pointer: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 55;
    st.b[15] = state;
    st.b[8] = 2;
    st.b[6..8].copy_from_slice(&pointer.to_be_bytes());
    st
}

fn parent(sim: &mut ObjectSim, x: u16, y: u16, marker: u8, activity: u8) {
    let mut parent = Actor::default();
    parent.position = Position { x, y };
    parent.marker = marker;
    parent.activity = activity;
    sim.slots[0].b = parent.to_raw_slot();
}

#[test]
fn handler_0x5a969_init_promotes_to_state_one_only() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0, 0xAD00);
    st.b[11] = 10;
    let before = st.b;
    sim.handler_0x5a969(&mut st, &tables);
    let mut expected = before;
    expected[15] = 1;
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a969_wobbles_the_sprite_on_even_frames() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    parent(&mut sim, 0x0100, 0x0080, 0, 2);
    sim.fff0 = 0;
    let mut st = staging(1, 0xAD00);
    st.b[11] = 10;
    st.b[26] = 0;
    for (frame, sprite) in [11u8, 10, 12, 10].into_iter().enumerate() {
        sim.handler_0x5a969(&mut st, &tables);
        assert_eq!(st.b[11], sprite, "even frame {frame}");
    }
    assert_eq!(st.b[26], 0, "four-step cycle wraps to zero");

    sim.fff0 = 1;
    let before = st.b;
    sim.handler_0x5a969(&mut st, &tables);
    assert_eq!((st.b[11], st.b[26]), (before[11], before[26]));
}

#[test]
fn handler_0x5a969_follows_the_parent_and_drops_twenty() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    parent(&mut sim, 0x1234, 0x5678, 2, 2);
    sim.fff0 = 1;
    let mut st = staging(1, 0xAD00);
    sim.handler_0x5a969(&mut st, &tables);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x1234);
    assert_eq!(u16::from_be_bytes([st.b[3], st.b[4]]), 0x5678 + 20);
    assert_eq!(st.b[2], 2);
    assert_eq!(st.b[15], 1);
    assert_eq!(sim.done, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a969_removes_itself_when_the_parent_is_inactive() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    parent(&mut sim, 0x0100, 0x0080, 0, 1);
    sim.active = 1;
    sim.standby = 0;
    sim.fff0 = 1;
    let mut st = staging(1, 0xAD00);
    sim.handler_0x5a969(&mut st, &tables);
    let hb = tables.hitbox_for(0, 0).unwrap();
    assert_eq!(&st.b[..19], &[0; 19]);
    assert_eq!(&st.b[19..23], &hb.map(|b| b as u8));
    assert_eq!(&st.b[23..], &[0; 9]);
    assert_eq!(sim.done, 2);
    assert_eq!((sim.active, sim.standby), (0, 0));
    assert_eq!(sim.remove_mode, 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a969_unresolved_source_reports_without_writes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(1, 0xAB08);
    st.b[11] = 10;
    let before = st.b;
    sim.handler_0x5a969(&mut st, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A969]);
}

#[test]
fn handler_0x5a969_unknown_state_reports_without_writes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    parent(&mut sim, 0x0100, 0x0080, 0, 2);
    let mut st = staging(2, 0xAD00);
    let before = st.b;
    sim.handler_0x5a969(&mut st, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A969]);
}

#[test]
fn handler_0x5a969_real_mode_b_anim55_step() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 55).unwrap().handler, 0x5A969);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    parent(&mut sim, 0x0100, 0x0080, 0, 1);
    let mut actor = Actor::default();
    actor.spawn_record = 0xAD00;
    actor.activity = 2;
    actor.definition = 55;
    actor.state = 0;
    actor.sprite = 10;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(0x0080, 0x0080, &lay, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[15], 1);
    sim.step(0x0080, 0x0080, &lay, &tables, &MarioContact::default(), false);
    assert_eq!(&sim.slots[1].b[..19], &[0; 19], "parent standby removes the child");
    assert_eq!(&sim.slots[1].b[23..], &[0; 9]);
    assert!(sim.unported.is_empty());
}
