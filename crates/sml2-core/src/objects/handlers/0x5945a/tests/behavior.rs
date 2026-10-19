use super::*;
use crate::actor::{Actor, Position};

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5945a_anim66_landing_and_patrol_stay_distinct_from_anim65() {
    let layout = LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.mario_ref_x = 200;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[13] = 0;
    st.b[14] = 0;
    st.b[15] = 0;
    position(&mut st.b, 100, 96);
    sim.handler_0x5945a(&mut st, &layout);
    assert_eq!(st.b[15], 2);
    assert_eq!(st.b[18], 72);
    assert_eq!(st.b[14], 16);
    assert_eq!(sim.facing, 1);
    assert!(sim.unported.is_empty());

    // State 1 is the shared $582FB patrol: even frame, no wall -> X-1.
    sim.fff0 = 0;
    st.b[15] = 1;
    st.b[13] = 0;
    st.b[14] = 0;
    position(&mut st.b, 100, 100);
    let mut expected = st.b;
    position(&mut expected, 99, 100);
    expected[11] ^= 1; // $5B511
    let layout = LevelLayout::from_bytes(&[0x40; 0x3000]).unwrap();
    sim.handler_0x5945a(&mut st, &layout);
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5945a_real_anim66_step_dispatches_to_its_entry() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x00; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0, 66).unwrap().handler, 0x5945A);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 3;
    sim.phase = false;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 96 };
    actor.activity = 2;
    actor.definition = 66;
    actor.state = 0;
    sim.slots[1].b = actor.to_raw_slot();
    let mario = MarioContact {
        x: 1000,
        ..Default::default()
    };
    sim.step(100, 100, &layout, &tables, &mario, false);
    assert_eq!(sim.slots[1].b[15], 2);
    assert_eq!(sim.slots[1].b[18], 72);
    assert_eq!(sim.slots[1].b[14], 16);
    assert!(sim.unported.is_empty());
}
