//! Played zone-boss levels: header start, arena entry, defeat by live input,
//! the reward and the map return, through live `run_play` from the source
//! map node with no state rewrites after input begins. The tapes were found
//! by the ignored `search::search_route` tool (`ROUTE_BOSS=1`) and frozen
//! here; they are native fixtures, not recorded ROM input.
use super::*;

/// Level 6 (TreeZone5): the scene `$0B` block at (2816-3071, 512-767) loads
/// the Tree Zone arena; six hits, the rising reward, then node `$20`.
const LEVEL6: &str = "\
         RB*12 RAB*32 D*12 RB*48 RA*8 UA*48 D*32 UA*24 D*16 LA*2 UA*48 RAB*32 LB*8 RUA*12 \
         LUA*16 UA*24 D*12 RUAB*48 RA*8 RB*8 RUA*2 A*2 R*24 D*32 LUA*4 RAB*12 RA*32 L*16 A*16 \
         RB*8 RUAB*32 RB*8 RA*4 -*12 RUA*12 LB*2 RUAB*4 -*24 LAB*8 RUA*36 R*32 RA*48 D*2 A*48 \
         R*12 D*24 UA*8 RB*16 UA*2 RA*4 -*12 RB*2 RUA*2 LAB*4 UA*16 RAB*4 R*4 LB*12 RUAB*48 \
         -*12 RB*32 RA*8 LA*4 RUA*12 RB*8 RAB*12 RB*32 RUAB*32 LUA*48 -*32 RB*32 LUA*2 A*16 \
         LB*8 RUA*2 LB*8 RAB*16 -*4 L*4 -*48 A*48 RB*8 D*8 RUA*48 D*2 RAB*6 A*2 LB*4 RAB*8 \
         LAB*24 LA*4 RA*48 RAB*4 RB*16 UA*12 LAB*32 RB*12 RAB*12 D*24 RUA*12 RUAB*8 RAB*2 \
         -*24 UA*16 RB*24 -*12 R*12 LAB*16 RUA*12 RAB*24 -*12 RUAB*96 L*8 LAB*2 RAB*32 UA*12 \
         RUAB*32 RUA*16 LUA*16 RB*24 A*8 LA*12 RA*16 LAB*2 RB*12 RUAB*16 UA*2 RUAB*32 -*2 R*8 \
         LA*2 R*32 -*4 RUA*8 UA*4 D*12 UA*8 LAB*12 LB*16 RA*24 RUA*32 RUAB*48 RA*24 D*20 \
         RUAB*48 A*8 UA*12 LUA*8 D*8 R*16 LB*2 RB*32 UA*16 RA*2 -*52 LB*48 RUA*96 LB*4 RAB*24 \
         RB*16 RAB*4 LAB*8 RB*16 RA*8 LAB*4 LB*12 RA*24 -*4 RA*44 D*4 LAB*24 RUA*32 RAB*24 \
         -*8 L*12 D*2 RUA*12 UA*4 LA*8 LB*16 RUAB*12 -*2 A*4 RAB*32 D*24 A*16 RUA*16 LUA*2 \
         RA*4 RUAB*24 RUA*12 R*48 D*72 -*303 LA*4 RB*32 RUAB*24 D*32 LUA*32 L*16 RB*2 RUA*32 \
         LA*14 R*8 RB*8 LUA*12 L*24 RUAB*48 LA*24 A*2 RUAB*24 A*8 -*500";

#[test]
fn level6_tree_boss_header_spawn_to_map_through_live_play() {
    let outcome = run_header_route(6, spans(LEVEL6), |manifest, _, sim, cam, nodes| {
        assert_eq!(manifest.level_id, 5);
        assert_eq!(manifest.spawns.records.len(), 29);
        assert_eq!((sim.x, sim.y, cam.x, cam.y), (46, 708, 80, 704));
        assert_eq!(
            (sim.map_node, nodes.progress_slot(sim.map_node)),
            (32, Some(5))
        );
    });
    // No direct boss_clear or phase writes: the hits, the clear and the
    // rising reward all come from the tape.
    assert!(outcome.sim.boss_clear);
    assert_eq!(outcome.obj.boss_hits_remaining, 0);
    assert_eq!(
        (outcome.exit_completed_frame, outcome.map_handoff_frame),
        (Some(4551), Some(4552))
    );
    assert_eq!(outcome.sim.dead, 0);
    assert!(outcome.obj.unported.is_empty());
    assert_eq!(outcome.sim.level_tally, 1);
    assert_eq!(outcome.sim.music_request, 15, "the clear's song");
    // The golden coin byte, in memory and in the save record.
    assert_eq!(outcome.sim.level_progress[5], 0x80);
    assert_eq!(outcome.sim.save_record(0)[8 + 5], 0x80);
    assert_eq!(outcome.sim.golden_coin_count(), 1);
    let handoff = outcome.map_handoff.expect("the live handoff must run");
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        (0x19, Some(0), Some(6), 1)
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
            if ctx.sim.dead != 0 {
                break;
            }
        }
    }
    let _ = std::fs::remove_file(path);
}
