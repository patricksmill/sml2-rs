//! Local co-op behavior at the edges the single-player ROM never exercises:
//! independent players, shared camera clamping, bubbles/revival and checkpoint
//! wipes.

use crate::session::{
    Bubble, FrameEffects, LevelContext, Session, SessionAssets, SessionState, StepOutcome,
};
use sml2_core::camera::CameraSim;
use sml2_core::objects::{
    B_DEF, B_FSTATE, B_HB_BOTTOM, B_HB_LEFT, B_HB_RIGHT, B_HB_TOP, B_PHASE, B_SCRX, B_SCRY,
    B_SPRITE, B_STATE, B_T24, B_TYPE, ObjectSim,
};
use sml2_core::player_sim::{PlayerKeys, PlayerSim};

fn live(session: &mut Session) -> &mut LevelContext {
    match &mut session.state {
        SessionState::InLevel(ctx) => ctx.as_mut(),
        _ => panic!("expected a live level"),
    }
}

/// World position whose `$1BFF` probes read cell `(col, row)`.
fn cell_player(col: usize, row: usize) -> (u16, u16) {
    (
        (col * 16).wrapping_sub(8) as u16,
        (row * 16).wrapping_sub(12) as u16,
    )
}

fn find_cell(ctx: &LevelContext, test: impl Fn(u8) -> bool) -> Option<(usize, usize)> {
    let wide = ctx.layout_live.world.cells_wide();
    let high = ctx.layout_live.world.cells_high();
    ctx.layout_live
        .cells
        .iter()
        .enumerate()
        .find_map(|(i, &tile)| test(tile).then_some((i % wide, i / wide)))
        .filter(|(_, row)| *row < high)
}

fn place_player(sim: &mut sml2_core::player_sim::PlayerSim, col: usize, row: usize) {
    let (x, y) = cell_player(col, row);
    sim.x = x;
    sim.y = y;
    sim.dead = 0;
    sim.dead_fade = false;
    sim.exit_phase = false;
    sim.exit_completed = false;
    sim.powerup = 0;
    sim.invincibility = 0;
    sim.star_high = 0;
    sim.on_ground = false;
    sim.vvel = 24;
    sim.hvel = 0x80;
}

fn step(session: &mut Session, first: PlayerKeys, second: PlayerKeys) -> FrameEffects {
    session.advance_two_players(
        crate::session::FrameInput {
            keys: first,
            div: 0x3d,
        },
        second,
    )
}

fn coop_session() -> Session {
    let mut session = Session::new(SessionAssets::load().expect("session assets"));
    session.set_local_multiplayer(true);
    session.begin_level(0, 0).expect("level 1 loads");
    session
}

#[test]
fn a_block_either_player_jumps_into_finishes_its_bump_as_a_used_block() {
    for bumper in 0..2 {
        let mut session = coop_session();
        let ctx = live(&mut session);
        let wide = ctx.layout_live.world.cells_wide();
        let at = |c: usize, r: usize| ctx.layout_live.cells.get(r * wide + c).copied();
        // A bumpable block with open air and then ground below it.
        let found = (0..ctx.layout_live.cells.len()).find_map(|i| {
            let (c, r) = (i % wide, i / wide);
            if !at(c, r).is_some_and(|t| t < 0x04) {
                return None;
            }
            (2..=4).find_map(|gap| {
                ((1..=gap).all(|k| at(c, r + k).is_some_and(|t| t >= 0x38))
                    && at(c, r + gap + 1).is_some_and(|t| t < 0x38))
                .then_some((c, r, gap))
            })
        });
        let (col, row, gap) = found.expect("level 1 has a block over open ground");
        let key = ctx.layout_live.world.cell_key(col, row).unwrap();
        {
            let sim = if bumper == 0 {
                &mut ctx.sim
            } else {
                &mut ctx.second.as_mut().unwrap().sim
            };
            let (x, y) = cell_player(col, row + gap);
            sim.x = x + 4;
            sim.y = y;
            sim.on_ground = true;
        }
        // Bring the shared screen (and the other player) to the block.
        let (x, y) = cell_player(col, row + gap);
        ctx.cam.x = x + 8;
        ctx.cam.y = y + 96 - 0x40;
        let other = if bumper == 0 {
            &mut ctx.second.as_mut().unwrap().sim
        } else {
            &mut ctx.sim
        };
        other.x = x + 40;
        other.y = y;
        let jump = PlayerKeys {
            a: true,
            ..Default::default()
        };
        let mut seen = Vec::new();
        for frame in 0..120 {
            let keys = if (2..20).contains(&frame) {
                jump
            } else {
                PlayerKeys::default()
            };
            if bumper == 0 {
                step(&mut session, keys, PlayerKeys::default());
            } else {
                step(&mut session, PlayerKeys::default(), keys);
            }
            let ctx = live(&mut session);
            let cell = ctx
                .sim
                .layout_writes
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| *v);
            if let Some(v) = cell {
                seen.push(v);
            }
            if cell == Some(24) {
                // `$5BFC`'s blank cell is covered by the bouncing block sprite.
                assert!(
                    !ctx.spr.as_ref().unwrap().bumpers.is_empty(),
                    "player {bumper}'s bump sprite is drawn while the cell is blank"
                );
            }
        }
        assert!(!seen.is_empty(), "player {bumper} reached the block");
        assert_ne!(
            seen.last(),
            Some(&24),
            "player {bumper}'s bump finished: {seen:?}"
        );
    }
}

#[test]
fn a_coin_player_two_takes_is_gone_for_both_players() {
    let mut session = coop_session();
    let ctx = live(&mut session);
    let (col, row) = find_cell(ctx, |tile| tile == 0x48).expect("level 1 has a coin");
    let key = ctx.layout_live.world.cell_key(col, row).unwrap();
    let walk_in = |session: &mut Session| {
        let ctx = live(session);
        let second = &mut ctx.second.as_mut().unwrap().sim;
        // The right side probe (`x+20`, `y+32`) reads cell (col, row).
        second.x = (col * 16) as u16 - 12;
        second.y = (row * 16) as u16 - 16;
        second.dead = 0;
        let right = PlayerKeys {
            right: true,
            ..Default::default()
        };
        step(session, PlayerKeys::default(), right);
        let ctx = live(session);
        (
            ctx.sim.coins_low,
            ctx.sim
                .layout_writes
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| *v),
        )
    };
    let before = live(&mut session).sim.coins_low;
    let (coins, cell) = walk_in(&mut session);
    assert_ne!(coins, before, "player 2 collected the coin");
    assert_eq!(cell, Some(0x60), "the shared map lost the coin");
    let (again, _) = walk_in(&mut session);
    assert_eq!(again, coins, "the same coin cannot be collected twice");
}

#[test]
fn the_party_spawns_side_by_side_around_the_entry_point() {
    let mut session = coop_session();
    let ctx = live(&mut session);
    let second = &ctx.second.as_ref().unwrap().sim;
    assert_eq!(ctx.sim.x.wrapping_sub(second.x), 24, "24 px apart");
    assert_eq!(ctx.sim.y, second.y);
}

#[test]
fn shared_vertical_bounds_allow_a_reach_above_the_top_and_kill_below_the_view() {
    let cam = CameraSim::new(0x0200, 0x0100);
    let mut above = PlayerSim::new();
    above.y = cam.y.wrapping_sub(80);
    crate::game::level_frame::clamp_shared_vertical(&mut above, &cam);
    assert_eq!(
        above.y,
        cam.y.wrapping_sub(80),
        "16 px above the top is allowed"
    );

    let mut far_above = PlayerSim::new();
    far_above.y = cam.y.wrapping_sub(120);
    crate::game::level_frame::clamp_shared_vertical(&mut far_above, &cam);
    assert_eq!(far_above.y, cam.y.wrapping_sub(96));
    assert!(!far_above.on_ground);
    assert_eq!(far_above.dead, 0);

    let mut below = PlayerSim::new();
    below.y = cam.y.wrapping_add(80);
    crate::game::level_frame::clamp_shared_vertical(&mut below, &cam);
    assert_eq!(below.dead, 0xFF, "the ROM death line is the bottom bound");
}

#[test]
fn screen_edges_hold_players_and_report_a_push() {
    // An all-air layout: no wall stands behind either player.
    let mut layout = live(&mut coop_session()).layout_live.clone();
    layout.cells.fill(0x60);
    let blank = |x: u16| {
        let mut p = PlayerSim::new();
        p.camx = 0x0400;
        p.x = x;
        p.y = 0;
        p
    };
    let left_edge = 0x0400 - 96 + 0x10;
    let held_left = PlayerKeys {
        left: true,
        ..Default::default()
    };
    let mut p = blank(left_edge - 3);
    p.dir_right = false;
    p.hvel = 0x10;
    let edge = crate::game::level_frame::apply_screen_edges(&mut p, &layout, &held_left);
    assert_eq!(p.x, left_edge, "put back on the left edge");
    assert_eq!(p.hvel, 0, "no speed into the edge");
    assert!(edge.pressing_left && !edge.off_screen);

    let mut p = blank(left_edge + 0x90 + 5);
    let edge =
        crate::game::level_frame::apply_screen_edges(&mut p, &layout, &PlayerKeys::default());
    assert_eq!(p.x, left_edge + 0x90, "put back on the right edge");
    assert!(!edge.pressing_right);
}

#[test]
fn a_wall_behind_a_player_lets_the_edge_leave_it_off_screen() {
    let mut session = coop_session();
    let ctx = live(&mut session);
    let (col, row) = find_cell(ctx, |tile| tile < 0x38).expect("level 1 has solid ground");
    // The right side probe (`x+20`, `y+32`) reads cell (col, row).
    let mut p = PlayerSim::new();
    p.world = ctx.layout_live.world;
    p.x = (col * 16) as u16 - 12;
    p.y = (row * 16) as u16 - 16;
    assert!(p.wall_beside(&ctx.layout_live, true));
    p.camx = p.x + 96 - 0x10 + 20;
    let x = p.x;
    let edge = crate::game::level_frame::apply_screen_edges(
        &mut p,
        &ctx.layout_live,
        &PlayerKeys::default(),
    );
    assert_eq!(p.x, x, "not pushed into the wall");
    assert!(edge.off_screen, "20 px past the edge with a wall behind");
}

#[test]
fn players_move_independently_inside_the_shared_screen() {
    let mut session = coop_session();
    let start_x = live(&mut session).sim.x;
    let right = PlayerKeys {
        right: true,
        ..Default::default()
    };
    let jump = PlayerKeys {
        a: true,
        ..Default::default()
    };
    // Player 1 walks right; player 2 jumps straight up on the first frame.
    for frame in 0..20 {
        let _ = step(
            &mut session,
            right,
            if frame < 8 {
                jump
            } else {
                PlayerKeys::default()
            },
        );
    }
    let ctx = live(&mut session);
    assert!(ctx.sim.x > start_x, "player 1 walked right");
    assert!(
        ctx.second.as_ref().is_some_and(|s| s.sim.x != ctx.sim.x),
        "the players did not move as one body"
    );
    let shared = |x: u16| x.wrapping_sub(ctx.cam.x).wrapping_add(96);
    for player in [&ctx.sim, &ctx.second.as_ref().unwrap().sim] {
        let sx = shared(player.x);
        assert!(
            (0x10..=0xA0).contains(&sx),
            "player screen X {sx:#04X} stayed inside the shared screen"
        );
    }
    let spr = ctx.spr.as_ref().unwrap();
    assert!(spr.player.is_some());
    assert!(spr.player_two.is_some());
    let frame = session.render_frame((160, 144));
    assert_eq!(frame.len(), 160 * 144);
}

#[test]
fn wide_view_players_reach_past_the_native_screen() {
    let mut session = coop_session();
    session.set_view((256, 144));
    let _ = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
    // Player 1 stands past the native `$A0` clamp but inside the 256-wide view.
    {
        let ctx = live(&mut session);
        let view = ctx.sim.view.expect("enlarged view");
        ctx.sim.x = (view.x + view.w - 40) as u16;
    }
    let _ = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
    let ctx = live(&mut session);
    let view = ctx.sim.view.expect("enlarged view");
    let screen_x = ctx.sim.x.wrapping_sub(ctx.cam.x).wrapping_add(96);
    assert!(
        screen_x > 0xB0,
        "player 1 kept screen X {screen_x:#04X}, past the native $A0 clamp"
    );
    let at = i32::from(ctx.sim.x) - view.x;
    assert!(
        (0..=view.w - 16).contains(&at),
        "player 1 stays inside the view"
    );
}

fn platform_anim(ctx: &LevelContext) -> u8 {
    let tables = &ctx.sprite_ctx().unwrap().tables;
    (0..=255u8)
        .find(|&anim| {
            tables
                .profile_for(ctx.manifest.level_id, anim)
                .is_some_and(|profile| profile.handler == 0x58C52)
        })
        .expect("a horizontal-patrol anim exists")
}

fn set_platform_slot(
    slot: &mut [u8; 32],
    anim: u8,
    state: u8,
    sprite: u8,
    world: (u16, u16),
    screen: (u8, u8),
) {
    let mut b = [0u8; 32];
    b[0..2].copy_from_slice(&world.0.to_be_bytes());
    b[3..5].copy_from_slice(&world.1.to_be_bytes());
    b[B_DEF] = anim;
    b[B_SPRITE] = sprite;
    b[B_STATE] = 2;
    b[B_FSTATE] = state;
    b[B_PHASE] = 0;
    b[B_T24] = 36;
    b[B_TYPE] = 0x20;
    b[B_HB_TOP] = 0;
    b[B_HB_BOTTOM] = 16;
    b[B_HB_LEFT] = 0;
    b[B_HB_RIGHT] = 16;
    b[B_SCRX] = screen.0;
    b[B_SCRY] = screen.1;
    *slot = b;
}

fn platform_screen(world: (u16, u16), cam: (u16, u16)) -> (u8, u8) {
    (
        world.0.wrapping_sub(cam.0).wrapping_add(88) as u8,
        world.1.wrapping_sub(cam.1).wrapping_add(88) as u8,
    )
}

fn park(sim: &mut PlayerSim, x: u16, y: u16) {
    sim.x = x;
    sim.y = y;
    sim.dead = 0;
    sim.dead_fade = false;
    sim.exit_phase = false;
    sim.exit_completed = false;
    sim.powerup = 0;
    sim.invincibility = 0;
    sim.star_high = 0;
    sim.pipe_dir = 0;
    sim.hvel = 0x80;
    sim.vvel = 0;
    sim.on_ground = false;
    sim.on_ground_obj = false;
}

fn unstick(sim: &mut PlayerSim) {
    sim.vvel = 0;
    sim.on_ground = false;
    sim.on_ground_obj = false;
}

fn platform_tick(session: &mut Session) {
    let ctx = live(session);
    unstick(&mut ctx.sim);
    unstick(&mut ctx.second.as_mut().unwrap().sim);
    let effects = session.advance_two_players(
        crate::session::FrameInput {
            keys: PlayerKeys::default(),
            div: 0,
        },
        PlayerKeys::default(),
    );
    assert_eq!(effects.outcome, StepOutcome::Continue);
}

fn platform_world(
    session: &mut Session,
    first: (u16, u16),
    second: (u16, u16),
    slots: &[(usize, u8, (u16, u16))],
) {
    let anim = {
        let ctx = live(session);
        platform_anim(ctx)
    };
    let ctx = live(session);
    ctx.obj = ObjectSim::new(Vec::new(), ctx.cam.x);
    ctx.obj.configure(ctx.manifest.level_id);
    let sprite = if ctx.obj.sprite_map_index == 0 {
        ctx.sprite_ctx()
            .unwrap()
            .tables
            .anim_to_sprite_a
            .get(anim as usize)
            .copied()
            .unwrap_or(0)
    } else {
        ctx.sprite_ctx()
            .unwrap()
            .tables
            .anim_to_sprite_b
            .get(anim as usize)
            .copied()
            .unwrap_or(0)
    };
    ctx.obj.world = ctx.layout_live.world;
    // No terrain floor under the synthetic riders; their only ground is the
    // platform slot under test.
    ctx.layout_live.cells.fill(0xFF);
    ctx.obj.fff0 = 2;
    ctx.obj.phase = false;
    ctx.obj.update_phase = 0;
    ctx.obj.prev_cam_x = 0x100;
    ctx.cam.x = 0x100;
    ctx.cam.y = 0x100;
    ctx.cam.transition = 0;
    ctx.cam.scene_id = 0;
    ctx.cam.scene_state = 0;
    ctx.cam.edge_map.fill(0);
    ctx.cam.prev_mario_y_lo = first.1 as u8;
    ctx.sim.camx = ctx.cam.x;
    ctx.sim.camy = ctx.cam.y;
    park(&mut ctx.sim, first.0, first.1);
    let second_sim = &mut ctx.second.as_mut().unwrap().sim;
    second_sim.camx = ctx.cam.x;
    second_sim.camy = ctx.cam.y;
    park(second_sim, second.0, second.1);
    ctx.cam.update_enables(first.0);
    let hitbox = ctx
        .sprite_ctx()
        .unwrap()
        .tables
        .hitbox_for(ctx.manifest.level_id, sprite)
        .unwrap();
    for &(index, state, world) in slots {
        let screen = platform_screen(world, (ctx.cam.x, ctx.cam.y));
        set_platform_slot(
            &mut ctx.obj.slots[index].b,
            anim,
            state,
            sprite,
            world,
            screen,
        );
        for (field, value) in [B_HB_TOP, B_HB_BOTTOM, B_HB_LEFT, B_HB_RIGHT]
            .into_iter()
            .zip(hitbox)
        {
            ctx.obj.slots[index].b[field] = value as u8;
        }
        ctx.obj.slot_done[index] = false;
    }
}

fn position(session: &Session) -> ((u16, u16), (u16, u16)) {
    match &session.state {
        SessionState::InLevel(ctx) => (
            (ctx.sim.x, ctx.sim.y),
            (
                ctx.second.as_ref().unwrap().sim.x,
                ctx.second.as_ref().unwrap().sim.y,
            ),
        ),
        _ => panic!("expected a live level"),
    }
}

#[test]
fn player_two_alone_rides_a_platform_without_moving_player_one() {
    let mut session = coop_session();
    platform_world(
        &mut session,
        (0x120, 0x100),
        (0x100, 0x100),
        &[(0, 1, (0x100, 0x128))],
    );
    for _ in 0..2 {
        platform_tick(&mut session);
    }
    let ((p1x, _), (p2x, _)) = position(&session);
    assert_eq!(p1x, 0x120, "player 1 must not be carried by P2's slot");
    assert_eq!(p2x, 0x0FF, "player 2 rode the platform's step");
}

#[test]
fn both_players_on_one_platform_are_carried_together() {
    let mut session = coop_session();
    platform_world(
        &mut session,
        (0x100, 0x100),
        (0x100, 0x100),
        &[(0, 1, (0x100, 0x128))],
    );
    for _ in 0..2 {
        platform_tick(&mut session);
    }
    let ((p1x, _), (p2x, _)) = position(&session);
    assert_eq!((p1x, p2x), (0x0FF, 0x0FF));
}

#[test]
fn different_platforms_carry_their_own_riders() {
    let mut session = coop_session();
    platform_world(
        &mut session,
        (0x100, 0x100),
        (0x120, 0x100),
        &[(0, 1, (0x100, 0x128)), (1, 1, (0x120, 0x128))],
    );
    for _ in 0..2 {
        platform_tick(&mut session);
    }
    let ((p1x, _), (p2x, _)) = position(&session);
    assert_eq!((p1x, p2x), (0x0FF, 0x11F));
}

#[test]
fn jumping_off_a_platform_ends_its_carry() {
    let mut session = coop_session();
    platform_world(
        &mut session,
        (0x120, 0x100),
        (0x100, 0x100),
        &[(0, 1, (0x100, 0x128))],
    );
    for _ in 0..2 {
        platform_tick(&mut session);
    }
    let ((_, _), (ridden, _)) = position(&session);
    assert_eq!(ridden, 0x0FF, "one carry step before leaving");
    park(
        &mut live(&mut session).second.as_mut().unwrap().sim,
        0x120,
        0x100,
    );
    for _ in 0..2 {
        platform_tick(&mut session);
    }
    let ((_, _), (p2x, _)) = position(&session);
    assert_eq!(p2x, 0x120, "no carry after the rider leaves the box");
}

#[test]
fn a_defeated_player_bubbles_drifts_to_the_other_and_pops_on_touch() {
    let mut session = coop_session();
    let (col, row) = {
        let ctx = live(&mut session);
        find_cell(ctx, |tile| tile & 0xFE == 0x50).expect("level 1 has a hazard")
    };
    {
        let ctx = live(&mut session);
        let second = ctx.second.as_mut().unwrap();
        place_player(&mut second.sim, col, row);
    }
    let effects = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
    assert_eq!(effects.outcome, StepOutcome::Continue);
    {
        let ctx = live(&mut session);
        let second = ctx.second.as_ref().unwrap();
        assert!(
            second.bubble.is_some(),
            "the hazard put player 2 in a bubble"
        );
        let spr = ctx.spr.as_ref().unwrap();
        assert!(spr.player_two.is_none(), "a bubbled player is not drawn");
        assert!(
            spr.player_two_bubble.is_none(),
            "the bubble waits before it appears"
        );
    }
    let gap = |session: &mut Session| {
        let ctx = live(session);
        let b = ctx.second.as_ref().unwrap().bubble.unwrap();
        i32::from(b.x.wrapping_sub(ctx.sim.x) as i16).abs()
            + i32::from(b.y.wrapping_sub(ctx.sim.y) as i16).abs()
    };
    for _ in 0..Bubble::APPEAR_DELAY {
        let _ = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
    }
    assert!(
        live(&mut session)
            .spr
            .as_ref()
            .unwrap()
            .player_two_bubble
            .is_some()
    );
    let before = gap(&mut session);
    for _ in 0..8 {
        let _ = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
    }
    assert!(
        gap(&mut session) < before,
        "the bubble drifts toward player 1"
    );
    // Player 1 touches it: player 2 is back where it burst, invulnerable,
    // and the level (no restart) keeps going.
    let at = {
        let ctx = live(&mut session);
        let b = ctx.second.as_ref().unwrap().bubble.unwrap();
        ctx.sim.x = b.x;
        ctx.sim.y = b.y;
        (b.x, b.y)
    };
    let effects = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
    assert_eq!(effects.outcome, StepOutcome::Continue);
    let ctx = live(&mut session);
    let second = ctx.second.as_ref().unwrap();
    assert!(second.bubble.is_none(), "popped");
    assert_eq!(second.sim.dead, 0);
    assert!(second.sim.invincibility > 100, "post-pop invulnerability");
    assert!(second.sim.x.abs_diff(at.0) < 4 && second.sim.y.abs_diff(at.1) < 8);
}

#[test]
fn select_bubbles_a_player_on_purpose() {
    let mut session = coop_session();
    let select = PlayerKeys {
        select: true,
        ..Default::default()
    };
    let _ = step(&mut session, PlayerKeys::default(), select);
    let ctx = live(&mut session);
    let bubble = ctx.second.as_ref().unwrap().bubble.expect("bubbled");
    assert_eq!(bubble.delay, 0, "a chosen bubble shows at once");
}

#[test]
fn a_party_wipe_restarts_the_entry_checkpoint_with_one_shared_life_spent() {
    let mut session = coop_session();
    let lives = session.lives;
    let (col, row) = {
        let ctx = live(&mut session);
        find_cell(ctx, |tile| tile & 0xFE == 0x50).expect("level 1 has a hazard")
    };
    // First defeat: player 2 bubbles, level continues.
    {
        let ctx = live(&mut session);
        let second = ctx.second.as_mut().unwrap();
        place_player(&mut second.sim, col, row);
    }
    let _ = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
    assert!(live(&mut session).second.as_ref().unwrap().bubble.is_some());
    assert_eq!(session.lives, lives, "a bubble is not a party wipe");
    // Second defeat: both are out; after NSMBW's 210-frame fade the party
    // restarts at the entry.
    {
        let ctx = live(&mut session);
        place_player(&mut ctx.sim, col, row);
    }
    let mut restarted_after = None;
    for frame in 1..=400u32 {
        let effects = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
        if effects.outcome == StepOutcome::LevelEntered(0) {
            restarted_after = Some(frame);
            break;
        }
    }
    let frames = restarted_after.expect("the wipe restarted the level");
    assert!(
        frames >= u32::from(Bubble::WIPE_FRAMES),
        "after the fade, not at once"
    );
    assert_eq!(session.lives, 4, "one shared life for the wipe");
    let second_on_ground = {
        let ctx = live(&mut session);
        assert!(ctx.first_bubble.is_none());
        let second = ctx.second.as_ref().unwrap();
        assert!(second.bubble.is_none());
        assert_eq!(second.sim.dead, 0);
        assert_eq!(ctx.sim.dead, 0);
        second.sim.on_ground
    };
    assert_eq!(session.second_player_airborne(), !second_on_ground);
}

#[test]
fn player_two_can_trigger_the_exit_bell_for_both() {
    let mut session = coop_session();
    let (col, row) = {
        let ctx = live(&mut session);
        find_cell(ctx, |tile| tile & 0xFE == 0x4A).expect("level 1 has an exit cell")
    };
    {
        let ctx = live(&mut session);
        let second = ctx.second.as_mut().unwrap();
        place_player(&mut second.sim, col, row);
    }
    let _ = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
    let ctx = live(&mut session);
    assert!(
        ctx.sim.exit_phase,
        "player 2's bell was adopted by the shared hand-off"
    );
    assert_eq!(
        ctx.sim.exit_tile_flag,
        ctx.second.as_ref().unwrap().sim.exit_tile_flag
    );
    let mut cleared = false;
    for _ in 0..260 {
        let effects = step(&mut session, PlayerKeys::default(), PlayerKeys::default());
        if matches!(
            effects.outcome,
            StepOutcome::LevelExited { cleared: true, .. }
        ) {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "the shared exit fade reached the map hand-off");
}
