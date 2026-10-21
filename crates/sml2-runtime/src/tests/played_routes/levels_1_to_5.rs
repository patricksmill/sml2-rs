//! Played approaches for levels 1-5: every header and secret-entry start to
//! every exit cell, through live `run_play` from the source map node with no
//! state rewrites after input begins. The tapes were found by the ignored
//! `search::search_route` tool against this same loop and frozen here; they
//! are native fixtures, not recorded ROM input.
use super::*;

/// What one played route must observe. Positions are Mario's (x, y).
struct Route {
    level: u8,
    progress: u8,
    tape: &'static str,
    entry: (u16, u16, u16, u16),
    node: u8,
    slot: usize,
    records: usize,
    bell: (usize, usize),
    arm_frame: u64,
    arm_position: (u16, u16),
    exit_tile_flag: u8,
    tally: u8,
    progress_after: u8,
    handoff: (u8, Option<u8>, Option<u8>, u8),
}

fn check(route: Route) {
    let tape = spans(route.tape);
    let frames = tape.len() as u64;
    let outcome = run_entry_route(
        route.level,
        route.progress,
        tape,
        |manifest, layout, sim, cam, nodes| {
            assert_eq!(manifest.level_id, route.level - 1);
            assert_eq!(manifest.spawns.records.len(), route.records);
            assert_eq!((sim.x, sim.y, cam.x, cam.y), route.entry);
            assert_eq!(
                (sim.map_node, nodes.progress_slot(sim.map_node)),
                (route.node, Some(route.slot as u8))
            );
            assert_eq!(sim.level_progress_flags, route.progress);
            let (col, row) = route.bell;
            let bell = if route.exit_tile_flag == 0 {
                0x4A
            } else {
                0x4B
            };
            assert_eq!(layout.cells[row * 256 + col], bell);
        },
    );
    let arm = outcome
        .bell_arm
        .expect("the played route must arm the bell");
    assert_eq!(
        (arm.frame, arm.position),
        (route.arm_frame, route.arm_position)
    );
    assert_eq!(
        (
            arm.fade_timer,
            arm.music_request,
            arm.exit_tile_flag,
            arm.dead
        ),
        (192, 3, route.exit_tile_flag, 0)
    );
    assert_eq!(outcome.gameplay_death_frames, 0);
    assert_eq!(outcome.frames, frames);
    assert_eq!(outcome.exit_completed_frame, Some(arm.frame + 192));
    assert_eq!(outcome.map_handoff_frame, Some(arm.frame + 193));
    assert_eq!((outcome.sim.x, outcome.sim.y), arm.position);
    assert_eq!(
        (
            outcome.sim.fade_timer,
            outcome.sim.level_tally,
            outcome.sim.dead
        ),
        (0, route.tally, 0)
    );
    assert!(outcome.sim.exit_completed && outcome.sim.map_handoff_done);
    assert_eq!(outcome.sim.secret_route, 0);
    assert_eq!(outcome.sim.level_progress_flags, route.progress);
    assert_eq!(outcome.sim.level_progress[route.slot], route.progress_after);
    assert_eq!(
        outcome.sim.save_record(0)[8 + route.slot],
        route.progress_after
    );
    let handoff = outcome.map_handoff.expect("the live handoff must run");
    assert_eq!(
        (handoff.node, handoff.screen, handoff.music, handoff.saves),
        route.handoff
    );
    assert_eq!(outcome.sim.map_node, route.handoff.0);
    assert!(!handoff.cutscene);
    assert!(outcome.active_object_frames > 0);
    assert!(outcome.obj.unported.is_empty());
}

/// Level 1: the header spawn to the `$4A` bell at (254, 30).
#[test]
fn level01_header_spawn_to_bell_through_live_play() {
    check(Route {
        level: 1,
        progress: 0,
        tape: concat!(
            "RUAB*218 R*29 RAB*250 R*74 RUAB*68 R*60 RAB*120 RB*24 RAB*26 RB*32 RUAB*108 R*170 ",
            "RUA*104 -*12 UA*12 RB*290 A*24 D*48 RUA*24 R*124 RA*38 RB*48 RUAB*64 D*24 RB*128 ",
            "LB*8 RUAB*66 RB*12 RAB*98 LB*24 RUAB*222 R*8 RA*104 LAB*32 RAB*148 D*32 RAB*106 ",
            "RB*24 RUA*36 RB*48 RA*131 RB*76 RA*12 -*38 -*200 ",
        ),
        entry: (34, 468, 80, 448),
        node: 0,
        slot: 0,
        records: 23,
        bell: (254, 30),
        arm_frame: 3344,
        arm_position: (4057, 468),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (20, None, Some(6), 2),
    });
}

/// Level 1: the secret entry to the `$4A` bell at (254, 30).
#[test]
fn level01_secret_entry_to_bell_through_live_play() {
    check(Route {
        level: 1,
        progress: 8,
        tape: "LA*4 RB*222 R*1 UA*60 R*32 RB*88 L*6 RUAB*34 RB*32 RUAB*132 RB*2 RUAB*315 -*200 ",
        entry: (2778, 420, 2811, 438),
        node: 0,
        slot: 0,
        records: 23,
        bell: (254, 30),
        arm_frame: 928,
        arm_position: (4057, 468),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (20, None, Some(6), 2),
    });
}

/// Level 2: the header spawn to the `$4A` bell at (253, 30).
#[test]
fn level02_header_spawn_to_bell_through_live_play() {
    check(Route {
        level: 2,
        progress: 0,
        tape: concat!(
            "RB*202 RUA*40 D*16 RUAB*288 R*14 A*48 RB*204 LA*32 RAB*80 RA*48 D*10 RB*56 RUAB*32 ",
            "RB*68 LA*36 R*132 RUAB*42 RB*168 RUA*108 RUAB*122 A*36 RA*48 RB*56 R*46 RB*68 RA*88 ",
            "RB*24 LA*18 RA*8 RB*32 R*4 LAB*16 RAB*92 R*28 LUA*12 RUAB*132 RUA*16 RAB*58 LUA*36 ",
            "RB*4 RA*12 R*32 RAB*152 L*2 RAB*178 RA*45 RB*128 RUAB*168 R*1 RAB*74 D*12 RUA*67 ",
            "-*200 ",
        ),
        entry: (34, 468, 80, 448),
        node: 35,
        slot: 1,
        records: 34,
        bell: (253, 30),
        arm_frame: 3439,
        arm_position: (4041, 468),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (35, Some(2), Some(16), 2),
    });
}

/// Level 2: the secret entry to the `$4A` bell at (253, 30).
#[test]
fn level02_secret_entry_to_bell_through_live_play() {
    check(Route {
        level: 2,
        progress: 8,
        tape: concat!(
            "UA*16 RUAB*156 R*2 UA*60 RB*46 RUA*60 RB*24 LUA*14 RUAB*12 RB*46 LB*4 RUA*38 LUA*42 ",
            "RB*152 D*16 RUAB*16 R*32 A*48 L*12 RUAB*12 D*46 RAB*204 R*40 RAB*172 LB*4 RUAB*72 ",
            "RB*32 RAB*210 -*200 ",
        ),
        entry: (2144, 468, 2177, 448),
        node: 35,
        slot: 1,
        records: 34,
        bell: (253, 30),
        arm_frame: 1588,
        arm_position: (4041, 468),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (35, Some(2), Some(16), 2),
    });
}

/// Level 3: the header spawn to the `$4A` bell at (238, 30).
#[test]
fn level03_header_spawn_to_plain_bell_through_live_play() {
    check(Route {
        level: 3,
        progress: 0,
        tape: concat!(
            "RB*84 RUA*198 RB*76 RUA*40 RB*48 R*24 UA*48 RUA*74 D*64 LAB*48 RA*24 RB*52 LA*8 R*32 ",
            "D*2 RA*160 RB*24 A*48 LUA*16 R*56 RA*32 RB*48 A*48 RA*32 RAB*48 D*8 RAB*20 -*24 ",
            "LAB*8 RB*24 RUAB*64 D*2 RUA*52 RUAB*64 R*12 RAB*16 LAB*12 R*8 LAB*12 RAB*270 L*2 ",
            "RAB*100 L*2 UA*36 L*12 RUAB*24 R*12 RUAB*72 -*8 RUAB*32 R*4 RAB*32 RB*2 RUAB*32 R*8 ",
            "D*24 RAB*16 RB*32 UA*34 RUA*48 RAB*4 A*4 RB*32 RUAB*48 D*8 RA*12 LA*2 R*28 RUA*56 ",
            "UA*8 RA*24 L*2 A*12 RUA*44 LB*12 RUA*40 RB*24 RA*52 RAB*104 LB*4 RUAB*48 LAB*8 D*4 ",
            "RAB*2 R*24 UA*32 D*2 A*4 LA*24 RAB*40 D*32 RUAB*48 LA*16 -*8 RA*48 D*20 LA*24 -*32 ",
            "UA*12 RAB*40 RB*8 UA*24 RB*36 LAB*2 -*24 RAB*42 D*24 RAB*16 LB*12 LA*4 UA*20 L*12 ",
            "RAB*24 -*12 LAB*12 -*12 RAB*36 D*16 RUA*24 RB*36 RUAB*48 RB*16 A*4 R*32 RUAB*4 RB*50 ",
            "RUAB*12 UA*8 RAB*40 D*12 A*26 D*1 RUA*24 LB*1 RUA*24 -*1 RAB*244 -*1 RUA*136 RB*128 ",
            "RUAB*225 -*200 ",
        ),
        entry: (46, 164, 80, 176),
        node: 34,
        slot: 2,
        records: 31,
        bell: (238, 30),
        arm_frame: 4883,
        arm_position: (3802, 468),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (34, Some(2), Some(16), 2),
    });
}

/// Level 3: the secret entry to the `$4A` bell at (238, 30).
#[test]
fn level03_secret_entry_to_plain_bell_through_live_play() {
    check(Route {
        level: 3,
        progress: 8,
        tape: concat!(
            "A*24 RB*118 RAB*48 RB*12 RUAB*48 A*32 RUA*20 RUAB*48 D*118 RUAB*16 RA*8 -*24 RB*8 ",
            "RUAB*48 LB*20 RUAB*68 L*12 RA*22 RB*60 LUA*6 R*20 RUAB*68 L*8 RA*44 L*4 RUAB*24 ",
            "LUA*4 RUA*84 -*2 LUA*8 RA*16 D*32 R*4 A*2 RB*24 RUAB*16 D*2 RA*6 RUAB*48 R*4 RUAB*24 ",
            "RA*48 LA*8 RUAB*126 D*32 RA*16 RB*48 RAB*24 LB*12 UA*8 R*1 UA*4 RAB*12 R*2 RAB*48 ",
            "RUA*32 RB*82 RAB*48 -*32 RUAB*140 RA*80 D*10 RUAB*226 -*200 ",
        ),
        entry: (1807, 132, 1823, 142),
        node: 34,
        slot: 2,
        records: 31,
        bell: (238, 30),
        arm_frame: 2243,
        arm_position: (3802, 468),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (34, Some(2), Some(16), 2),
    });
}

/// Level 3: the secret entry to the alternate `$4B` bell at (238, 10).
/// Same side-pipe approach to the sealed `$4B` room as the header start.
#[test]
fn level03_secret_entry_to_alternate_bell_through_live_play() {
    check(Route {
        level: 3,
        progress: 8,
        tape: concat!(
            "RB*12 RAB*32 RB*48 A*12 R*32 RUAB*56 LUA*32 R*28 RUAB*64 D*48 RAB*48 RB*8 RUAB*48 ",
            "RB*24 RUAB*24 L*8 -*8 RB*48 LA*4 UA*8 R*54 LAB*8 RUA*24 R*32 RA*26 -*18 RAB*32 RB*50 ",
            "RUAB*24 LB*10 RUA*16 -*18 RB*8 RAB*52 D*12 RUAB*36 -*44 RB*126 RAB*76 LB*2 RUA*20 ",
            "L*8 RA*12 RB*12 LA*24 LB*4 LA*8 LB*4 RAB*104 R*48 UA*12 RAB*90 RB*60 RUAB*24 UA*16 ",
            "LAB*4 RA*24 LAB*4 RA*48 RB*8 RAB*16 R*45 RUA*4 LB*32 LAB*24 RUA*12 RB*20 RAB*26 ",
            "RB*300 -*200 ",
        ),
        entry: (1807, 132, 1823, 142),
        node: 34,
        slot: 2,
        records: 31,
        bell: (238, 10),
        arm_frame: 2273,
        arm_position: (3802, 148),
        exit_tile_flag: 0xFF,
        tally: 0,
        progress_after: 0x01,
        handoff: (37, Some(2), Some(16), 2),
    });
}

/// Level 4: the header spawn to the `$4A` bell at (222, 30).
#[test]
fn level04_header_spawn_to_bell_through_live_play() {
    check(Route {
        level: 4,
        progress: 0,
        tape: concat!(
            "RAB*56 RB*136 LA*2 R*32 RUAB*38 D*4 R*22 LUA*108 RUA*128 L*24 RA*92 LB*2 RAB*344 ",
            "-*12 LAB*48 R*48 A*24 RB*68 L*36 RA*48 RUAB*44 -*42 LB*4 A*28 RB*100 LA*12 RB*12 ",
            "A*28 RAB*48 R*64 LA*12 RB*48 LUA*48 RB*212 RUA*100 RB*8 RAB*42 R*34 RUA*32 RB*48 ",
            "UA*1 -*34 RAB*318 RB*4 RUAB*84 RB*220 RUAB*104 D*32 RAB*104 D*100 RB*394 -*200 ",
        ),
        entry: (49, 468, 80, 448),
        node: 33,
        slot: 3,
        records: 36,
        bell: (222, 30),
        arm_frame: 3633,
        arm_position: (3546, 468),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (33, Some(2), Some(16), 2),
    });
}

/// Level 4: the secret entry to the `$4A` bell at (222, 30).
#[test]
fn level04_secret_entry_to_bell_through_live_play() {
    check(Route {
        level: 4,
        progress: 8,
        tape: "RAB*174 R*32 RUAB*108 D*116 RAB*382 -*200",
        entry: (2572, 132, 2605, 167),
        node: 33,
        slot: 3,
        records: 36,
        bell: (222, 30),
        arm_frame: 812,
        arm_position: (3546, 468),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (33, Some(2), Some(16), 2),
    });
}

/// Level 5: the header spawn to the `$4A` bell at (78, 14).
/// The climb bonks the hidden `$7E` block at (62,19), stands on its `$07`
/// overlay, and jumps up through the one-way row-15 platforms.
#[test]
fn level05_header_spawn_to_bell_through_live_play() {
    check(Route {
        level: 5,
        progress: 0,
        tape: concat!(
            "RAB*52 RB*64 UA*16 RB*100 LAB*32 RUA*12 LUA*16 RB*48 UA*24 LUA*8 -*50 RAB*84 LB*16 ",
            "UA*2 -*8 LA*8 R*32 LUA*2 A*48 -*8 RA*64 R*16 UA*12 RUAB*78 D*4 A*36 RB*70 A*24 RB*32 ",
            "LUA*32 R*16 UA*24 -*34 LAB*118 LA*42 LB*24 RUAB*45 L*1 UA*32 RB*32 A*12 D*7 RB*6 L*2 ",
            "-*10 UA*50 R*1 UA*12 LUA*19 -*10 UA*25 RB*32 RUAB*162 -*200 ",
        ),
        entry: (61, 676, 80, 673),
        node: 36,
        slot: 4,
        records: 30,
        bell: (78, 14),
        arm_frame: 1714,
        arm_position: (1241, 212),
        exit_tile_flag: 0,
        tally: 1,
        progress_after: 0x80,
        handoff: (36, Some(2), Some(16), 2),
    });
}
