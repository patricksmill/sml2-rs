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

/// `$2D7D` produces contact before `$8121` toggles the slot phase. An
/// initialized odd slot can carry only on phase 1 (`$5800F`), even when FFF0
/// is even on both adjacent engine passes. State 0 bypasses that slot gate.
#[test]
fn conveyor_contact_initialization_and_alternating_dispatch_consume_player_bytes() {
    use crate::actor::{Hitbox, ScreenPosition};
    use crate::player_sim::PlayerSim;

    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    assert_eq!(tables.anim_to_sprite_b[62], 153);
    assert_eq!(tables.hitbox_for(10, 153), Some([-5, 1, -19, 19]));

    for initial_phase in [false, true] {
        let mut obj = ObjectSim::new(vec![], 0);
        obj.configure(10);
        obj.phase = initial_phase;
        obj.fff0 = 1;
        let mut actor = Actor::default();
        actor.activity = 2;
        actor.definition = 62;
        actor.sprite = 153;
        actor.position = Position { x: 100, y: 100 };
        actor.screen = ScreenPosition { x: 88, y: 88 };
        actor.hitbox = Hitbox {
            top: -5,
            bottom: 1,
            left: -19,
            right: 19,
        };
        obj.slots[5].b = actor.to_raw_slot();
        let mut player = PlayerSim::new();
        player.x = 100;
        player.y = 67;
        player.hvel = 0x80;
        player.vvel = 0;

        let contact = MarioContact {
            x: player.x,
            y: player.y,
            hvel: 0x80,
            ..Default::default()
        };
        obj.step(100, 100, &layout, &tables, &contact, false);
        // No type before this init, so it cannot stand/carry retroactively.
        assert_eq!((obj.slots[5].b[15], obj.slots[5].b[18]), (1, 32));
        assert!(!obj.on_object);
        assert_eq!(obj.mario_x_adjust, None);

        let expected = if initial_phase {
            vec![
                (2, 1, 101, 0),
                (2, 0, 101, 8),
                (3, 1, 101, 0),
                (3, 0, 101, 8),
                (4, 1, 102, 0),
            ]
        } else {
            vec![
                (2, 0, 100, 8),
                (3, 1, 100, 0),
                (3, 0, 100, 8),
                (4, 1, 101, 0),
            ]
        };
        for (counter, phase, x, flags) in expected {
            let contact = MarioContact {
                x: player.x,
                y: player.y,
                hvel: 0x80,
                ..Default::default()
            };
            obj.step(100, 100, &layout, &tables, &contact, false);
            // Screen Y=88, top=-5: delta=98+(67-100)-(88-5-20)=2.
            // `$2DF7` raises contact but does not nudge Y for delta < 3.
            assert!(obj.on_object);
            assert_eq!((obj.fff0, obj.update_phase), (counter, phase));
            assert_eq!(obj.slots[5].b[23], flags);
            player.apply_object_result(&mut obj, false);
            assert_eq!(player.x.to_le_bytes(), [x, 0]);
            assert_eq!(player.y.to_le_bytes(), [67, 0]);
            assert!(player.on_ground_obj);
            assert!(!player.on_ground);
            assert_eq!(obj.mario_x_adjust, None);
        }

        // Last admitted pass cleared its interaction flags. Walking beyond the
        // horizontal box or jumping above it must not reuse its carry result.
        for (x, y, vvel) in [(140, 67, 0), (100, 40, 75)] {
            player.x = x;
            player.y = y;
            let contact = MarioContact {
                x,
                y,
                hvel: 0x80,
                vvel,
                ..Default::default()
            };
            obj.step(100, 100, &layout, &tables, &contact, false);
            assert!(!obj.on_object);
            assert_eq!(obj.mario_x_adjust, None);
            player.apply_object_result(&mut obj, vvel != 0);
            assert_eq!((player.x, player.y), (x, y));
            assert!(!player.on_ground_obj);
        }
        assert!(obj.unported.is_empty());
    }
}
