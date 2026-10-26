//! Frame-boundary contracts through the production native callers.
use super::*;
use crate::divider_clock::VirtualDividerClock;
use crate::session::{LevelContext, Session, SessionAssets, SessionState};
use sml2_core::objects::*;

fn level() -> LevelContext {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(0, 0).unwrap();
    let SessionState::InLevel(mut ctx) = session.state else {
        panic!("expected level")
    };
    ctx.obj = ObjectSim::new(vec![], ctx.cam.x);
    ctx.sim.music_request = 0;
    *ctx
}

fn live_tick(ctx: &mut LevelContext, start: bool) {
    ctx.step_frame(
        PlayerKeys::default(),
        false,
        false,
        start,
        false,
        0x3d,
        &load_map_nodes(),
    );
}

fn diagnostic(ctx: LevelContext, frames: u64) -> PlayOutcome {
    run_play(
        &ctx.manifest,
        &ctx.layout_live,
        &ctx.bg_live,
        &ctx.anim_rows,
        &ctx.hud,
        ctx.hud_y,
        1,
        (160, 144),
        ctx.spr,
        ctx.sim,
        ctx.cam,
        ctx.obj,
        Some(vec![0; 48]),
        Some(vec![0; 48]),
        None,
        Some(frames),
        VirtualDividerClock::new(0),
    )
    .unwrap()
}

#[test]
fn native_pause_freezes_objects_on_the_following_frame_and_preserves_sfx() {
    let mut ctx = level();
    let phase = ctx.obj.phase;
    live_tick(&mut ctx, true);
    assert_ne!(ctx.obj.phase, phase);
    let phase = ctx.obj.phase;
    assert!(ctx.sim.paused);
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (0, 0));
    assert_eq!(ctx.sim.sound_mode, 1);
    ctx.sim.sfx0 = 0x27;
    live_tick(&mut ctx, false);
    assert_eq!((ctx.obj.fff0, ctx.obj.update_phase), (0, 0));
    assert_eq!(ctx.sim.sfx0, 0x27);
    assert_eq!(ctx.obj.phase, phase);
    ctx.sim.sfx0 = 0;
    ctx.sim.on_ground = true;
    ctx.sim.hvel = 0x7f;
    live_tick(&mut ctx, false);
    assert_eq!(
        ctx.sim.sfx0, 0,
        "pose selection must not arm walking SFX while frozen"
    );
}

#[test]
fn native_transition_consumes_old_scroll_bumps_and_fire_scans_once() {
    for session in [true, false] {
        let mut ctx = level();
        ctx.cam.transition = 1;
        ctx.cam.scroll_y = 0x31;
        ctx.cam.scroll_x = 0x70;
        ctx.sim.fade_timer = 4;
        ctx.sim.below_bonk = Some((0xdf, 0x68));
        ctx.sim.bump_coin_pending = true;
        ctx.sim.fire_scan[0] = Some((0x4a, 0x4a));
        ctx.sim.fireballs[0][0] = 1;
        let slot = &mut ctx.obj.slots[0].b;
        slot[B_TYPE] = 0x80;
        slot[B_SCRY] = 0x97;
        slot[B_SCRX] = 0xf8;
        let slot = &mut ctx.obj.slots[1].b;
        slot[B_TYPE] = 0x20;
        slot[B_SCRY] = 0x50;
        slot[B_SCRX] = 0x50;
        slot[B_HB_BOTTOM] = 0x10;
        slot[B_HB_RIGHT] = 0x10;
        let (sim, obj) = if session {
            live_tick(&mut ctx, false);
            (ctx.sim, ctx.obj)
        } else {
            let out = diagnostic(ctx, 1);
            (out.sim, out.obj)
        };
        assert_eq!(sim.fade_timer, 3, "one transition fade tick");
        assert_eq!(
            sim.object_bump_special, 0xff,
            "bonk uses prior scroll before camera moves"
        );
        assert_eq!(sim.coins_low, 0);
        assert!(!sim.bump_coin_pending);
        assert_eq!(sim.below_bonk, None);
        assert_eq!(sim.fire_scan[0], None);
        assert_eq!(sim.fireballs[0][0], 0);
        assert_eq!(sim.sfx0, 2);
        assert_eq!(obj.slots[0].b[B_FLAGS], 0x20);
    }
}

#[test]
fn native_frozen_frames_still_tick_blocks_after_pose_selection() {
    for session in [true, false] {
        let mut ctx = level();
        ctx.sim.paused = true;
        ctx.sim.moon = 2;
        ctx.sim.on_ground = false;
        ctx.sim.moon_jump_pose_timer = 2;
        ctx.sim.block_slots[0][0] = 1;
        let sim = if session {
            live_tick(&mut ctx, false);
            assert_eq!(ctx.spr.as_ref().unwrap().player.unwrap().2, 51);
            ctx.sim
        } else {
            diagnostic(ctx, 1).sim
        };
        assert_eq!(sim.moon_jump_pose_timer, 1);
        assert_eq!(sim.block_slots[0][1], 1);
        assert_eq!(sim.block_frame, Some(102));
    }
}

#[test]
fn native_boss_load_keeps_music_and_compensates_one_vblank() {
    for session in [true, false] {
        let mut ctx = level();
        ctx.manifest.level_id = 5;
        ctx.cam.scene_load_pending = true;
        ctx.sim.fade_timer = 10;
        let (sim, cam, obj) = if session {
            live_tick(&mut ctx, false);
            assert_eq!(ctx.boss_definition, Some(0));
            (ctx.sim, ctx.cam, ctx.obj)
        } else {
            let out = diagnostic(ctx, 1);
            (out.sim, out.cam, out.obj)
        };
        assert_eq!(sim.fade_timer, 8, "player tick plus boss-load VBlank");
        assert_eq!((sim.music_request, sim.level_song), (32, 32));
        assert_eq!(cam.scene_state, 0xf0);
        assert!(!cam.scene_load_pending);
        assert_eq!(obj.fff0, 1);
    }
}

#[test]
fn death_and_completed_exit_preserve_native_host_boundaries() {
    let mut ctx = level();
    ctx.sim.dead = 0xff;
    ctx.sim.dead_fade = true;
    ctx.sim.fade_timer = 2;
    live_tick(&mut ctx, false);
    assert_eq!(ctx.sim.fade_timer, 1);
    assert_eq!(ctx.obj.fff0, 0);
    live_tick(&mut ctx, false);
    assert_eq!((ctx.sim.fade_timer, ctx.sim.lives), (0, 4));
    assert!(ctx.sim.map_handoff_done);

    let mut ctx = level();
    ctx.sim.dead = 0xff;
    ctx.sim.dead_fade = true;
    ctx.sim.fade_timer = 2;
    let out = diagnostic(ctx, 2);
    assert_eq!((out.sim.fade_timer, out.sim.lives, out.obj.fff0), (2, 5, 0));
    assert!(!out.sim.map_handoff_done);

    for session in [true, false] {
        let mut ctx = level();
        ctx.sim.exit_phase = true;
        ctx.sim.exit_completed = true;
        ctx.sim.fade_timer = 3;
        let sim = if session {
            live_tick(&mut ctx, false);
            ctx.sim
        } else {
            diagnostic(ctx, 1).sim
        };
        assert_eq!(sim.fade_timer, if session { 2 } else { 3 });
        assert!(sim.map_handoff_done);
    }
}

#[test]
fn player_events_consume_both_bumps_without_crediting_the_fireball() {
    use crate::game::level_frame::resolve_player_events;
    let mut ctx = level();
    ctx.cam.scroll_y = 0x31;
    ctx.cam.scroll_x = 0x70;
    ctx.sim.below_bonk = Some((0xdf, 0x68));
    ctx.sim.fire_below_bonk = Some((0xdf, 0x68));
    ctx.sim.bump_coin_pending = true;
    resolve_player_events(&mut ctx.sim, &ctx.cam, &mut ctx.obj);
    assert_eq!(ctx.sim.coins_low, 1);
    assert_eq!(ctx.sim.sfx0, 7);
    assert_eq!(ctx.sim.block_slots[0][0], 4);
    assert_eq!((ctx.sim.below_bonk, ctx.sim.fire_below_bonk), (None, None));
    resolve_player_events(&mut ctx.sim, &ctx.cam, &mut ctx.obj);
    assert_eq!(ctx.sim.coins_low, 1);
    assert_eq!(ctx.sim.block_slots.iter().filter(|s| s[0] != 0).count(), 1);
}

#[test]
fn replay_contact_survives_the_mutating_pose_checkpoint() {
    use crate::game::level_frame::{prepare_object_step, step_objects};
    for invincibility in [0, 1] {
        let mut ctx = level();
        ctx.cam = sml2_core::camera::CameraSim::new(336, 88);
        ctx.sim.x = 258;
        ctx.sim.y = 120;
        ctx.sim.vvel = 24;
        ctx.sim.on_ground = false;
        ctx.sim.invincibility = invincibility;
        let slot = &mut ctx.obj.slots[0].b;
        slot[B_TYPE] = 0x40;
        slot[B_SCRX] = 18;
        slot[B_SCRY] = 144;
        let observation = crate::replay::observe_player_pose(
            &mut ctx.sim,
            &ctx.cam,
            &PlayerKeys::default(),
            false,
            false,
        );
        let contact = observation.contact;
        assert_eq!(ctx.sim.invincibility, 0);
        prepare_object_step(&mut ctx.sim, &mut ctx.cam, &mut ctx.obj, 0, 0x3d);
        step_objects(
            &mut ctx.sim,
            &mut ctx.cam,
            &mut ctx.obj,
            &ctx.layout_live,
            &ctx.spr.as_ref().unwrap().tables,
            &contact,
            false,
        );
        assert_eq!(ctx.obj.slots[0].b[B_FLAGS], 0x10);
        assert_eq!(ctx.sim.dead, if invincibility == 0 { 15 } else { 0 });
        assert_eq!(ctx.sim.sfx0, if invincibility == 0 { 15 } else { 0 });
        assert!(!ctx.sim.on_ground);
        assert!(!ctx.sim.on_ground_obj);
        assert_eq!(ctx.obj.div, 0x3d);
    }
}

#[test]
fn replay_boss_preparation_leaves_fade_compensation_to_the_host() {
    use crate::game::level_frame::prepare_object_step;
    let mut ctx = level();
    ctx.cam.scene_load_pending = true;
    ctx.sim.fade_timer = 9;
    assert_eq!(
        prepare_object_step(&mut ctx.sim, &mut ctx.cam, &mut ctx.obj, 5, 0x78),
        Some(0)
    );
    assert_eq!(ctx.sim.fade_timer, 9);
    assert_eq!((ctx.sim.music_request, ctx.sim.level_song), (32, 32));
    assert_eq!(ctx.cam.scene_state, 0xf0);
    assert_eq!(
        prepare_object_step(&mut ctx.sim, &mut ctx.cam, &mut ctx.obj, 5, 0x79),
        None
    );
    assert_eq!(ctx.obj.div, 0x79);
    ctx.cam.scene_load_pending = true;
    ctx.cam.boss_fight_phase = 9;
    assert_eq!(
        prepare_object_step(&mut ctx.sim, &mut ctx.cam, &mut ctx.obj, 5, 0),
        None
    );
    assert!(!ctx.cam.scene_load_pending);
    assert!(ctx.obj.unported.contains(&0x0EB0));
}

#[test]
fn replay_pose_checkpoint_keeps_its_fade_block_and_sound_contract() {
    for (was_dead, fade, frame, frame_check, block_tick, invincibility, sfx) in [
        (false, false, 13, 102, 1, 0, 4),
        (false, true, 13, 13, 0, 0, 4),
        (true, false, 107, 107, 0, 1, 0),
    ] {
        let mut ctx = level();
        ctx.sim.on_ground = true;
        ctx.sim.hvel = 0x7f;
        ctx.sim.invincibility = 1;
        ctx.sim.block_slots[0][0] = 1;
        let observation = crate::replay::observe_player_pose(
            &mut ctx.sim,
            &ctx.cam,
            &PlayerKeys::default(),
            was_dead,
            fade,
        );
        assert_eq!(
            (observation.frame, observation.frame_check),
            (frame, frame_check)
        );
        assert_eq!(observation.contact.invincibility, 1);
        assert_eq!(ctx.sim.block_slots[0][1], block_tick);
        assert_eq!(ctx.sim.invincibility, invincibility);
        assert_eq!(ctx.sim.sfx0, sfx);
    }
}

#[test]
fn native_death_trigger_runs_objects_before_the_following_frame_freezes() {
    for session in [true, false] {
        let mut ctx = level();
        ctx.sim.dead = 0xff;
        ctx.sim.vvel = 1;
        ctx.sim.y = (ctx.cam.y & 0xff00) | u16::from((ctx.cam.y as u8).wrapping_add(77));
        let (sim, obj, death_frames) = if session {
            live_tick(&mut ctx, false);
            assert!(ctx.sim.dead_fade);
            assert_eq!(ctx.sim.fade_timer, 192);
            assert!(!ctx.obj.phase, "trigger frame runs objects");
            live_tick(&mut ctx, false);
            (ctx.sim, ctx.obj, ctx.gameplay_death_frames as u64)
        } else {
            let out = diagnostic(ctx, 2);
            (out.sim, out.obj, out.gameplay_death_frames)
        };
        assert!(sim.dead_fade);
        assert_eq!(sim.fade_timer, if session { 191 } else { 192 });
        assert!(!obj.phase, "second frame leaves the object clock frozen");
        assert_eq!((obj.fff0, obj.update_phase), (0, 0));
        assert_eq!(death_frames, 1);
        assert_eq!(sim.music_request, 2);
    }
}

#[test]
fn replay_installs_the_current_edge_after_crouch_look() {
    use sml2_core::{camera::CameraSim, level::LAYOUT_SIZE};
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    for (previous_edge, captured_edge, expected_y) in [(8, 0, 0x1c1), (0, 8, 0x1c0)] {
        let mut sim = PlayerSim::new();
        sim.x = 80;
        sim.y = 0x1c0;
        sim.on_ground = true;
        let mut cam = CameraSim::new(80, 0x1c0);
        cam.prev_mario_y_lo = 0xc0;
        cam.edge_now = Some(previous_edge);
        let mut obj = ObjectSim::new(vec![], 80);
        crate::replay::step_replay_player_camera(
            &mut sim,
            &mut cam,
            &mut obj,
            &layout,
            EndingInput {
                held: PlayerKeys {
                    b: true,
                    down: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            Some(captured_edge),
        );
        assert_eq!(cam.y.to_le_bytes(), (expected_y as u16).to_le_bytes());
        assert_eq!(cam.edge_now, Some(captured_edge));
    }
}
