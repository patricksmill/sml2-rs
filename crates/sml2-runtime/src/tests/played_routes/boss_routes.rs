//! Played zone-boss levels: header start, arena entry, defeat by live input,
//! the reward and the map return, through live `run_play` from the source
//! map node with no state rewrites after input begins. The tapes were found
//! by the ignored `search::search_route` tool (`ROUTE_BOSS=1`) and frozen
//! here; they are native fixtures, not recorded ROM input.
use super::*;

/// Level 10 (PumpkinZone4): the scene `$0C` block at (3584-3839, 256-511)
/// loads the Pumpkin Zone arena.
const LEVEL10: &str = "\
         RAB*160 D*44 RA*90 RB*76 RUAB*168 RB*32 RA*66 LAB*2 RB*372 RUA*124 RB*116 RUAB*122 \
         R*48 LAB*12 RB*36 UA*16 LUA*16 R*36 RAB*32 D*32 R*30 LAB*36 RUAB*112 LB*12 RAB*72 \
         R*64 LAB*2 -*88 A*8 RB*172 A*6 -*8 RAB*48 LAB*12 RB*16 LA*8 RUAB*86 RUA*78 L*4 \
         RUAB*132 L*2 RUAB*104 RB*12 UA*28 RB*228 RA*56 RB*16 RUAB*64 -*16 RA*12 D*34 RUAB*48 \
         D*48 RUAB*66 RB*8 RUAB*40 R*24 RA*26 LB*16 RB*12 RA*52 LB*16 RA*60 R*1 UA*8 LUA*8 \
         RA*44 RB*542 RUAB*78 L*48 RA*236 RB*48 RUA*284 L*12 RUAB*70 -*517 -*200";

#[test]
fn level10_pumpkin_boss_header_spawn_to_map_through_live_play() {
    let outcome = run_header_route(10, spans(LEVEL10), |manifest, _, sim, cam, nodes| {
        assert_eq!(manifest.level_id, 9);
        assert_eq!(manifest.spawns.records.len(), 31);
        assert_eq!((sim.x, sim.y, cam.x, cam.y), (40, 468, 80, 448));
        assert_eq!(
            (sim.map_node, nodes.progress_slot(sim.map_node)),
            (43, Some(9))
        );
    });
    assert!(outcome.sim.boss_clear);
    assert_eq!(outcome.obj.boss_hits_remaining, 0);
    assert_eq!(
        (outcome.exit_completed_frame, outcome.map_handoff_frame),
        (Some(5477), Some(5478))
    );
    assert_eq!(outcome.sim.dead, 0);
    assert!(outcome.obj.unported.is_empty());
    assert_eq!(outcome.sim.level_tally, 1);
    assert_eq!(outcome.sim.music_request, 15, "the clear's song");
    assert_eq!(outcome.sim.level_progress[9], 0x80);
    assert_eq!(outcome.sim.save_record(0)[8 + 9], 0x80);
    assert_eq!(outcome.sim.golden_coin_count(), 1);
    let handoff = outcome.map_handoff.expect("the live handoff must run");
    // `$1C` is the hub's Pumpkin Zone door.
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        (0x1C, Some(0), Some(6), 1)
    );
    assert!(!handoff.cutscene);
}

/// Level 14 (MarioZone4): the scene `$10` block at (3584-3839, 0-255) loads
/// the Three Piggys arena; the parts take stomps against their own HP.
const LEVEL14: &str = "\
         LA*24 RUA*76 RUAB*50 LB*4 UA*52 -*8 UA*16 RB*100 UA*2 RB*130 RAB*74 -*24 RUA*32 \
         RB*126 RAB*24 LA*16 RB*48 A*144 RB*60 RUA*232 RB*36 RUAB*48 R*34 RUAB*48 R*8 RUA*32 \
         -*26 RA*4 R*36 RA*100 D*8 RAB*114 -*30 RA*40 -*4 RA*56 RB*8 RUAB*38 RB*18 RUAB*110 \
         RB*48 RUA*100 RB*1 RA*8 R*132 RUAB*118 RB*112 RUA*190 -*8 RUAB*20 R*52 RUAB*48 R*32 \
         UA*72 RUA*24 D*12 RAB*84 RB*124 RAB*60 R*2 UA*24 D*48 RAB*72 A*32 RB*120 R*12 \
         RUAB*468 A*96 R*50 RUAB*112 LB*32 UA*12 RA*8 D*80 RUA*16 D*16 LA*4 R*80 RA*2 L*50 \
         R*364 RUA*48 R*74 RAB*4 -*80 RUAB*12 RB*48 LUA*104 R*116 LAB*54 L*12 LUA*24 -*40 \
         RUA*42 LB*16 R*24 LUA*64 R*72 LUA*4 RA*12 RB*1 LAB*32 -*538 -*200";

#[test]
fn level14_mario_zone_boss_header_spawn_to_map_through_live_play() {
    let outcome = run_header_route(14, spans(LEVEL14), |manifest, _, sim, cam, nodes| {
        assert_eq!(manifest.level_id, 13);
        assert_eq!(manifest.spawns.records.len(), 32);
        assert_eq!((sim.x, sim.y, cam.x, cam.y), (56, 468, 80, 448));
        assert_eq!(
            (sim.map_node, nodes.progress_slot(sim.map_node)),
            (83, Some(29))
        );
    });
    assert!(outcome.sim.boss_clear);
    assert_eq!(outcome.obj.boss_hits_remaining, 0);
    assert_eq!(
        (outcome.exit_completed_frame, outcome.map_handoff_frame),
        (Some(6405), Some(6406))
    );
    assert_eq!(outcome.sim.dead, 0);
    assert!(outcome.obj.unported.is_empty());
    assert_eq!(outcome.sim.level_tally, 1);
    assert_eq!(outcome.sim.music_request, 15, "the clear's song");
    assert_eq!(outcome.sim.level_progress[29], 0x80);
    assert_eq!(outcome.sim.save_record(0)[8 + 29], 0x80);
    assert_eq!(outcome.sim.golden_coin_count(), 1);
    let handoff = outcome.map_handoff.expect("the live handoff must run");
    // `$1D` is the hub's Mario Zone door.
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        (0x1D, Some(0), Some(6), 1)
    );
    assert!(!handoff.cutscene);
}

/// Level 24 (MacroZone4): the scene `$0F` block at (2816-3071, 256-511) loads the Macro Zone arena.
const LEVEL24: &str = "\
         RB*36 RA*116 -*12 RUA*24 RB*76 RUAB*60 LUA*18 R*88 L*14 R*24 UA*54 RB*12 RUAB*178 \
         R*12 RUAB*62 R*136 RUAB*76 A*8 LB*12 RB*12 RA*132 RB*48 RUA*24 RB*48 RAB*16 R*32 \
         UA*32 R*16 RAB*76 -*44 RUAB*12 R*28 RUA*40 RUAB*64 D*4 A*48 D*2 RA*56 LB*60 RUA*24 \
         RB*48 RA*85 L*8 RAB*60 -*12 LA*2 RB*16 RUAB*48 LA*24 D*17 A*12 RB*48 RAB*4 RB*246 \
         RAB*22 RB*4 RAB*116 -*4 RAB*48 -*16 RUAB*424 L*32 RUAB*240 RB*1 LA*132 RAB*98 L*16 \
         RA*309 -*360 -*200";

#[test]
fn level24_macro_zone_boss_header_spawn_to_map_through_live_play() {
    let outcome = run_header_route(24, spans(LEVEL24), |manifest, _, sim, cam, nodes| {
        assert_eq!(manifest.level_id, 23);
        assert_eq!(manifest.spawns.records.len(), 24);
        assert_eq!((sim.x, sim.y, cam.x, cam.y), (44, 468, 80, 448));
        assert_eq!(
            (sim.map_node, nodes.progress_slot(sim.map_node)),
            (53, Some(14))
        );
    });
    assert!(outcome.sim.boss_clear);
    assert_eq!(outcome.obj.boss_hits_remaining, 0);
    assert_eq!(
        (outcome.exit_completed_frame, outcome.map_handoff_frame),
        (Some(4287), Some(4288))
    );
    assert_eq!(outcome.sim.dead, 0);
    assert!(outcome.obj.unported.is_empty());
    assert_eq!(outcome.sim.level_tally, 1);
    assert_eq!(outcome.sim.music_request, 15, "the clear's song");
    assert_eq!(outcome.sim.level_progress[14], 0x80);
    assert_eq!(outcome.sim.save_record(0)[8 + 14], 0x80);
    assert_eq!(outcome.sim.golden_coin_count(), 1);
    let handoff = outcome.map_handoff.expect("the live handoff must run");
    // `$1B` is the hub's Macro Zone door.
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        (0x1B, Some(0), Some(6), 1)
    );
    assert!(!handoff.cutscene);
}

/// Prints the map-entered session's state through `ROUTE_TAPE` at
/// `ROUTE_EVERY`-frame steps, to compare with `search::trace_route`.
#[test]
#[ignore = "authoring tool: run explicitly to trace a session replay"]
fn trace_session_route() {
    use crate::session::{SessionAssets, SessionState, StepOutcome, MAP_ENTRY_FRAMES};
    let level: u8 = std::env::var("ROUTE_LEVEL").unwrap().parse().unwrap();
    let every: usize = std::env::var("ROUTE_EVERY").map_or(50, |v| v.parse().unwrap());
    let tape = spans(&std::env::var("ROUTE_TAPE").unwrap());
    let assets = RouteAssets::load(level);
    let mut probe = PlayerSim::new();
    seed_unique_level_map_node(&mut probe, assets.manifest.level_id, &assets.nodes);
    let path = std::env::temp_dir().join(format!("sml2_trace_session_{}.sav", std::process::id()));
    let (mut runner, ..) =
        continuous::map_entry_runner(SessionAssets::load().unwrap(), probe.map_node, 0, &path);
    runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    assert!((0..MAP_ENTRY_FRAMES + 8).any(|_| matches!(
        runner.step_frame(PlayerKeys::default()),
        StepOutcome::LevelEntered(_)
    )));
    for (i, &keys) in tape.iter().enumerate() {
        runner.step_frame(keys);
        let SessionState::InLevel(ctx) = &runner.session.state else {
            println!("SESSION {} left the level", i + 1);
            break;
        };
        if (i + 1) % every == 0 || ctx.sim.dead != 0 {
            println!(
                "SESSION {} ({},{}) dead {} hits {} clear {} div {} vm {} pc {} scene {:#04X} phase {} paused {} fade {}",
                i + 1,
                ctx.sim.x,
                ctx.sim.y,
                ctx.sim.dead,
                ctx.obj.boss_hits_remaining,
                ctx.sim.boss_clear,
                ctx.obj.div,
                ctx.scene_vm.owns_gameplay(),
                ctx.scene_vm.pc,
                ctx.cam.scene_state,
                ctx.cam.boss_fight_phase,
                ctx.sim.paused,
                ctx.sim.fade_timer
            );
            if std::env::var("ROUTE_SLOTS").is_ok() {
                let slots: Vec<_> = ctx.obj.slots[..4]
                    .iter()
                    .map(|slot| (slot.b[5], slot.b[10], slot.b[9]))
                    .collect();
                println!(
                    "  slots {slots:?} div {} pow {} inv {} vv {} hv {} ground {}",
                    ctx.obj.div,
                    ctx.sim.powerup,
                    ctx.sim.invincibility,
                    ctx.sim.vvel,
                    ctx.sim.hvel,
                    ctx.sim.on_ground
                );
            }
            if ctx.sim.dead != 0 {
                break;
            }
        }
    }
    let _ = std::fs::remove_file(path);
}

/// Runs `ROUTE_TAPE` followed by each `|`-separated ending in `ROUTE_ENDINGS`
/// and prints the boss hits left, for probing a fight's timing.
#[test]
#[ignore = "authoring tool: run explicitly to probe boss endings"]
fn probe_boss_endings() {
    let level: u8 = std::env::var("ROUTE_LEVEL").unwrap().parse().unwrap();
    let base = std::env::var("ROUTE_TAPE").unwrap();
    let endings = std::env::var("ROUTE_ENDINGS").unwrap();
    let assets = RouteAssets::load(level);
    let endings: Vec<&str> = endings.split('|').collect();
    let results: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = endings
            .iter()
            .map(|ending| {
                let assets = &assets;
                let tape = spans(&format!("{base} {ending}"));
                scope.spawn(move || {
                    let (outcome, _) = assets.play(0, PlayTape::Buttons(tape), |_, _, _, _, _| {});
                    let parts: Vec<_> = outcome.obj.slots[1..4]
                        .iter()
                        .map(|slot| (slot.b[5], slot.b[8]))
                        .collect();
                    format!(
                        "hits {} defs {} parts {parts:?} dead {} clear {} at ({},{}) :: {ending}",
                        outcome.obj.boss_hits_remaining,
                        outcome.obj.boss_def_count,
                        outcome.sim.dead,
                        outcome.sim.boss_clear,
                        outcome.sim.x,
                        outcome.sim.y
                    )
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for line in results {
        println!("PROBE {line}");
    }
}
