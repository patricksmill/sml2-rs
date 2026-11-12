//! Deterministic two-players-on-one-keyboard smoke across all 32 levels.
//!
//! This exercises the real `Session::advance_two_players` path (never a test
//! fixture): independent motion, both sprites in the rendered frame, hazard
//! damage -> bubble -> revive, a party wipe -> checkpoint restart, and an exit
//! bell carried by player 2. After the 32-level pass it runs controlled pipe
//! and door-boss arena probes so those transitions are deterministic rather
//! than opportunistic. It does not claim route completion.
//!
//! Optional DX pass:
//!   SML2_DX_BUNDLE=/path/to/bundle.json cargo run -p sml2-runtime \
//!       --example local_multiplayer_smoke
//!
//! Output is one stable line per level/mode; fixed `div` and inputs make it
//! reproducible.

use sml2_core::objects::{
    B_DEF, B_FSTATE, B_HB_BOTTOM, B_HB_LEFT, B_HB_RIGHT, B_HB_TOP, B_PHASE, B_SCRX, B_SCRY,
    B_SPRITE, B_STATE, B_T24, B_TYPE, ObjectSim,
};
use sml2_core::player_sim::{PlayerKeys, PlayerSim};
use sml2_runtime::FrameInput;
use sml2_runtime::session::{Session, SessionAssets, SessionState, StepOutcome};

fn live(session: &mut Session) -> Option<&mut sml2_runtime::session::LevelContext> {
    match &mut session.state {
        SessionState::InLevel(ctx) => Some(ctx.as_mut()),
        _ => None,
    }
}

fn cell_player(col: usize, row: usize) -> (u16, u16) {
    (
        (col * 16).wrapping_sub(8) as u16,
        (row * 16).wrapping_sub(12) as u16,
    )
}

fn find_cell(
    ctx: &sml2_runtime::session::LevelContext,
    test: impl Fn(u8) -> bool,
) -> Option<(usize, usize)> {
    let wide = ctx.layout_live.world.cells_wide();
    let high = ctx.layout_live.world.cells_high();
    ctx.layout_live
        .cells
        .iter()
        .enumerate()
        .find_map(|(i, &tile)| test(tile).then_some((i % wide, i / wide)))
        .filter(|(_, row)| *row < high)
}

fn place_player(sim: &mut PlayerSim, col: usize, row: usize) {
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

fn step(session: &mut Session, first: PlayerKeys, second: PlayerKeys, div: u8) -> StepOutcome {
    session
        .advance_two_players(FrameInput { keys: first, div }, second)
        .outcome
}

fn reset(session: &mut Session, expanded: bool) {
    session.expanded_levels = expanded;
    session.lives = 5;
    session.coins_low = 0;
    session.coins_high = 0;
    session.level_tally = 0;
    session.powerup = 0;
    session.money_bag = 0;
    session.active_slot = None;
}

fn checksum(frame: &[u8]) -> u64 {
    frame.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

struct LevelReport {
    motion: bool,
    sprites: u8,
    transition: bool,
    pipe: bool,
    bubble: &'static str,
    revive: &'static str,
    wipe: &'static str,
    exit: &'static str,
    render: u64,
}

fn run_level(session: &mut Session, level: u8, expanded: bool) -> Result<LevelReport, String> {
    reset(session, expanded);
    session.set_local_multiplayer(true);
    session.begin_level(level, 0)?;
    {
        let ctx = live(session).ok_or("level did not go live")?;
        if ctx.second.is_none() {
            return Err("player 2 was not seeded".into());
        }
    }

    // Hazard -> bubble -> appears, drifts -> popped by player 1's touch.
    // A level with no hazard reports it.
    let mut bubble = "no-hazard";
    let mut revive = "skip";
    if let Some((col, row)) = live(session).and_then(|ctx| find_cell(ctx, |t| t & 0xFE == 0x50)) {
        if let Some(ctx) = live(session) {
            place_player(&mut ctx.second.as_mut().unwrap().sim, col, row);
        }
        let _ = step(session, PlayerKeys::default(), PlayerKeys::default(), 0x3D);
        let bubbled = live(session)
            .is_some_and(|ctx| ctx.second.as_ref().is_some_and(|s| s.bubble.is_some()));
        if bubbled {
            bubble = "ok";
            revive = "never-drawn";
            for _ in 0..400 {
                let _ = step(session, PlayerKeys::default(), PlayerKeys::default(), 0x3D);
                let Some(ctx) = live(session) else { break };
                let Some(second) = ctx.second.as_ref() else {
                    break;
                };
                let Some(b) = second.bubble else {
                    revive = if second.sim.dead == 0 {
                        "ok"
                    } else {
                        "popped-dead"
                    };
                    break;
                };
                let drawn = ctx
                    .sprite_ctx()
                    .is_some_and(|spr| spr.player_two.is_none() && spr.player_two_bubble.is_some());
                if drawn {
                    // Player 1 walks into the visible bubble.
                    revive = "not-popped";
                    ctx.sim.x = b.x;
                    ctx.sim.y = b.y;
                }
            }
        } else {
            bubble = "did-not-bubble";
        }
    }

    // Party wipe: both players into the hazard; after the all-bubbled fade,
    // one shared life and the same level.
    let mut wipe = "skip";
    if bubble == "ok" {
        let cell = live(session).and_then(|ctx| find_cell(ctx, |t| t & 0xFE == 0x50));
        if let Some((col, row)) = cell {
            let lives = session.lives;
            if let Some(ctx) = live(session) {
                place_player(&mut ctx.sim, col, row);
                place_player(&mut ctx.second.as_mut().unwrap().sim, col, row);
            }
            let mut restarted = false;
            for _ in 0..400 {
                let outcome = step(session, PlayerKeys::default(), PlayerKeys::default(), 0x3D);
                if matches!(outcome, StepOutcome::LevelEntered(id) if id == level) {
                    restarted = true;
                    break;
                }
            }
            wipe = if restarted
                && session.lives == lives.saturating_sub(1)
                && live(session).is_some_and(|ctx| {
                    ctx.first_bubble.is_none()
                        && ctx.second.as_ref().is_some_and(|s| s.bubble.is_none())
                }) {
                "ok"
            } else if restarted {
                "restarted-life-mismatch"
            } else {
                "no-restart"
            };
        }
    }

    // Fresh entry for motion/transition/render.
    reset(session, expanded);
    session.begin_level(level, 0)?;
    let mut transition = false;
    let mut pipe = false;
    let (start_x, start_hvel) = live(session).map_or((0, 0x80), |ctx| (ctx.sim.x, ctx.sim.hvel));
    let right = PlayerKeys {
        right: true,
        ..Default::default()
    };
    let jump = PlayerKeys {
        a: true,
        ..Default::default()
    };
    for frame in 0..120 {
        let second = if frame < 8 {
            jump
        } else {
            PlayerKeys::default()
        };
        let _ = step(session, right, second, 0x3D);
        if let Some(ctx) = live(session) {
            transition |= ctx.cam.transition != 0;
            pipe |= ctx.sim.pipe_dir >= 0x10
                || ctx.second.as_ref().is_some_and(|s| s.sim.pipe_dir >= 0x10);
        }
    }
    let motion =
        live(session).is_some_and(|ctx| ctx.sim.x != start_x || ctx.sim.hvel != start_hvel);
    // A fresh entry must render both players even if the motion probe ended
    // in a hazard or a bubble.
    reset(session, expanded);
    session.begin_level(level, 0)?;
    let _ = step(session, PlayerKeys::default(), PlayerKeys::default(), 0x3D);
    let sprites = live(session).map_or(0, |ctx| {
        u8::from(ctx.sprite_ctx().is_some_and(|s| s.player.is_some()))
            + u8::from(ctx.sprite_ctx().is_some_and(|s| s.player_two.is_some()))
    });
    let render = checksum(&session.render_frame((160, 144)));

    // Exit bell: either player may trigger it; both hand off together.
    let mut exit = "no-bell";
    if let Some((col, row)) = live(session).and_then(|ctx| find_cell(ctx, |t| t & 0xFE == 0x4A)) {
        if let Some(ctx) = live(session) {
            place_player(&mut ctx.second.as_mut().unwrap().sim, col, row);
        }
        let _ = step(session, PlayerKeys::default(), PlayerKeys::default(), 0x3D);
        let adopted = live(session).is_some_and(|ctx| ctx.sim.exit_phase);
        if !adopted {
            exit = "not-adopted";
        } else {
            exit = "timeout";
            for _ in 0..280 {
                let outcome = step(session, PlayerKeys::default(), PlayerKeys::default(), 0x3D);
                if matches!(outcome, StepOutcome::LevelExited { cleared: true, .. }) {
                    exit = "ok";
                    break;
                }
            }
        }
    }
    Ok(LevelReport {
        motion,
        sprites,
        transition,
        pipe,
        bubble,
        revive,
        wipe,
        exit,
        render,
    })
}

fn place_standing(sim: &mut PlayerSim, x: u16, y: u16) {
    sim.x = x;
    sim.y = y;
    sim.dead = 0;
    sim.dead_fade = false;
    sim.exit_phase = false;
    sim.exit_completed = false;
    sim.powerup = 0;
    sim.invincibility = 0;
    sim.on_ground = true;
    sim.hvel = 0x80;
    sim.vvel = 24;
    sim.pipe_dir = 0;
}

/// Level 11 (runtime id 10) has a known right-edge scene pipe. Player 1
/// enters it; player 2 stays out, so NSMBW's `setBalloonInNextgoto` puts it
/// in a bubble that follows player 1 into the transition.
fn pipe_probe(session: &mut Session) -> Result<(), String> {
    reset(session, false);
    session.set_local_multiplayer(true);
    session.begin_level(10, 0)?;
    let ctx = live(session).ok_or("pipe level did not go live")?;
    place_standing(&mut ctx.sim, 0x1F0, 0xA4);
    ctx.sim.phase = 0;
    ctx.sim.pipe_dir = 0x30;
    place_standing(&mut ctx.second.as_mut().unwrap().sim, 0x1E0, 0xA4);
    ctx.cam.x = 0x1B0;
    ctx.cam.y = 0xC3;
    ctx.cam.transition = 0;
    ctx.cam.scene_id = 0;
    ctx.cam.scene_state = 0;
    ctx.cam.prev_mario_y_lo = 0xA4;
    ctx.cam.update_enables(ctx.sim.x);
    let right = PlayerKeys {
        right: true,
        ..Default::default()
    };
    let mut followed = false;
    let mut transition = false;
    for _ in 0..16 {
        let _ = step(session, right, PlayerKeys::default(), 0);
        if let Some(ctx) = live(session) {
            followed |= ctx.second.as_ref().is_some_and(|s| s.bubble.is_some());
            transition |= ctx.cam.transition != 0 || ctx.cam.scene_id != 0;
        }
    }
    if !followed {
        return Err("player 2 did not follow player 1's pipe in a bubble".into());
    }
    if !transition {
        return Err("the pipe did not trigger the shared transition".into());
    }
    Ok(())
}

/// Runtime level 9 (extracted level 10) has source scene 12, the door boss
/// arena. Walking into its right edge must enter the arena for both players.
fn boss_arena_probe(session: &mut Session) -> Result<(), String> {
    reset(session, false);
    session.set_local_multiplayer(true);
    session.begin_level(9, 0)?;
    let ctx = live(session).ok_or("boss level did not go live")?;
    place_standing(&mut ctx.sim, 0x0EF1, 0x01D4);
    place_standing(&mut ctx.second.as_mut().unwrap().sim, 0x0EC0, 0x01D4);
    ctx.cam.x = 0x0EB0;
    ctx.cam.y = 0x01C0;
    ctx.cam.transition = 0;
    ctx.cam.scene_id = 0;
    ctx.cam.scene_state = 0;
    ctx.cam.prev_mario_y_lo = 0xD4;
    ctx.cam.update_enables(ctx.sim.x);
    let right = PlayerKeys {
        right: true,
        ..Default::default()
    };
    let mut entered = false;
    let mut carried = false;
    for _ in 0..128 {
        let before = live(session).map(|ctx| {
            (
                (ctx.sim.x, ctx.sim.y),
                (
                    ctx.second.as_ref().unwrap().sim.x,
                    ctx.second.as_ref().unwrap().sim.y,
                ),
                ctx.cam.transition != 0,
            )
        });
        let _ = step(session, right, PlayerKeys::default(), 0);
        if let (Some(before), Some(ctx)) = (before, live(session)) {
            let second = ctx.second.as_ref().unwrap();
            if before.2 {
                let first_delta = (
                    ctx.sim.x.wrapping_sub(before.0.0),
                    ctx.sim.y.wrapping_sub(before.0.1),
                );
                let second_delta = (
                    second.sim.x.wrapping_sub(before.1.0),
                    second.sim.y.wrapping_sub(before.1.1),
                );
                carried |= first_delta == second_delta && first_delta != (0, 0);
            }
            if ctx.boss_definition == Some(1) || ctx.cam.boss_mode() {
                entered = true;
                if second.bubble.is_some() {
                    return Err("player 2 was lost during the arena entry".into());
                }
                break;
            }
        }
    }
    if entered && !carried {
        return Err("the arena transition did not carry player 2".into());
    }
    if entered {
        Ok(())
    } else {
        Err("door boss arena entry was not reached".into())
    }
}

fn platform_anim(ctx: &sml2_runtime::session::LevelContext) -> u8 {
    let tables = &ctx.sprite_ctx().unwrap().tables;
    (0..=255u8)
        .find(|&anim| {
            tables
                .profile_for(ctx.manifest.level_id, anim)
                .is_some_and(|profile| profile.handler == 0x58C52)
        })
        .expect("horizontal-patrol anim")
}

fn platform_screen(world: (u16, u16), cam: (u16, u16)) -> (u8, u8) {
    (
        world.0.wrapping_sub(cam.0).wrapping_add(88) as u8,
        world.1.wrapping_sub(cam.1).wrapping_add(88) as u8,
    )
}

fn park_for_platform(sim: &mut PlayerSim, x: u16, y: u16) {
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

fn unstick_for_platform(sim: &mut PlayerSim) {
    sim.vvel = 0;
    sim.on_ground = false;
    sim.on_ground_obj = false;
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

fn platform_setup(
    session: &mut Session,
    first: (u16, u16),
    second: (u16, u16),
    slots: &[(usize, u8, (u16, u16))],
) -> Result<(), String> {
    reset(session, false);
    session.begin_level(0, 0)?;
    let anim = {
        let ctx = live(session).ok_or("platform level did not go live")?;
        platform_anim(ctx)
    };
    let ctx = live(session).ok_or("platform level did not go live")?;
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
    park_for_platform(&mut ctx.sim, first.0, first.1);
    let second_sim = &mut ctx.second.as_mut().unwrap().sim;
    second_sim.camx = ctx.cam.x;
    second_sim.camy = ctx.cam.y;
    park_for_platform(second_sim, second.0, second.1);
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
    Ok(())
}

fn platform_tick(session: &mut Session) -> Result<(), String> {
    if let Some(ctx) = live(session) {
        unstick_for_platform(&mut ctx.sim);
        unstick_for_platform(&mut ctx.second.as_mut().unwrap().sim);
    }
    let outcome = step(session, PlayerKeys::default(), PlayerKeys::default(), 0);
    if outcome != StepOutcome::Continue {
        return Err(format!("platform frame exited: {outcome:?}"));
    }
    Ok(())
}

fn platform_positions(session: &mut Session) -> Result<((u16, u16), (u16, u16)), String> {
    let ctx = live(session).ok_or("platform level ended")?;
    Ok((
        (ctx.sim.x, ctx.sim.y),
        (
            ctx.second.as_ref().unwrap().sim.x,
            ctx.second.as_ref().unwrap().sim.y,
        ),
    ))
}

/// P2-alone / same-platform / different-platform / jump-off platform carries.
fn platform_probe(session: &mut Session) -> Result<(), String> {
    platform_setup(
        session,
        (0x120, 0x100),
        (0x100, 0x100),
        &[(0, 1, (0x100, 0x128))],
    )?;
    for _ in 0..2 {
        platform_tick(session)?;
    }
    let ((alone_p1, _), (alone_p2, _)) = platform_positions(session)?;
    if alone_p1 != 0x120 || alone_p2 != 0x0FF {
        return Err(format!(
            "P2-alone carry wrong: {alone_p1:#X}, {alone_p2:#X}"
        ));
    }

    platform_setup(
        session,
        (0x100, 0x100),
        (0x100, 0x100),
        &[(0, 1, (0x100, 0x128))],
    )?;
    for _ in 0..2 {
        platform_tick(session)?;
    }
    let ((same_p1, _), (same_p2, _)) = platform_positions(session)?;
    if (same_p1, same_p2) != (0x0FF, 0x0FF) {
        return Err(format!(
            "same-platform carry wrong: {same_p1:#X}, {same_p2:#X}"
        ));
    }

    platform_setup(
        session,
        (0x100, 0x100),
        (0x120, 0x100),
        &[(0, 1, (0x100, 0x128)), (1, 1, (0x120, 0x128))],
    )?;
    for _ in 0..2 {
        platform_tick(session)?;
    }
    let ((diff_p1, _), (diff_p2, _)) = platform_positions(session)?;
    if (diff_p1, diff_p2) != (0x0FF, 0x11F) {
        return Err(format!(
            "split-platform carry wrong: {diff_p1:#X}, {diff_p2:#X}"
        ));
    }

    platform_setup(
        session,
        (0x120, 0x100),
        (0x100, 0x100),
        &[(0, 1, (0x100, 0x128))],
    )?;
    for _ in 0..2 {
        platform_tick(session)?;
    }
    if let Some(ctx) = live(session) {
        park_for_platform(&mut ctx.second.as_mut().unwrap().sim, 0x120, 0x100);
    }
    for _ in 0..2 {
        platform_tick(session)?;
    }
    let ((_, _), (off_p2, _)) = platform_positions(session)?;
    if off_p2 != 0x120 {
        return Err(format!("jump-off still carried: {off_p2:#X}"));
    }
    Ok(())
}

fn main() {
    let assets = SessionAssets::load().expect("extracted session assets");
    let mut session = Session::new(assets);
    let dx_bundle = std::env::var_os("SML2_DX_BUNDLE").map(std::path::PathBuf::from);
    let dx = dx_bundle.as_ref().map(|path| {
        let bytes = std::fs::read(path).expect("read requested SML2_DX_BUNDLE");
        let bundle: sml2_dx::Bundle =
            serde_json::from_slice(&bytes).expect("decode requested SML2_DX_BUNDLE");
        std::sync::Arc::new(
            sml2_dx::Presentation::new(&bundle).expect("validate requested SML2_DX_BUNDLE"),
        )
    });
    println!(
        "local_multiplayer_smoke dx={}",
        if dx.is_some() { "enabled" } else { "disabled" }
    );

    let mut failures = 0usize;
    for expanded in [false, true] {
        let mode = if expanded { "expanded" } else { "original" };
        for level in 0..32u8 {
            match run_level(&mut session, level, expanded) {
                Ok(report) => {
                    let required_failed = !report.motion
                        || report.sprites != 2
                        || (report.bubble != "no-hazard" && report.bubble != "ok")
                        || (report.bubble == "ok" && report.revive != "ok")
                        || (report.bubble == "ok" && report.wipe != "ok")
                        || (report.exit != "no-bell" && report.exit != "ok");
                    if required_failed {
                        failures += 1;
                    }
                    println!(
                        "level {level:02} {mode} motion={} sprites={} transition={} pipe={} bubble={} revive={} wipe={} exit={} boss=arena-probe-separate render={:016x}",
                        report.motion as u8,
                        report.sprites,
                        report.transition as u8,
                        report.pipe as u8,
                        report.bubble,
                        report.revive,
                        report.wipe,
                        report.exit,
                        report.render
                    );
                }
                Err(error) => {
                    failures += 1;
                    println!("level {level:02} {mode} error={error}");
                }
            }
        }
    }

    match pipe_probe(&mut session) {
        Ok(()) => println!("probe pipe ok"),
        Err(error) => {
            failures += 1;
            println!("probe pipe error={error}");
        }
    }
    match boss_arena_probe(&mut session) {
        Ok(()) => println!("probe boss_arena ok"),
        Err(error) => {
            failures += 1;
            println!("probe boss_arena error={error}");
        }
    }
    match platform_probe(&mut session) {
        Ok(()) => println!("probe platforms ok"),
        Err(error) => {
            failures += 1;
            println!("probe platforms error={error}");
        }
    }

    if let Some(dx) = dx {
        session.set_dx(Some(dx), sml2_core::character::Character::Mario);
        for level in 0..32u8 {
            reset(&mut session, true);
            session.set_local_multiplayer(true);
            match session.begin_level(level, 0) {
                Ok(()) => {
                    let _ = step(
                        &mut session,
                        PlayerKeys {
                            right: true,
                            ..Default::default()
                        },
                        PlayerKeys {
                            a: true,
                            ..Default::default()
                        },
                        0x3D,
                    );
                    let sprites = live(&mut session).map_or(0, |ctx| {
                        u8::from(ctx.sprite_ctx().is_some_and(|s| s.player.is_some()))
                            + u8::from(ctx.sprite_ctx().is_some_and(|s| s.player_two.is_some()))
                    });
                    let render = checksum(&session.render_frame((160, 144)));
                    if sprites != 2 {
                        failures += 1;
                    }
                    println!("level {level:02} dx sprites={sprites} render={render:016x}");
                }
                Err(error) => {
                    failures += 1;
                    println!("level {level:02} dx error={error}");
                }
            }
        }
    }

    println!("local_multiplayer_smoke failures={failures}");
    if failures != 0 {
        std::process::exit(1);
    }
}
