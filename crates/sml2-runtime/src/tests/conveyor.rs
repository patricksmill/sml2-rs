use super::*;
use crate::session::{FrameInput, Session, SessionAssets, SessionState};
use sml2_core::actor::{Actor, ContactKind, Hitbox, Position, ScreenPosition};
use sml2_core::camera::CameraSim;
use sml2_core::objects::ObjectSim;

/// `$05D5`: player, camera, contact/objects, then player/camera carry outputs.
/// A new carry cannot move the camera until the next gameplay camera pass.
#[test]
fn conveyor_live_step_applies_carry_before_next_camera_follow_and_allows_jump() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(10, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!()
    };
    ctx.layout_live = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    ctx.sim = PlayerSim::new();
    ctx.sim.x = 100;
    ctx.sim.y = 72;
    ctx.sim.hvel = 0x80;
    ctx.sim.vvel = 0;
    ctx.sim.on_ground_obj = true;
    ctx.cam = CameraSim::new(100, 72);
    ctx.cam.right_enabled = true;
    ctx.obj = ObjectSim::new(vec![], 0);
    ctx.obj.configure(10);
    ctx.obj.phase = false;
    ctx.obj.fff0 = 1;
    let mut actor = Actor::default();
    actor.activity = 2;
    actor.definition = 62;
    actor.sprite = 153;
    actor.state = 1;
    actor.contact_kind = ContactKind::from_raw(32);
    actor.position = Position { x: 100, y: 105 };
    actor.screen = ScreenPosition { x: 88, y: 121 };
    actor.hitbox = Hitbox {
        top: -5,
        bottom: 1,
        left: -19,
        right: 19,
    };
    ctx.obj.slots[5].b = actor.to_raw_slot();

    session.advance(FrameInput::default());
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!(ctx.sim.x.to_le_bytes(), [101, 0]);
    assert_eq!(ctx.sim.y.to_le_bytes(), [72, 0]);
    assert!(ctx.sim.on_ground_obj);
    assert_eq!((ctx.cam.x, ctx.cam.scroll_delta_right), (100, 1));
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (2, 1));

    session.advance(FrameInput::default());
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert_eq!((ctx.sim.x, ctx.cam.x), (101, 102));
    assert_eq!(ctx.cam.scroll_delta_right, 0);
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (2, 0));
    assert!(ctx.sim.on_ground_obj);

    session.advance(FrameInput {
        keys: PlayerKeys {
            a: true,
            ..Default::default()
        },
        div: 0,
    });
    let SessionState::InLevel(ctx) = &session.state else {
        panic!()
    };
    assert!(ctx.sim.y < 72);
    assert!(!ctx.sim.on_ground_obj);
    assert_eq!(ctx.sim.x.to_le_bytes(), [101, 0]);
    assert_eq!(ctx.obj.mario_x_adjust, None);
    assert!(ctx.obj.unported.is_empty());
}
