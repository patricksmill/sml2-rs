use super::*;
use crate::actor::{Actor, InteractionFlags, Position};

fn fixture(state: u8) -> (ObjectSim, Staging) {
    let sim = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    st.b[15] = state;
    st.b[23] &= 0x0F;
    (sim, st)
}

#[test]
fn handler_0x5a236_init_arms_contact_32() {
    let (mut sim, mut st) = fixture(0);
    let mut expected = st.b;
    expected[15] = 1;
    expected[18] = 32;
    sim.handler_0x5a236(&mut st);
    assert_eq!(st.b, expected);
    assert_eq!(sim.mario_x_adjust, None);
}

#[test]
fn handler_0x5a236_sprite_cycle_and_even_frame_carry() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 0;
    sim.mario.x = 50;
    st.b[11] = 10;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    st.b[26] = 0;
    sim.handler_0x5a236(&mut st);
    assert_eq!(st.b[26], 1);
    assert_eq!(st.b[11], 11);
    assert_eq!(sim.mario_x_adjust, Some(51));

    sim.fff0 = 4;
    sim.handler_0x5a236(&mut st);
    assert_eq!(st.b[26], 2);
    assert_eq!(st.b[11], 12);

    sim.fff0 = 8;
    sim.handler_0x5a236(&mut st);
    assert_eq!(st.b[26], 0);
    assert_eq!(st.b[11], 10);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a236_odd_frame_does_not_carry() {
    let (mut sim, mut st) = fixture(1);
    sim.fff0 = 1;
    sim.mario.x = 50;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    sim.handler_0x5a236(&mut st);
    assert_eq!(sim.mario_x_adjust, None);
}

#[test]
fn handler_0x5a236_real_mode_b_anim62_step() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.profile_for(0x0A, 62).unwrap().handler, 0x5A236);
    let mut sim = ObjectSim::new(vec![], 0);
    sim.configure(0x0A);
    sim.fff0 = 1;
    let mut actor = Actor::default();
    actor.position = Position { x: 100, y: 100 };
    actor.activity = 2;
    actor.definition = 62;
    sim.slots[1].b = actor.to_raw_slot();
    sim.step(100, 100, &layout, &tables, &MarioContact::default(), false);
    assert_eq!(sim.slots[1].b[18], 32);
    assert_eq!(sim.slots[1].b[15], 1);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x5a236_carries_mario_and_drives_camera_follow() {
    use crate::camera::CameraSim;
    use crate::player_sim::PlayerSim;

    let mut obj = ObjectSim::new(vec![], 0);
    let mut st = Staging::default();
    st.b[15] = 1;
    st.b[23] = InteractionFlags::from_bits(0x08).bits();
    obj.fff0 = 0;
    obj.mario.x = 80;
    obj.mario.hvel = 0x80;

    obj.handler_0x5a236(&mut st);
    assert_eq!(obj.mario_x_adjust, Some(81));
    assert_eq!(obj.scroll_delta_right, 1);

    let mut cam = CameraSim::new(80, 72);
    cam.right_enabled = true;
    cam.apply_object_result(&mut obj);
    assert_eq!(cam.scroll_delta_right, 1);
    assert_eq!(obj.scroll_delta_right, 0);

    let mut player = PlayerSim::new();
    player.x = 81;
    player.y = 72;
    player.x_sub = 0;
    cam.step(&mut player);

    assert_eq!(cam.x, 82);
    assert_eq!(cam.scroll_delta_right, 0);
}

#[test]
fn mario_can_walk_freely_after_stepping_off_platform() {
    use crate::player_sim::{PlayerKeys, PlayerSim};

    let mut player = PlayerSim::new();
    player.x = 80;
    player.y = 72;
    player.camx = 80;
    player.on_ground = true;
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();

    let mut obj = ObjectSim::new(vec![], 0);
    // Platform carried Mario in frame 1
    obj.mario_x_adjust = Some(81);
    obj.on_object = true;
    player.apply_object_result(&mut obj, false);
    assert_eq!(player.x, 81);
    assert_eq!(obj.mario_x_adjust, None);

    // Frame 2: Mario steps off platform and walks right
    let keys = PlayerKeys {
        right: true,
        ..Default::default()
    };
    player.step(&keys, false, false, &layout);
    player.step(&keys, false, false, &layout);
    assert!(player.x > 81, "Mario moved right in player.step");
    let moved_x = player.x;

    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let contact = MarioContact {
        x: player.x,
        y: player.y,
        on_ground: player.on_ground,
        on_object: player.on_ground_obj,
        hvel: player.hvel,
        ..Default::default()
    };
    obj.step(80, 72, &layout, &tables, &contact, false);
    assert_eq!(obj.mario_x_adjust, None);

    player.apply_object_result(&mut obj, false);
    assert_eq!(
        player.x, moved_x,
        "Mario's position was preserved after getting off platform"
    );
}
