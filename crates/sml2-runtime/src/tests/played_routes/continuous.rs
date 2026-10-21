//! The existing fixed approaches also run through map selection, live session,
//! map return and real disk persistence. Fixtures start at source map nodes;
//! they do not claim to navigate from the title to every node.
use super::*;
use crate::session::{SessionAssets, SessionRunner, SessionState, StepOutcome, MAP_ENTRY_FRAMES};
use sml2_core::castle_sequence::GOLDEN_COIN_OFFSETS;
use sml2_core::map_sim::MapSim;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn every_ordinary_source_map_origin_loads_both_header_and_secret_entries() {
    use crate::session::{FrameInput, Session};
    let nodes = load_map_nodes();
    let origins: Vec<_> = (0..130u8)
        .filter_map(|node| {
            let level = nodes.level_byte(node)?;
            (level < 32).then_some((node, level))
        })
        .collect();
    let mut levels = std::collections::BTreeSet::new();
    let mut checked = 0;
    for (node, level) in origins {
        for progress in [0, 8] {
            let mut session = Session::new(SessionAssets::load().unwrap());
            let content = session.assets.level(level).unwrap();
            let header = content.manifest.header.clone();
            let secret = content.manifest.secret_entry;
            // The source node bands' screen owners ($3F04D..$3F13A).
            let screen = match node {
                0x1E..=0x27 => 2,
                0x28..=0x31 | 0x78 | 0x7A => 1,
                0x32..=0x3B | 0x6A..=0x6C => 3,
                0x3C..=0x45 => 4,
                0x46..=0x4F => 5,
                0x50..=0x59 => 6,
                0x5A..=0x63 => 7,
                0x6E..=0x77 => 8,
                _ => 0,
            };
            let slot = nodes.progress_slot(node).unwrap() as usize;
            let mut map = MapSim::new();
            map.node = node;
            map.mario_dead = 0xFF; // standing at the node after its arrival handler
            map.load_screen(screen, 0, &session.assets.node_records);
            map.node = node;
            map.load_node(node, &session.assets.node_records);
            let sentinel = if slot == 40 { 39 } else { 40 };
            map.level_progress[sentinel] = 0x80;
            map.level_progress[slot] = progress;
            let expected_progress = map.level_progress;
            session.state = SessionState::Overworld(map);
            session.level_tally = 0x12;
            let mut entered = false;
            for frame in 0..MAP_ENTRY_FRAMES + 8 {
                let keys = PlayerKeys {
                    a: frame == 0,
                    ..Default::default()
                };
                let effects = session.advance(FrameInput { keys, div: 0 });
                assert!(
                    effects.diagnostics.is_empty(),
                    "node {node}: {:?}",
                    effects.diagnostics
                );
                if matches!(effects.outcome, StepOutcome::LevelEntered(_)) {
                    entered = true;
                    break;
                }
            }
            assert!(
                entered,
                "node {node} must select level {level}, progress={progress}"
            );
            let SessionState::InLevel(ctx) = &session.state else {
                panic!("level")
            };
            assert_eq!((ctx.manifest.level_id, ctx.sim.map_node), (level, node));
            assert_eq!(ctx.sim.level_progress, expected_progress);
            assert_eq!(ctx.sim.level_tally, 0x12);
            let start = if progress == 8 { secret } else { None };
            let expected = start
                .map(|s| (s.mario_x, s.mario_y, s.camera_x, s.camera_y))
                .unwrap_or((
                    header.mario_spawn_x,
                    header.mario_spawn_y,
                    header.camera_start_x,
                    header.camera_start_y,
                ));
            assert_eq!((ctx.sim.x, ctx.sim.y, ctx.cam.x, ctx.cam.y), expected);
            levels.insert(level);
            checked += 1;
        }
    }
    assert_eq!(
        levels.len(),
        31,
        "the node-free Wario level is covered by castle_flow"
    );
    assert_eq!(
        checked, 64,
        "32 ordinary map origins, each with two progress states"
    );
}

/// A live session over `assets` standing on source map node `node` with
/// `progress` in its slot, saving to `path`, before the A press that enters
/// the level. Returns
/// the runner, the node's progress slot, a bystander slot holding another
/// level's secret-route bit, and the whole progress block at entry.
pub(super) fn map_entry_runner(
    assets: SessionAssets,
    node: u8,
    progress: u8,
    path: &Path,
) -> (SessionRunner, usize, usize, [u8; 42]) {
    let slot = assets.nodes.progress_slot(node).unwrap() as usize;
    let screen = match node {
        0x1E..=0x27 => 2,
        0x28..=0x31 | 0x78 | 0x7A => 1,
        0x32..=0x3B | 0x6A..=0x6C => 3,
        0x3C..=0x45 => 4,
        0x46..=0x4F => 5,
        0x50..=0x59 => 6,
        0x5A..=0x63 => 7,
        0x6E..=0x77 => 8,
        _ => 0,
    };
    let mut map = MapSim::new();
    map.node = node;
    map.level_progress[slot] = progress;
    for off in GOLDEN_COIN_OFFSETS {
        map.level_progress[off] = 0x80;
    }
    // Another level's secret-route bit, which the route must leave alone.
    let bystander = if slot == 2 { 1 } else { 2 };
    map.level_progress[bystander] = 0x08;
    map.coins_low = 0x42;
    map.lives = 7;
    map.mario_dead = 0xFF; // source arrival already handled at this standing node
    map.load_screen(screen, 0, &assets.node_records);
    // Bonus-course nodes are visited by walking on an already loaded screen;
    // loading a screen at node $44 itself would run the post-clear $45 rewrite.
    map.node = node;
    map.load_node(node, &assets.node_records);
    let initial_progress = map.level_progress;
    let mut runner = SessionRunner::new(assets, path.to_path_buf(), 1, (160, 144), true);
    runner.session.state = SessionState::Overworld(map);
    runner.session.lives = 7;
    runner.session.coins_low = 0x42;
    runner.session.level_tally = 0x12;
    runner.session.money_bag = 3;
    runner.session.active_slot = Some(1);
    let record = runner.save_record();
    runner.session.slots.write_slot(1, &record);
    (runner, slot, bystander, initial_progress)
}

pub(super) fn verify_map_entry_completion_and_reload(
    manifest: &LevelManifest,
    node: u8,
    progress: u8,
    tape: &[PlayerKeys],
    expected: &PlayOutcome,
) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "sml2_continuous_{}_{}.sav",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let (mut runner, slot, bystander, initial_progress) =
        map_entry_runner(SessionAssets::load().unwrap(), node, progress, &path);
    let untouched_a = *runner.session.slots.slot(0).unwrap();
    let untouched_c = *runner.session.slots.slot(2).unwrap();
    runner.step_frame(PlayerKeys {
        a: true,
        ..Default::default()
    });
    let mut entered = false;
    for _ in 0..MAP_ENTRY_FRAMES + 8 {
        if matches!(
            runner.step_frame(PlayerKeys::default()),
            StepOutcome::LevelEntered(_)
        ) {
            entered = true;
            break;
        }
    }
    if !entered {
        if let SessionState::Overworld(map) = &runner.session.state {
            eprintln!(
                "map node={} screen={} walking={} move={} latch={} dead={} pending={}",
                map.node,
                map.screen_id,
                map.cursor_walking,
                map.cursor_move,
                map.scroll_only_latch,
                map.mario_dead,
                runner.session.pending_level.is_some()
            );
        }
    }
    assert!(
        entered,
        "level {} must load from its map node",
        manifest.level_id
    );
    let SessionState::InLevel(ctx) = &runner.session.state else {
        panic!("level")
    };
    assert_eq!(ctx.manifest.level_id, manifest.level_id);
    assert_eq!(ctx.sim.level_progress_flags, progress);
    assert_eq!(
        ctx.sim.level_progress, initial_progress,
        "map entry must carry the entire save block"
    );
    assert_eq!(
        (ctx.sim.level_tally, ctx.sim.lives, ctx.sim.coins_low),
        (0x12, 7, 0x42)
    );
    let mut exited = false;
    for &keys in tape {
        if let SessionState::InLevel(ctx) = &runner.session.state {
            if !ctx.sim.exit_phase {
                assert_eq!(
                    ctx.sim.dead, 0,
                    "the played route must not die: frame={} xy=({}, {}) clock={} phase={} camera=({}, {}) active_objects={:?}",
                    ctx.frame_n, ctx.sim.x, ctx.sim.y, ctx.obj.fff0,
                    ctx.obj.phase, ctx.cam.x, ctx.cam.y,
                    ctx.obj.slots.iter().filter(|slot| slot.active())
                        .map(|slot| (slot.def(), slot.x(), slot.y())).collect::<Vec<_>>(),
                );
            }
            assert!(ctx.obj.unported.is_empty());
        }
        if matches!(
            runner.step_frame(keys),
            StepOutcome::LevelExited { cleared: true, .. }
        ) {
            exited = true;
            break;
        }
    }
    if !exited {
        if let SessionState::InLevel(ctx) = &runner.session.state {
            eprintln!("route end level={} xy=({}, {}) dead={} frame={} exit={} arm={:?} moon={} float={} flags={}", ctx.manifest.level_id, ctx.sim.x, ctx.sim.y, ctx.sim.dead, ctx.frame_n, ctx.sim.exit_phase, ctx.bell_arm.as_ref().map(|arm| (arm.frame, arm.position)), ctx.sim.moon, ctx.sim.moon_float_timer, ctx.sim.level_progress_flags);
        } else {
            eprintln!("route left the level unexpectedly");
        }
    }
    assert!(
        exited,
        "level {} must complete through ordinary input",
        manifest.level_id
    );
    // A midpoint bell can select a real bonus room before the deferred landing.
    for frame in 0..6000 {
        if matches!(&runner.session.state, SessionState::Overworld(map) if map.room.is_none()) {
            break;
        }
        runner.step_frame(PlayerKeys {
            a: frame % 8 == 0,
            ..Default::default()
        });
    }
    let SessionState::Overworld(map) = &runner.session.state else {
        panic!("returned map")
    };
    assert!(map.room.is_none());
    assert_eq!(map.node, expected.sim.map_node);
    assert_eq!(map.level_progress[slot], expected.sim.level_progress[slot]);
    assert_eq!(map.level_progress[bystander], 0x08);
    for off in GOLDEN_COIN_OFFSETS {
        assert_eq!(map.level_progress[off], 0x80);
    }
    assert_eq!(runner.session.level_tally, 0x12 + expected.sim.level_tally);
    let screen = map.screen_id;
    let saved = fs::read(&path).expect("the actual host must have written a save");
    assert_eq!(saved, runner.save_record());
    assert_eq!(runner.session.slots.slot(1).unwrap().as_slice(), saved);
    assert_eq!(runner.session.slots.slot(0).unwrap(), &untouched_a);
    assert_eq!(runner.session.slots.slot(2).unwrap(), &untouched_c);
    let mut reloaded = SessionRunner::new(
        SessionAssets::load().unwrap(),
        path.clone(),
        1,
        (160, 144),
        true,
    );
    assert!(reloaded.load_from_disk());
    assert_eq!(reloaded.save_record().as_slice(), saved);
    let SessionState::Overworld(map) = &reloaded.session.state else {
        panic!("reloaded map")
    };
    assert_eq!(
        map.screen_id, screen,
        "reload must select the source zone, not the hub"
    );
    fs::remove_file(path).unwrap();
}
