use super::*;
use sml2_core::camera::CameraSim;

/// Decimal CLI level 10 uses extracted level11. Exercise both linked mouths
/// with normal movement, without arming a pipe or scene directly.
#[test]
fn level_10_first_pipe_exits_into_the_collision_room() {
    pipe_roundtrip(false);
    pipe_roundtrip(true);
}

/// `$05D5` -> `$0B2A` -> `$20A4` must claim the scene on the edge frame,
/// then `$2323` writes both Y pages on the first resume. Exercise the live
/// session caller so a helper-only test cannot hide an extra gameplay step.
#[test]
fn live_session_pipe_page_write_preserves_low_bytes_and_suspends_gameplay() {
    use crate::session::{FrameInput, Session, SessionAssets, SessionState};

    fn level(session: &Session) -> &crate::session::LevelContext {
        match &session.state {
            SessionState::InLevel(ctx) => ctx,
            _ => panic!("expected the pipe's live level"),
        }
    }

    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(10, 0).unwrap();
    if let SessionState::InLevel(ctx) = &mut session.state {
        ctx.sim.x = 0x1F0;
        ctx.sim.y = 0xA4;
        ctx.sim.phase = 0;
        ctx.sim.pipe_dir = 0x30;
        ctx.sim.on_ground = true;
        ctx.sim.hvel = 0x80;
        ctx.cam = CameraSim::new(0x1B0, 0xC3);
        ctx.cam.prev_mario_y_lo = 0xA4;
        ctx.cam.update_enables(ctx.sim.x);
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extracted/levels");
        ctx.cam.edge_map.copy_from_slice(&fs::read(root.join("level11.edge.bin")).unwrap());
        ctx.cam.scene_map = fs::read(root.join("level11.scene.bin")).unwrap();
    }
    let input = FrameInput {
        keys: PlayerKeys { right: true, ..Default::default() },
        div: 0,
    };
    // The first pass reads the prior draw at screen X $A0, even though
    // this frame crosses $A1. Its draw publishes the next pass's latch.
    session.advance(input);
    let ctx = level(&session);
    assert_eq!((ctx.cam.transition, ctx.cam.scene_id), (0, 0));
    assert!(ctx.cam.mario_screen_x >= 0xA1);
    assert!(!ctx.scene_vm.owns_gameplay());
    session.advance(input);
    let ctx = level(&session);
    assert_eq!((ctx.cam.transition, ctx.cam.scene_id), (1, 17));
    assert!(ctx.scene_vm.owns_gameplay());
    let source = (ctx.sim.x, ctx.sim.y, ctx.cam.x, ctx.cam.y);
    let clock = (ctx.obj.fff0, ctx.obj.update_phase);
    let timer = ctx.sim.level_timer;

    // `$20A4`'s single prologue wait has completed; scene 17 is `40 21 FF`.
    session.advance(input);
    let ctx = level(&session);
    assert_eq!(ctx.sim.x, source.0);
    assert_eq!(ctx.cam.x, source.2);
    assert_eq!(ctx.sim.y.to_le_bytes(), [source.1 as u8, 2]);
    assert_eq!(ctx.cam.y.to_le_bytes(), [source.3 as u8, 2]);
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), clock);
    assert_eq!(ctx.sim.level_timer, timer);
    assert_eq!(ctx.cam.scene_transition_flag, 0);
    let destination = (ctx.sim.x, ctx.sim.y, ctx.cam.x, ctx.cam.y);

    // `$2323` own wait, two streamer waits and the `$21E8` tail wait.
    // Each advance resumes one wait; END returns ownership after the last.
    for _ in 0..3 {
        session.advance(input);
        let ctx = level(&session);
        assert!(ctx.scene_vm.owns_gameplay());
        assert_eq!((ctx.sim.x, ctx.sim.y, ctx.cam.x, ctx.cam.y), destination);
        assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), clock);
        assert_eq!(ctx.sim.level_timer, timer);
    }
    assert_eq!(level(&session).cam.scene_transition_flag, 1);
    session.advance(input);
    let ctx = level(&session);
    assert!(!ctx.scene_vm.owns_gameplay());
    assert_eq!((ctx.cam.scene_id, ctx.cam.scene_transition_latch), (0, 1));
    assert_eq!((ctx.sim.x, ctx.sim.y, ctx.cam.x, ctx.cam.y), destination);

    session.advance(input);
    let ctx = level(&session);
    assert_eq!(ctx.sim.x, destination.0 + 1);
    assert_eq!(ctx.cam.x, destination.2 + 4);
    assert_eq!((ctx.sim.y, ctx.cam.y), (destination.1, destination.3));
    assert_ne!(ctx.obj.update_phase, clock.1);
}

fn pipe_roundtrip(reverse: bool) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extracted");
    let layout =
        LevelLayout::from_bytes(&fs::read(root.join("levels/level11.decompressed.bin")).unwrap())
            .unwrap();
    let scripts =
        SceneScripts::from_bytes(&fs::read(root.join("scenes/script_table.bin")).unwrap()).unwrap();
    let mut cam = CameraSim::new(
        if reverse { 0x250 } else { 0x1B0 },
        if reverse { 0x2C0 } else { 0xC0 },
    );
    cam.edge_map
        .copy_from_slice(&fs::read(root.join("levels/level11.edge.bin")).unwrap());
    cam.scene_map = fs::read(root.join("levels/level11.scene.bin")).unwrap();
    cam.prev_mario_y_lo = if reverse { 164 } else { 148 };
    let mut sim = PlayerSim::new();
    sim.x = if reverse { 530 } else { 460 };
    sim.y = if reverse { 676 } else { 148 };
    sim.on_ground = true;
    sim.hvel = 0x80;
    sim.fade_timer = 200;
    sim.level_timer = sml2_core::level_timer::LevelTimer::new(4);
    let mut scene = SceneVm::new();
    let mut ending = EndingEntry::default();
    let keys = PlayerKeys {
        right: !reverse,
        left: reverse,
        ..Default::default()
    };
    let mut entered = false;
    let mut teleported = false;
    for _ in 0..160 {
        sim.camx = cam.x;
        sim.camy = cam.y;
        sim.phase = (sim.phase + 1) & 7;
        sim.frame_counter = sim.frame_counter.wrapping_add(1);
        let owned = scene.owns_gameplay();
        let timer = sim.level_timer;
        let before = (sim.x, sim.y, cam.x, cam.y, sim.fade_timer);
        let frame = scene_frame_head(
            &mut scene,
            &mut ending,
            &mut sim,
            &mut cam,
            EndingInput {
                held: keys,
                pressed: PlayerKeys::default(),
            },
        );
        assert_eq!(
            frame.is_none(),
            owned,
            "scene owns the complete gameplay dispatch"
        );
        if matches!(frame, Some(EndingFrame::Gameplay(_))) {
            if cam.transition == 0 {
                sim.step(&keys, false, false, &layout);
            } else {
                sim.tick_fade_timer();
            }
            cam.step(&mut sim);
            cam.update_enables(sim.x);
        }
        if frame.is_none() {
            assert_eq!(sim.level_timer, timer, "the countdown belongs to gameplay");
            assert_eq!(
                (sim.x, sim.y, cam.x, cam.y),
                (before.0, before.1, before.2, before.3)
            );
            assert_eq!(
                sim.fade_timer,
                before.4.saturating_sub(1),
                "VBlank fade continues"
            );
        }
        let step = scene.step(&mut sim, &mut cam, &scripts);
        entered |= sim.pipe_dir == if reverse { 0x40 } else { 0x30 };
        teleported |= step == SceneStep::Ran(0x40);
        if teleported && cam.transition == 0 && sim.pipe_dir == 0 {
            assert!(entered);
            assert_eq!(
                (sim.x, sim.y),
                if reverse { (466, 164) } else { (541, 676) },
                "pipe exit belongs beside the destination mouth"
            );
            assert_eq!(
                (cam.x, cam.y),
                if reverse {
                    (0x1B0, 0xC0)
                } else {
                    (0x250, 0x2C0)
                }
            );
            assert!(sim.on_ground);
            // Ordinary physics must read the destination's resident collision
            // map after travel clears, including the $3B platform upstairs
            // and $18 bottom floor downstairs.
            sim.hvel = 0x80;
            for _ in 0..40 {
                sim.camx = cam.x;
                sim.camy = cam.y;
                sim.step(&PlayerKeys::default(), false, false, &layout);
            }
            assert!(sim.on_ground, "destination floor must catch Mario");
            assert_eq!(sim.collision_tile, if reverse { 0x3B } else { 0x18 });
            assert_eq!(sim.y, if reverse { 164 } else { 724 });
            return;
        }
    }
    panic!("first pipe did not finish");
}
