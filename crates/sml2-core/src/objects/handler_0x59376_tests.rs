use super::*;
use crate::level::SpawnRecord;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn record() -> SpawnRecord {
    SpawnRecord {
        x: 0x0100,
        y: 0x0080,
        anim_index: 63,
        mode_flag: 2,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [0; 6],
    }
}

fn staging(state: u8, x: u16, y: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 63;
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
fn handler_0x59376_init_arms_zero_contact_and_flash() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&AIR).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0, 0x0100, 0x0080);
    st.b[18] = 0x55;
    sim.handler_0x59376(&mut st, &lay, &tables);
    assert_eq!((st.b[18], st.b[14], st.b[15]), (0, 16, 1));
    assert_eq!(sim.sfx2, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59376_falls_on_the_fall_curve_until_grounded() {
    let tables = tables();
    let air = LevelLayout::from_bytes(&AIR).unwrap();
    let ground = LevelLayout::from_bytes(&GROUND).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[16] = 5;
    sim.handler_0x59376(&mut st, &air, &tables);
    assert_eq!((y_of(&st.b), st.b[16], st.b[15]), (0x0080 + 2, 6, 1));

    let mut st = staging(1, 0x0100, 0x0080);
    sim.handler_0x59376(&mut st, &ground, &tables);
    assert_eq!((st.b[15], st.b[28], y_of(&st.b)), (2, 128, 0x0080));
}

#[test]
fn handler_0x59376_flash_phase_counts_and_mutates_the_sprite() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&AIR).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = staging(2, 0x0100, 0x0080);
    st.b[28] = 1;
    st.b[11] = 0x10;
    sim.handler_0x59376(&mut st, &lay, &tables);
    assert_eq!((st.b[15], st.b[28], st.b[11]), (3, 63, 0x0B));

    sim.fff0 = 4;
    let mut st = staging(3, 0x0100, 0x0080);
    st.b[28] = 62;
    st.b[11] = 0x10;
    sim.handler_0x59376(&mut st, &lay, &tables);
    assert_eq!((st.b[15], st.b[28], st.b[11]), (3, 61, 0x10 ^ 0x1B));

    sim.fff0 = 2;
    let mut st = staging(3, 0x0100, 0x0080);
    st.b[28] = 62;
    st.b[11] = 0x10;
    sim.handler_0x59376(&mut st, &lay, &tables);
    assert_eq!(
        (st.b[28], st.b[11]),
        (61, 0x10),
        "only every fourth frame blinks"
    );
}

#[test]
fn handler_0x59376_flash_end_rebirths_as_anim62_with_the_tail() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&AIR).unwrap();
    let mut sim = ObjectSim::new(vec![record()], 0);
    sim.configure(0x00);
    sim.mario_ref_x = 0x0200;
    sim.fff0 = 1; // odd: no sprite blink on the expiry frame
    let mut st = staging(3, 0x0100, 0x0080);
    st.b[28] = 1;
    st.b[6] = 0xAB;
    st.b[7] = 0x08;
    st.b[11] = 0x10;
    sim.handler_0x59376(&mut st, &lay, &tables);
    assert_eq!(st.b[5], 62, "definition is rewritten before the reset");
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0100);
    assert_eq!((st.b[15], st.b[13]), (2, 0x20), "faces Mario");
    assert_eq!((st.b[18], st.b[11], st.b[28]), (82, 112, 48));
    assert_eq!((st.b[14], st.b[24]), (16, 8));
    assert_eq!(sim.done, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x59376_unknown_state_reports_without_writes() {
    let tables = tables();
    let lay = LevelLayout::from_bytes(&AIR).unwrap();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(4, 0x0100, 0x0080);
    let before = st.b;
    sim.handler_0x59376(&mut st, &lay, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x59376]);
}
