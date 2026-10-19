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
        x: 0x0120,
        y: 0x0080,
        anim_index: 79,
        mode_flag: 2,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [0; 6],
    }
}

fn staging(state: u8, x: u16, y: u16) -> Staging {
    let mut st = Staging::default();
    st.b[5] = 79;
    st.b[15] = state;
    st.b[0..2].copy_from_slice(&x.to_be_bytes());
    st.b[3..5].copy_from_slice(&y.to_be_bytes());
    st
}

fn pos(st: &Staging) -> (u16, u16) {
    (
        u16::from_be_bytes([st.b[0], st.b[1]]),
        u16::from_be_bytes([st.b[3], st.b[4]]),
    )
}

#[test]
fn handler_0x5a85c_state0_arms_type32_and_waits_for_a_rider() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(0, 0x0100, 0x0080);
    st.b[18] = 0x55;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!((st.b[18], st.b[15]), (32, 0));

    st.b[23] = 8;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!(st.b[15], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a85c_up_leg_steps_object_and_rider_every_other_frame() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario.x = 0x0104;
    sim.mario.y = 0x0084;
    sim.fff0 = 1;
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[23] = 8;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!(pos(&st), (0x0100, 0x0080), "odd frame does nothing");
    assert_eq!((sim.mario_x_adjust, sim.mario_y_adjust), (None, None));

    sim.fff0 = 2;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!(pos(&st), (0x00FF, 0x007F));
    assert_eq!(st.b[25], 1);
    assert_eq!(
        (sim.mario_x_adjust, sim.mario_y_adjust),
        (Some(0x0103), Some(0x0083))
    );
    assert_eq!(sim.scroll_delta_left, 1);
}

#[test]
fn handler_0x5a85c_up_leg_ends_at_48_and_down_leg_at_32() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = staging(1, 0x0100, 0x0080);
    st.b[25] = 47;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!((st.b[25], st.b[15]), (0, 2));

    st.b[25] = 31;
    st.b[23] = 8;
    let before = pos(&st);
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!(pos(&st), (before.0 - 1, before.1 + 1));
    assert_eq!((st.b[25], st.b[15], st.b[28]), (0, 3, 64));

    st.b[25] = 47;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!((st.b[25], st.b[15]), (0, 4), "state 3 climbs again");
}

#[test]
fn handler_0x5a85c_flash_phase_counts_down_then_respawns() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![record()], 0);
    sim.configure(0x0A);
    sim.fff0 = 0;
    let mut st = staging(4, 0x0100, 0x0080);
    st.b[28] = 3;
    st.b[14] = 0;
    st.b[6] = 0xAB;
    st.b[7] = 0x08;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!((st.b[14], st.b[28], st.b[15]), (0x10, 2, 4));

    sim.fff0 = 1;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!(
        (st.b[14], st.b[28]),
        (0x10, 1),
        "odd frame: no blink, still counting"
    );
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!(st.b[15], 0, "reset restores the spawn state");
    assert_eq!(st.b[5], 79);
    assert_eq!(st.b[11], tables.anim_to_sprite_b[79]);
    assert_eq!(u16::from_be_bytes([st.b[0], st.b[1]]), 0x0120);
    assert_eq!(sim.done, 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a85c_unknown_state_reports_without_writes() {
    let tables = tables();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staging(5, 0x0100, 0x0080);
    let before = st.b;
    sim.handler_0x5a85c(&mut st, &tables);
    assert_eq!(st.b, before);
    assert_eq!(sim.unported, vec![0x5A85C]);
}
