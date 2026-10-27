//! Pose `$0618` reads the preceding `$02:$4100` reference bytes.
use super::*;
use crate::session::{FrameInput, Session, SessionAssets, SessionState};

fn session() -> Session {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0x0c, 0).unwrap();
    session
}

fn level(session: &mut Session) -> &mut LevelContext {
    match &mut session.state {
        SessionState::InLevel(ctx) => ctx,
        _ => panic!("expected a level"),
    }
}

#[test]
fn regional_palette_consumes_staged_bytes_before_objects_replace_them() {
    let mut game = session();
    let ctx = level(&mut game);
    ctx.sim.x = 0x0538;
    ctx.sim.y = 0x0100;
    ctx.obj.mario_ref_x = 0x0240;
    ctx.obj.mario_ref_y = 0x0120;
    game.advance(FrameInput::default());
    let ctx = level(&mut game);
    assert_eq!(ctx.dx_regional.selector, 1);
    assert_eq!(ctx.dx_regional.bg, Some(0x6140));
    assert_eq!(ctx.obj.mario_ref_x >> 8, 5);
    game.advance(FrameInput::default());
    let ctx = level(&mut game);
    assert_eq!(ctx.dx_regional.selector, 2);
    assert_eq!(ctx.dx_regional.bg, Some(0x6180));
}

#[test]
fn paused_and_fade_frames_retain_selector_and_staged_bytes() {
    for freeze in 0..3 {
        let mut game = session();
        let ctx = level(&mut game);
        ctx.obj.mario_ref_x = 0x0240;
        ctx.obj.mario_ref_y = 0x0120;
        ctx.sim.fade_timer = 0x100;
        match freeze {
            0 => ctx.sim.paused = true,
            1 => ctx.sim.dead_fade = true,
            _ => ctx.sim.exit_phase = true,
        }
        game.advance(FrameInput::default());
        let ctx = level(&mut game);
        assert_eq!(ctx.dx_regional.selector, 0, "freeze {freeze}");
        assert_eq!(ctx.dx_regional.bg, None);
        assert_eq!(ctx.obj.mario_ref_x, 0x0240);
        assert_eq!(ctx.obj.mario_ref_y, 0x0120);
    }
}

#[test]
fn dead_draw_skips_selector_even_when_object_references_advance() {
    let mut game = session();
    let ctx = level(&mut game);
    ctx.obj.mario_ref_x = 0x0240;
    ctx.obj.mario_ref_y = 0x0120;
    ctx.sim.dead = 0xff;
    game.advance(FrameInput::default());
    assert_eq!(level(&mut game).dx_regional.bg, None);
}

#[test]
fn a_real_level_load_resets_regional_state() {
    let mut game = session();
    level(&mut game)
        .dx_regional
        .update_references(0x0c, 0x0240, 0x0120);
    game.begin_level(0x0c, 0).unwrap();
    assert_eq!(level(&mut game).dx_regional, Default::default());
}

#[test]
fn a_blocking_scene_transfer_retains_staged_palette_references() {
    let mut game = session();
    let ctx = level(&mut game);
    ctx.obj.mario_ref_x = 0x0240;
    ctx.obj.mario_ref_y = 0x0120;
    ctx.cam.scene_id = 4;
    ctx.scene_vm.start();
    for _ in 0..4 {
        game.advance(FrameInput::default());
        let ctx = level(&mut game);
        assert_eq!(ctx.dx_regional.bg, None);
        assert_eq!(ctx.obj.mario_ref_x.to_le_bytes(), [0x40, 2]);
        assert_eq!(ctx.obj.mario_ref_y.to_le_bytes(), [0x20, 1]);
    }
}
