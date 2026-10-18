use super::*;
use crate::actor::{Actor, Position};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn layout(metatile: u8) -> LevelLayout {
    LevelLayout::from_bytes(&[metatile; 0x3000]).unwrap()
}

fn position(slot: &mut [u8; 32], x: u16, y: u16) {
    slot[0..2].copy_from_slice(&x.to_be_bytes());
    slot[3..5].copy_from_slice(&y.to_be_bytes());
}

#[test]
fn handler_0x5a9d3_init_blinks_clears_flip_and_arms_contact77() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 0;
    let mut st = Staging::default();
    st.b[11] = 0x10;
    st.b[13] = 0x20;
    st.b[15] = 0;
    let before = st.b;
    sim.handler_0x5a9d3(&mut st, &layout(0x60));
    let mut expected = before;
    expected[11] = 0x11; // $5B511 toggles sprite bit 0
    expected[13] = 0;
    expected[15] = 1;
    expected[18] = 77;
    assert_eq!(st.b, expected);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a9d3_blinks_every_eighth_frame() {
    for (fff0, sprite) in [(7u8, 0x10u8), (8, 0x11), (9, 0x10), (16, 0x11)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = fff0;
        let mut st = Staging::default();
        st.b[11] = 0x10;
        st.b[15] = 1;
        sim.handler_0x5a9d3(&mut st, &layout(0x60));
        assert_eq!(st.b[11], sprite, "fff0={fff0}");
    }
}

#[test]
fn handler_0x5a9d3_crawl_body_walks_and_turns_on_the_leading_wall() {
    for (state, metatile, want_x, want_state, want_flip) in [
        (1u8, 0x60u8, 0x00FFu16, 1u8, 0x00u8),
        (1, 0x00, 0x00FF, 2, 0x20),
        (2, 0x60, 0x0101, 2, 0x00),
        (2, 0x00, 0x0101, 1, 0x20),
        (3, 0x60, 0x00FF, 3, 0x00),
        (4, 0x60, 0x0101, 4, 0x00),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 2; // even, but not a blink frame
        let mut st = Staging::default();
        position(&mut st.b, 0x0100, 0x0080);
        st.b[13] = 0x20;
        st.b[15] = state;
        sim.handler_0x5a9d3(&mut st, &layout(metatile));
        assert_eq!(
            u16::from_be_bytes([st.b[0], st.b[1]]),
            want_x,
            "state={state}, metatile={metatile:#04x}"
        );
        assert_eq!(st.b[15], want_state);
        assert_eq!(st.b[13], want_flip);
    }
}

#[test]
fn handler_0x5a9d3_real_mode_b_anim82_step() {
    let tables = tables();
    let lay = layout(0x60);
    assert_eq!(tables.profile_for(0x0A, 82).unwrap().handler, 0x5A9D3);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 2;
    let mut actor = Actor::default();
    actor.position = Position { x: 0x0100, y: 0x0080 };
    actor.activity = 2;
    actor.definition = 82;
    actor.state = 0;
    actor.sprite_flip = crate::actor::SpriteFlipFlags::from_bits(0x20);
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(0x0080, 0x0080, &lay, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 77);
    assert_eq!(sim.slots[1].b[15], 1);
    assert_eq!(sim.slots[1].b[13], 0);
    assert!(sim.unported.is_empty());
}
