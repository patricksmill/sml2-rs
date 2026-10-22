//! Route search for authoring played-route tapes; ignored by default. It grows
//! held-button tapes through the same live `run_play` loop the route tests use,
//! keeping the shortest surviving tape per 16-pixel cell and extending the
//! cells nearest the next waypoint (an archive search in the Go-Explore style).
//! A found tape is printed in [`spans`] form for a route test to freeze.
//!
//! ```bash
//! ROUTE_LEVEL=1 ROUTE_PROGRESS=0 ROUTE_FLAG=0 ROUTE_GOAL=4064,480 \
//!   cargo test --release -p sml2-runtime search_route -- --ignored --nocapture
//! ```
//!
//! `ROUTE_GOAL` lists `x,y` pixel waypoints separated by `;`, the last being
//! the bell; `ROUTE_PREFIX` seeds the search with a spans tape;
//! `ROUTE_BOSS=1` searches a boss level instead: the last waypoint is the
//! arena's scene block, hits on the boss then score above position, and a
//! route arrives when the boss clear hands the level back to the map;
//! `ROUTE_EVALS` bounds the run; `ROUTE_LOG` names a progress log file.
use super::*;
use crate::session::SessionAssets;
use std::collections::HashMap;
use std::io::Write;

const MACROS: &[&str] = &[
    "R", "RB", "L", "LB", "-", "D", "RA", "RBA", "LA", "LBA", "A", "UA", "RUA", "RBUA", "LUA",
];
const LENGTHS: &[usize] = &[2, 4, 8, 12, 16, 24, 32, 48];

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

#[derive(Clone)]
struct Cell {
    tape: Vec<PlayerKeys>,
    score: i64,
    picks: u32,
}

enum Verdict {
    /// `boss` is the boss's remaining hits once one is defined.
    Alive {
        x: u16,
        y: u16,
        ground: bool,
        boss: Option<u8>,
        clear: bool,
        /// While `$05D5` locks input (an arena intro), how long is left in
        /// 32-frame steps: waiting longer is progress the position can't show.
        lock: u8,
    },
    Failed,
    Arrived {
        frame: u64,
    },
}

/// A switch such as `ROUTE_SESSION=1` (`bool` parsing alone rejects `1`).
fn switch(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| matches!(v.as_str(), "1" | "true"))
}

fn env<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// `run_play` continues the map entry's divider, so it sees the session's
/// enemy randomness; `ROUTE_SESSION=1` also replays every candidate through
/// the map-entered session itself.
fn judge(assets: &RouteAssets, progress: u8, flag: u8, tape: &[PlayerKeys]) -> Verdict {
    match play_judge(assets, progress, flag, tape) {
        Verdict::Failed => Verdict::Failed,
        verdict if !switch("ROUTE_SESSION") => verdict,
        verdict => match (session_judge(assets, progress, flag, tape), verdict) {
            (Verdict::Arrived { frame: a }, Verdict::Arrived { frame: b }) => {
                Verdict::Arrived { frame: a.max(b) }
            }
            (Verdict::Alive { .. }, verdict @ Verdict::Alive { .. }) => verdict,
            // A boss route's session leaves the level when the clear hands off.
            (_, verdict @ Verdict::Arrived { .. }) if switch("ROUTE_BOSS") => verdict,
            _ => Verdict::Failed,
        },
    }
}

fn session_judge(assets: &RouteAssets, progress: u8, flag: u8, tape: &[PlayerKeys]) -> Verdict {
    use crate::session::{FrameInput, SessionState, StepOutcome, MAP_ENTRY_FRAMES};
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "sml2_route_search_{}_{}.sav",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut probe = PlayerSim::new();
    seed_unique_level_map_node(&mut probe, assets.manifest.level_id, &assets.nodes);
    // Loading the assets dominates a run, and a run leaves them unchanged.
    static POOL: std::sync::Mutex<Vec<SessionAssets>> = std::sync::Mutex::new(Vec::new());
    let assets = POOL
        .lock()
        .unwrap()
        .pop()
        .unwrap_or_else(|| SessionAssets::load().unwrap());
    let (mut runner, ..) = continuous::map_entry_runner(assets, probe.map_node, progress, &path);
    // `step_frame` without its audio synthesis and save writes: the same
    // virtual divider ticks, so the simulation sees identical input.
    fn step(runner: &mut crate::host::SessionRunner, keys: PlayerKeys) -> StepOutcome {
        runner.virtual_clock.advance_tick();
        let div = runner.virtual_clock.sample();
        runner.session.advance(FrameInput { keys, div }).outcome
    }
    step(
        &mut runner,
        PlayerKeys {
            a: true,
            ..Default::default()
        },
    );
    let entered = (0..MAP_ENTRY_FRAMES + 8).any(|_| {
        matches!(
            step(&mut runner, PlayerKeys::default()),
            StepOutcome::LevelEntered(_)
        )
    });
    let mut verdict = Verdict::Failed;
    if entered {
        for &keys in tape {
            step(&mut runner, keys);
            let SessionState::InLevel(ctx) = &runner.session.state else {
                break;
            };
            if !ctx.obj.unported.is_empty() || (ctx.sim.dead != 0 && ctx.bell_arm.is_none()) {
                break;
            }
        }
        if let SessionState::InLevel(ctx) = &runner.session.state {
            verdict = match &ctx.bell_arm {
                Some(arm) if arm.exit_tile_flag == flag && arm.dead == 0 => {
                    Verdict::Arrived { frame: arm.frame }
                }
                Some(_) => Verdict::Failed,
                None if ctx.sim.dead != 0 || !ctx.obj.unported.is_empty() => Verdict::Failed,
                None => Verdict::Alive {
                    x: ctx.sim.x,
                    y: ctx.sim.y,
                    ground: ctx.sim.on_ground,
                    boss: (ctx.obj.boss_def_count != 0).then_some(ctx.obj.boss_hits_remaining),
                    clear: false,
                    lock: lock_steps(ctx.sim.fade_timer),
                },
            };
        }
    }
    let _ = std::fs::remove_file(path);
    POOL.lock().unwrap().push(runner.session.assets);
    verdict
}

fn lock_steps(fade_timer: u16) -> u8 {
    if fade_timer > 1 {
        (fade_timer >> 5).min(254) as u8 + 1
    } else {
        0
    }
}

/// Idle frames a boss route gets after its tape, so a collected reward coin
/// can run its fade to the map handoff.
const BOSS_SETTLE: usize = 1000;

/// `tape` extended with idle frames to `len`.
fn pad(tape: &[PlayerKeys], len: usize) -> Vec<PlayerKeys> {
    let mut tape = tape.to_vec();
    if tape.len() < len {
        tape.resize(len, PlayerKeys::default());
    }
    tape
}

fn play_judge(assets: &RouteAssets, progress: u8, flag: u8, tape: &[PlayerKeys]) -> Verdict {
    let verdict = play_judge_once(assets, progress, flag, tape);
    match verdict {
        // After the clear, the reward coin and fade finish without input.
        Verdict::Alive { clear: true, .. } => {
            match play_judge_once(assets, progress, flag, &pad(tape, tape.len() + BOSS_SETTLE)) {
                arrived @ Verdict::Arrived { .. } => arrived,
                _ => verdict,
            }
        }
        verdict => verdict,
    }
}

fn play_judge_once(assets: &RouteAssets, progress: u8, flag: u8, tape: &[PlayerKeys]) -> Verdict {
    let (outcome, _) = assets.play(
        progress,
        PlayTape::Buttons(tape.to_vec()),
        |_, _, _, _, _| {},
    );
    if switch("ROUTE_DEBUG") {
        eprintln!(
            "JUDGE len {} unported {:?} death_frames {} dead {} bell {} handoff {:?} clear {}",
            tape.len(),
            outcome.obj.unported,
            outcome.gameplay_death_frames,
            outcome.sim.dead,
            outcome.bell_arm.is_some(),
            outcome.map_handoff_frame,
            outcome.sim.boss_clear
        );
    }
    if !outcome.obj.unported.is_empty() || outcome.gameplay_death_frames != 0 {
        return Verdict::Failed;
    }
    if switch("ROUTE_BOSS") {
        // The clear's exit fade records an arm too; only the handoff counts.
        if outcome.sim.dead != 0 {
            return Verdict::Failed;
        }
        if let Some(frame) = outcome.map_handoff_frame {
            return if outcome.sim.boss_clear {
                Verdict::Arrived { frame }
            } else {
                Verdict::Failed
            };
        }
    } else if let Some(arm) = outcome.bell_arm {
        return if arm.exit_tile_flag == flag && arm.dead == 0 {
            Verdict::Arrived { frame: arm.frame }
        } else {
            Verdict::Failed
        };
    }
    if outcome.sim.dead != 0 {
        return Verdict::Failed;
    }
    Verdict::Alive {
        x: outcome.sim.x,
        y: outcome.sim.y,
        ground: outcome.sim.on_ground,
        boss: (outcome.obj.boss_def_count != 0).then_some(outcome.obj.boss_hits_remaining),
        clear: switch("ROUTE_BOSS") && outcome.sim.boss_clear,
        lock: lock_steps(outcome.sim.fade_timer),
    }
}

/// Waypoints passed count most; then closeness to the next one. A waypoint
/// counts only once Mario stands within 16 pixels of it.
/// A boss on the field outranks every waypoint, and each hit outranks the
/// last.
fn boss_score(boss: Option<u8>, clear: bool) -> i64 {
    boss.map_or(0, |hits| 100_000_000 - i64::from(hits) * 1_000_000)
        + if clear { 100_000_000 } else { 0 }
}

fn score(goals: &[(i64, i64)], reached: &mut usize, x: u16, y: u16, ground: bool) -> i64 {
    let (x, y) = (i64::from(x), i64::from(y));
    while *reached + 1 < goals.len() {
        let (gx, gy) = goals[*reached];
        if ground && (gx - x).abs() <= 16 && (gy - y).abs() <= 8 {
            *reached += 1;
        } else {
            break;
        }
    }
    let (gx, gy) = goals[*reached];
    *reached as i64 * 100_000 - (gx - x).abs() - (gy - y).abs() / 4
}

fn span_list(tape: &[PlayerKeys]) -> Vec<(String, usize)> {
    to_spans(tape)
        .split_whitespace()
        .map(|span| {
            let (held, frames) = span.split_once('*').unwrap();
            (held.to_string(), frames.parse().unwrap())
        })
        .collect()
}

fn join(list: &[(String, usize)]) -> Vec<PlayerKeys> {
    let text: Vec<String> = list.iter().map(|(h, n)| format!("{h}*{n}")).collect();
    spans(&text.join(" "))
}

/// Greedy cleanup of an arriving tape: drop spans, or give a span its
/// neighbour's buttons, whenever the route still arrives at the same bell.
fn simplify(
    assets: &RouteAssets,
    progress: u8,
    flag: u8,
    tape: Vec<PlayerKeys>,
) -> (Vec<PlayerKeys>, u64) {
    let arrives = |list: &[(String, usize)]| match judge(assets, progress, flag, &join(list)) {
        Verdict::Arrived { frame } => Some(frame),
        _ => None,
    };
    let mut list = span_list(&tape);
    let mut frame = arrives(&list).expect("found tape arrives");
    loop {
        let mut changed = false;
        let mut i = 0;
        while i < list.len() {
            let mut tries = vec![];
            let mut without = list.clone();
            without.remove(i);
            tries.push(without);
            for n in [i.wrapping_sub(1), i + 1] {
                if let Some((held, _)) = list.get(n) {
                    let mut merged = list.clone();
                    merged[i].0 = held.clone();
                    tries.push(merged);
                }
            }
            let mut halved = list.clone();
            halved[i].1 = halved[i].1.div_ceil(2);
            if halved[i].1 != list[i].1 {
                tries.push(halved);
            }
            let accepted = tries.into_iter().find_map(|t| {
                let t = span_list(&join(&t));
                arrives(&t).map(|f| (t, f))
            });
            if let Some((t, f)) = accepted {
                list = t;
                frame = f;
                changed = true;
            } else {
                i += 1;
            }
        }
        if !changed {
            let tape = pad(&join(&list), frame as usize);
            return (tape[..frame as usize].to_vec(), frame);
        }
    }
}

#[test]
#[ignore = "authoring tool: run explicitly to search for a route tape"]
fn search_route() {
    let level: u8 = env("ROUTE_LEVEL", 1);
    let progress: u8 = env("ROUTE_PROGRESS", 0);
    let flag: u8 = env("ROUTE_FLAG", 0);
    let budget: usize = env("ROUTE_EVALS", 40_000);
    let goals: Vec<(i64, i64)> = std::env::var("ROUTE_GOAL")
        .expect("ROUTE_GOAL=x,y[;x,y...]")
        .split(';')
        .map(|p| {
            let (x, y) = p.split_once(',').unwrap();
            (x.trim().parse().unwrap(), y.trim().parse().unwrap())
        })
        .collect();
    let mut log = std::env::var("ROUTE_LOG")
        .ok()
        .map(|path| std::fs::File::create(path).unwrap());
    let assets = RouteAssets::load(level);
    let prefix = std::env::var("ROUTE_PREFIX")
        .map(|t| spans(&t))
        .unwrap_or_default();
    let mut rng = Rng(env("ROUTE_SEED", 0x9E37_79B9_7F4A_7C15));
    // Each cell remembers its waypoint progress inside its tape's score.
    let mut archive: HashMap<(u16, u16, bool, usize, u8, u8), Cell> = HashMap::new();
    match judge(&assets, progress, flag, &prefix) {
        Verdict::Alive {
            x,
            y,
            ground,
            boss,
            clear,
            lock,
        } => {
            let mut reached = 0;
            let s = score(&goals, &mut reached, x, y, ground) + boss_score(boss, clear);
            archive.insert(
                (
                    x >> 4,
                    y >> 4,
                    ground,
                    reached,
                    if clear { 0xFE } else { boss.unwrap_or(0xFF) },
                    lock,
                ),
                Cell {
                    tape: prefix,
                    score: s,
                    picks: 0,
                },
            );
        }
        _ => panic!("the prefix must leave Mario alive"),
    }
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut evals = 0;
    let mut best = i64::MIN;
    while evals < budget {
        let mut ranked: Vec<_> = archive.iter().map(|(k, c)| (*k, c.score)).collect();
        ranked.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
        ranked.truncate(env("ROUTE_WIDTH", 200));
        let batch: Vec<(Vec<PlayerKeys>, usize)> = (0..threads * 2)
            .map(|_| {
                // Favour high-ranked cells that have been extended least.
                let weights: Vec<f64> = ranked
                    .iter()
                    .enumerate()
                    .map(|(rank, (k, _))| {
                        (-(rank as f64) / 40.0).exp() / (1.0 + f64::from(archive[k].picks)).sqrt()
                    })
                    .collect();
                let total: f64 = weights.iter().sum();
                let mut pick = (rng.below(1_000_000) as f64 / 1_000_000.0) * total;
                let mut chosen = ranked[0].0;
                for (w, (k, _)) in weights.iter().zip(&ranked) {
                    if pick < *w {
                        chosen = *k;
                        break;
                    }
                    pick -= w;
                }
                archive.get_mut(&chosen).unwrap().picks += 1;
                let mut tape = archive[&chosen].tape.clone();
                for _ in 0..1 + rng.below(3) {
                    let held = spans(&format!("{}*1", MACROS[rng.below(MACROS.len())]))[0];
                    tape.extend(std::iter::repeat_n(held, LENGTHS[rng.below(LENGTHS.len())]));
                }
                (tape, chosen.3)
            })
            .collect();
        let verdicts: Vec<Verdict> = std::thread::scope(|scope| {
            let handles: Vec<_> = batch
                .iter()
                .map(|(tape, _)| scope.spawn(|| judge(&assets, progress, flag, tape)))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        evals += batch.len();
        for ((tape, mut reached), verdict) in batch.into_iter().zip(verdicts) {
            match verdict {
                Verdict::Failed => {}
                Verdict::Arrived { frame } => {
                    let (tape, frame) = simplify(
                        &assets,
                        progress,
                        flag,
                        pad(&tape, frame as usize)[..frame as usize].to_vec(),
                    );
                    let found = format!("{} -*200", to_spans(&tape));
                    let mut entry = String::new();
                    let (outcome, node) = assets.play(
                        progress,
                        PlayTape::Buttons(spans(&found)),
                        |m, _, sim, cam, nodes| {
                            entry = format!(
                                "entry mario ({},{}) cam ({},{}) node {} slot {:?} records {}",
                                sim.x,
                                sim.y,
                                cam.x,
                                cam.y,
                                sim.map_node,
                                nodes.progress_slot(sim.map_node),
                                m.spawns.records.len()
                            )
                        },
                    );
                    let slot = assets.nodes.progress_slot(node).unwrap() as usize;
                    let h = outcome.map_handoff.as_ref().unwrap();
                    println!("FOUND level {level} progress {progress} frame {frame}");
                    println!("{entry}");
                    match outcome.bell_arm.as_ref() {
                        Some(arm) => println!(
                            "arm {:?} fade {} music {} flag {:#04X}; done {:?} handoff {:?}; frames {}",
                            arm.position,
                            arm.fade_timer,
                            arm.music_request,
                            arm.exit_tile_flag,
                            outcome.exit_completed_frame,
                            outcome.map_handoff_frame,
                            outcome.frames
                        ),
                        None => println!(
                            "boss clear {} at ({},{}); done {:?} handoff {:?}; frames {}",
                            outcome.sim.boss_clear,
                            outcome.sim.x,
                            outcome.sim.y,
                            outcome.exit_completed_frame,
                            outcome.map_handoff_frame,
                            outcome.frames
                        ),
                    }
                    println!(
                        "after: tally {} node {} progress[{slot}] {:#04X} save {:#04X} flags {:#04X} route {} dead {}; handoff node {} screen {:?} music {:?} saves {} cutscene {}",
                        outcome.sim.level_tally,
                        outcome.sim.map_node,
                        outcome.sim.level_progress[slot],
                        outcome.sim.save_record(0)[8 + slot],
                        outcome.sim.level_progress_flags,
                        outcome.sim.secret_route,
                        outcome.sim.dead,
                        h.node,
                        h.screen,
                        h.music,
                        h.saves,
                        h.cutscene
                    );
                    println!("TAPE {found}");
                    if let Some(f) = log.as_mut() {
                        writeln!(f, "FOUND arm frame {frame}\nTAPE {found}").unwrap();
                    }
                    return;
                }
                Verdict::Alive {
                    x,
                    y,
                    ground,
                    boss,
                    clear,
                    lock,
                } => {
                    let s = score(&goals, &mut reached, x, y, ground) + boss_score(boss, clear)
                        - i64::from(lock) * 1_000;
                    let key = (
                        x >> 4,
                        y >> 4,
                        ground,
                        reached,
                        if clear { 0xFE } else { boss.unwrap_or(0xFF) },
                        // In an arena the boss's own script can hold Mario in
                        // place, so elapsed time is new ground too.
                        if boss.is_some() {
                            (tape.len() >> 6).min(255) as u8
                        } else {
                            lock
                        },
                    );
                    let better = archive.get(&key).is_none_or(|c| tape.len() < c.tape.len());
                    if better {
                        archive.insert(
                            key,
                            Cell {
                                tape,
                                score: s,
                                picks: 0,
                            },
                        );
                    }
                    if s > best {
                        best = s;
                        if let Some(f) = log.as_mut() {
                            // The tape lets a stopped search resume as a prefix.
                            let tape = archive
                                .get(&key)
                                .map_or(String::new(), |c| to_spans(&c.tape));
                            writeln!(
                                f,
                                "evals {evals} cells {} best {s} at ({x},{y})\nBESTTAPE {tape}",
                                archive.len()
                            )
                            .unwrap();
                        }
                    }
                }
            }
        }
    }
    let top = archive.values().max_by_key(|c| c.score).unwrap();
    println!("BEST score {} TAPE {}", top.score, to_spans(&top.tape));
    panic!("no route found within {budget} evaluations");
}

/// Prints Mario's position after every frame of `ROUTE_TAPE` (spans form),
/// for reading where a stalled search is standing.
#[test]
#[ignore = "authoring tool: run explicitly to trace a tape"]
fn trace_route() {
    let level: u8 = env("ROUTE_LEVEL", 1);
    let progress: u8 = env("ROUTE_PROGRESS", 0);
    let from: usize = env("ROUTE_FROM", 0);
    let tape = spans(&std::env::var("ROUTE_TAPE").expect("ROUTE_TAPE"));
    let assets = RouteAssets::load(level);
    for len in from.max(1)..=tape.len() {
        let (outcome, _) = assets.play(
            progress,
            PlayTape::Buttons(tape[..len].to_vec()),
            |_, _, _, _, _| {},
        );
        println!(
            "TRACE {len} {} ({},{}) ground {} dead {} scene {} boss def {} count {} hits {} phase {} clear {} fade {} tally {}",
            to_spans(&tape[len - 1..len]),
            outcome.sim.x,
            outcome.sim.y,
            outcome.sim.on_ground,
            outcome.sim.dead,
            outcome.cam.scene_id,
            outcome.obj.boss_def_index,
            outcome.obj.boss_def_count,
            outcome.obj.boss_hits_remaining,
            outcome.cam.boss_fight_phase,
            outcome.sim.boss_clear,
            outcome.sim.fade_timer,
            outcome.sim.level_tally
        );
        if switch("ROUTE_SLOTS") {
            // Raw definition and screen position: boss pages bypass `active`.
            let slots: Vec<_> = outcome.obj.slots[..4]
                .iter()
                .map(|slot| (slot.b[5], slot.b[10], slot.b[9]))
                .collect();
            println!(
                "  slots {slots:?} cam ({},{}) handoff {:?} exit {} {:?} map {:?} unported {:?}",
                outcome.cam.x,
                outcome.cam.y,
                outcome.obj.boss_clear_handoff,
                outcome.sim.exit_completed,
                outcome.exit_completed_frame,
                outcome.map_handoff_frame,
                outcome.obj.unported
            );
        }
    }
}
