use super::*;
use crate::level::{LevelLayout, LAYOUT_SIZE};

/// Every cell `$60` (the level map's own empty air) — a probe only sees
/// what a test sets explicitly.
fn blank_layout() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap()
}

fn cell_index(row: usize, col: usize) -> u32 {
    (row * 256 + col) as u32
}

/// `$1E8D`'s first (y+32) probe point for a small Mario at (100, 200) with
/// the probe x at 120: row 13, col 7.
fn small_sim() -> PlayerSim {
    let mut sim = PlayerSim::new();
    sim.powerup = 0; // small: the y+20 probe is skipped
    sim.x = 100;
    sim.y = 200;
    sim
}

#[test]
fn collision_samples_the_wram_mirror_below_the_level_layout() {
    let mut layout = blank_layout();
    layout.cells[47 * 256] = 0x31;
    layout.cells[16 * 256] = 0x4C;
    layout.cells[45 * 256] = 0x7F;
    let mut sim = small_sim();
    // `$1EFA`: row 47 -> $DF00; rows 48/77 -> $E000/$FD00,
    // which mirror $C000/$DD00. $FE00 is outside the WRAM mirror.
    assert_eq!(sim.tile(&layout, 8, 768), 0x31);
    assert_eq!(sim.tile(&layout, 8, 784), 0x4C);
    assert_eq!(sim.tile(&layout, 8, 1248), 0x7F);
    assert_eq!(sim.tile(&layout, 8, 1264), 0);

    sim.layout_writes.push((cell_index(16, 0), 0x38));
    assert_eq!(sim.tile(&layout, 8, 784), 0x38);
    assert_eq!(sim.tile(&layout, 8, 272), 0x38);
}

#[test]
fn bottom_hazard_death_does_not_land_on_an_invented_floor() {
    let mut layout = blank_layout();
    layout.cells[47 * 256 + 169] = 0x50;
    let mut sim = small_sim();
    sim.x = 2702;
    sim.y = 742;
    sim.camx = 2736;
    sim.camy = 704;
    sim.powerup = 1;
    sim.hvel = 0xB8;
    sim.vvel = 0;
    sim.on_ground = false;
    // `$1C6D` kills at screen Y $88. `$129F` probes row 48, whose
    // echo of row 16 is air; `$156B` still falls 3px before `$319F`.
    sim.step(
        &PlayerKeys {
            right: true,
            b: true,
            ..Default::default()
        },
        false,
        false,
        &layout,
    );
    assert_eq!(sim.x.to_le_bytes(), [0x8F, 0x0A]);
    assert_eq!(sim.y.to_le_bytes(), [0xE9, 0x02]);
    assert_eq!((sim.hvel, sim.vvel, sim.dead), (0xB8, 0x5F, 0xFF));
    assert!(!sim.on_ground);
}

/// The VBlank tail decrements `sFadeTimer` on **every** in-level frame, and
/// that is not a detail: a scene's own fade is what the boss's parts wait
/// out before leaving their intro state (`$9C8B`), and `$B14E`'s "the fade
/// is idle" test is the gate in front of the whole def-2 sequence. With the
/// decrement missing from the in-level path, the arena intro's 352 frames
/// never expired and every part sat frozen at sprite `$1E` forever.
#[test]
fn the_fade_timer_ticks_down_on_every_in_level_frame() {
    let layout = blank_layout();
    let keys = PlayerKeys::default();
    let mut sim = small_sim();
    sim.fade_timer = 3;
    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.fade_timer, 2);
    sim.step(&keys, false, false, &layout);
    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.fade_timer, 0);
    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.fade_timer, 0, "clamped at zero, and it stays there");

    // A travelling frame counts too — the decrement is in VBlank, not in
    // the state body that `$0F71`'s pipe branch replaces mid-frame.
    sim.fade_timer = 5;
    sim.pipe_dir = 0x30;
    sim.step(&keys, false, false, &layout);
    assert_eq!(
        sim.fade_timer, 4,
        "the decrement runs before the early return"
    );

    // tick_fade_timer decrements independently and clamps to 0
    sim.tick_fade_timer();
    assert_eq!(sim.fade_timer, 3);
    sim.fade_timer = 0;
    sim.tick_fade_timer();
    assert_eq!(sim.fade_timer, 0);
}

/// `$129F`'s second test is `sMarioOnObject`: when it is set the routine
/// jumps to `$1378`, which forces `sMarioOnGround = 255`, clears
/// `sPipeTravelDirection` and returns — the floor probe never runs. The
/// runtime did not set the flag until the object engine's on-object result
/// was threaded back (before that it was false forever), so this reading of
/// it was inert: Mario standing on a walker fell through it for the one
/// frame the probe owned.
#[test]
fn on_object_skips_the_floor_probe_and_keeps_mario_grounded() {
    let layout = blank_layout(); // no floor anywhere
    let mut sim = small_sim();
    sim.vvel = 0;
    sim.on_ground = false;
    sim.on_ground_obj = true;
    sim.collision_tile = 0x99; // a sentinel only the probe would overwrite
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert!(sim.on_ground, "$1378 sets sMarioOnGround = 255");
    assert_eq!(sim.collision_tile, 0x99, "the probe is skipped entirely");
}

/// The control for the test above: with the flag clear the same blank
/// layout falls through to the probe, finds nothing, and launches the
/// grounded-but-fast Mario off the ledge (`$129F`: `hvel - 72 >= $78`,
/// i.e. `hvel >= $C0` — the running-right and running-left bands).
#[test]
fn without_on_object_the_blank_layout_launches_him_off_the_ledge() {
    let layout = blank_layout();
    // Running fast ($C0 is the launch band's floor): the launch fires, and
    // the frame's own later +-1 leaves `vvel` at 27, not 28.
    let mut fast = small_sim();
    fast.vvel = 0;
    fast.on_ground = true;
    fast.on_ground_obj = false;
    fast.hvel = 0xC0;
    fast.step(&PlayerKeys::default(), false, false, &layout);
    assert!(!fast.on_ground, "nothing under him and nothing to stand on");
    assert_eq!(
        fast.vvel, 27,
        "$129F sets 28; the rest of the frame leaves 27 (hvel {})",
        fast.hvel
    );
    // Ambling at the neutral speed, below the band: no launch at all.
    let mut slow = small_sim();
    slow.vvel = 0;
    slow.on_ground = true;
    slow.on_ground_obj = false;
    slow.hvel = 0x80;
    slow.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(slow.vvel, 0, "below $C0 the launch does not fire");
}

/// `$5375`/`$53FE`: the walk-cycle arm's test is
/// `sMarioOnGround | sMarioOnObject`, so a Mario standing on an object
/// draws the standing/walk pose with `sMarioOnGround` clear — the state the
/// runtime left him in while it never set the flag, so the air pose won.
#[test]
fn on_object_draws_the_walk_pose_with_ground_clear() {
    let mut sim = small_sim();
    sim.in_air = true; // he is standing on something that is not the floor
    sim.on_ground = false;
    sim.hvel = 0x80;
    sim.dir_right = true;
    sim.on_ground_obj = true;
    assert_eq!(sim.frame_index(0), 32, "the standing pose, small (+32)");
    sim.on_ground_obj = false;
    assert_eq!(sim.frame_index(0), 40, "the air pose (+32) wins instead");
}

#[test]
fn coin_touch_credits_clears_the_cell_and_is_not_a_wall() {
    let mut layout = blank_layout();
    layout.cells[13 * 256 + 7] = 0x48;
    let mut sim = small_sim();
    assert_eq!(sim.wall_tile(&layout, 120), None); // $48 is not solid
    assert_eq!(sim.sfx0, 7);
    assert_eq!(sim.coins_low, 0x01);
    assert_eq!(sim.coins_high, 0x00);
    assert_eq!(sim.layout_writes, vec![(cell_index(13, 7), 0x60)]);
    // The overlay is what the next probe reads: no double credit.
    assert_eq!(sim.wall_tile(&layout, 120), None);
    assert_eq!(sim.coins_low, 0x01);
    assert_eq!(sim.layout_writes.len(), 1);
}

/// `$1DA5`'s `cp $48` is exact: a `$49` (the `& $FE == $48` door) collects
/// too, but writes the `$7F` tile, not `$60`.
#[test]
fn coin_touch_writes_7f_for_the_49_tile() {
    let mut layout = blank_layout();
    layout.cells[13 * 256 + 7] = 0x49;
    let mut sim = small_sim();
    assert_eq!(sim.wall_tile(&layout, 120), None);
    assert_eq!(sim.coins_low, 0x01);
    assert_eq!(sim.layout_writes, vec![(cell_index(13, 7), 0x7F)]);
}

/// Big Mario probes y+20 first — (120, 220) = row 12, col 7 here.
#[test]
fn coin_touch_runs_on_the_y20_probe_too() {
    let mut layout = blank_layout();
    layout.cells[12 * 256 + 7] = 0x48;
    let mut sim = PlayerSim::new(); // `$0F2A` default: big, not ducking
    sim.x = 100;
    sim.y = 200;
    assert_eq!(sim.wall_tile(&layout, 120), None);
    assert_eq!(sim.coins_low, 0x01);
    assert_eq!(sim.layout_writes, vec![(cell_index(12, 7), 0x60)]);
}

/// A solid tile still ends the probe as a wall and collects nothing.
#[test]
fn solid_tile_still_wins_over_the_touch() {
    let mut layout = blank_layout();
    layout.cells[13 * 256 + 7] = 0x10;
    let mut sim = small_sim();
    assert_eq!(sim.wall_tile(&layout, 120), Some(0x10));
    assert_eq!(sim.coins_low, 0x00);
    assert!(sim.layout_writes.is_empty());
}

/// The BCD credit: 09 -> 10, 99 + carry -> 00 with the high byte, and the
/// `$0999` clamp.
#[test]
fn coin_credit_is_bcd_and_caps_at_0999() {
    let mut sim = PlayerSim::new();
    sim.coins_low = 0x09;
    sim.coin_credit();
    assert_eq!((sim.coins_high, sim.coins_low), (0x00, 0x10));
    sim.coins_low = 0x99;
    sim.coin_credit();
    assert_eq!((sim.coins_high, sim.coins_low), (0x01, 0x00));
    sim.coins_high = 0x09;
    sim.coins_low = 0x99;
    sim.coin_credit();
    assert_eq!((sim.coins_high, sim.coins_low), (0x09, 0x99));
}

// -----------------------------------------------------------------
// $1BFF's exit-bell arm ($1CBA-$1D01). Level 1's only `$4A` sits at the
// very east end (col 254 / row 30, world x 4064) — past every capture's
// range — so the arm is pinned here rather than by a replay.
// -----------------------------------------------------------------

/// For a small Mario at (100, 200) the `(x+16, y+28)` environment sample
/// is (116, 228) = row 13, col 6.
const EXIT_CELL: usize = 13 * 256 + 6;

/// `$4A` (the plain exit): the arm clears everything it owns, requests
/// the level-clear music and starts the 192-frame exit fade.
#[test]
fn exit_bell_4a_starts_the_level_clear() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x4A;
    let mut sim = small_sim();
    // Pre-seed every byte the arm must overwrite.
    sim.sfx0 = 20;
    sim.sfx1 = 20;
    sim.music_request = 9;
    sim.low_time_flag = 0xFF;
    sim.secret_route = 0xFF;
    sim.tile_anim_config = 0x31;
    assert!(sim.exit_bell_probe(&layout));
    assert_eq!(sim.exit_tile_flag, 0x00);
    assert_eq!(sim.low_time_flag, 0);
    assert_eq!(sim.music_request, 3);
    assert_eq!(sim.sfx0, 0);
    assert_eq!(sim.sfx1, 0);
    assert!(sim.exit_phase);
    assert_eq!(sim.fade_timer, 192);
    assert_eq!(sim.secret_route, 0);
    assert_eq!(sim.tile_anim_config, 0);
}

/// `$4B` is the "route already cleared" twin: same arm, but
/// `sExitTileFlag` becomes `$FF` (`$3547`'s tally guard reads it back).
#[test]
fn exit_bell_4b_sets_the_already_cleared_flag() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x4B;
    let mut sim = small_sim();
    assert!(sim.exit_bell_probe(&layout));
    assert_eq!(sim.exit_tile_flag, 0xFF);
    assert!(sim.exit_phase);
    assert_eq!(sim.fade_timer, 192);
}

/// The arm is the `$4A/$4B` pair exactly — the neighbouring coin
/// (`$48/$49`) and water (`$4C-$4F`) classes are not exits, and neither
/// is anything else.
#[test]
fn exit_bell_needs_the_exact_4a_4b_pair() {
    for tile in [0x00, 0x38, 0x48, 0x49, 0x4C, 0x4D, 0x52, 0x60, 0x7E] {
        let mut layout = blank_layout();
        layout.cells[EXIT_CELL] = tile;
        let mut sim = small_sim();
        assert!(
            !sim.exit_bell_probe(&layout),
            "tile ${tile:02X} must not arm the exit"
        );
        assert!(!sim.exit_phase);
    }
}

/// `$1BFF`'s tail order: the water arm reads the same sample, so a `$4C`
/// tile swims and never arms the exit.
#[test]
fn water_beats_the_exit_arm_at_the_same_sample() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x4C;
    let mut sim = small_sim();
    assert_eq!(sim.environment_probe(&layout, &PlayerKeys::default()), None);
    assert!(sim.swim);
    assert!(!sim.exit_phase);
}

/// `$1BFF` never collects: a `$48/$49` at the environment sample arms no
/// exit and credits no coin (only the side probes' `$1D75` collects).
#[test]
fn exit_sample_never_credits_a_coin() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x49;
    let mut sim = small_sim();
    assert_eq!(sim.environment_probe(&layout, &PlayerKeys::default()), None);
    assert_eq!(sim.coins_low, 0);
    assert!(sim.layout_writes.is_empty());
    assert!(!sim.exit_phase);
}

/// The hazard arm `jp`s the tail, so a `$50/$51` tile at the big-Mario
/// `y+22` probe blocks the bell even with a `$4A` one row below.
#[test]
fn hazard_arm_blocks_the_exit_probe() {
    let mut layout = blank_layout();
    layout.cells[12 * 256 + 6] = 0x50; // y+22 → the big-Mario hazard probe
    layout.cells[EXIT_CELL] = 0x4A; // y+28 → a bell one row down
    let mut sim = PlayerSim::new(); // $0F2A default: big, not ducking
    sim.x = 100;
    sim.y = 200;
    assert_eq!(
        sim.environment_probe(&layout, &PlayerKeys::default()),
        Some(15)
    );
    assert!(!sim.exit_phase);
    assert_eq!(sim.exit_tile_flag, 0);
}

/// `$1C39`: big Mario falls through from the `y+22` hazard probe to
/// `y+28`, so a `$51` one row below still kills (and still blocks the
/// exit arm). Every capture runs small, so no replay reaches this arm.
#[test]
fn hazard_probe_falls_through_to_the_y28_sample() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x51;
    let mut sim = PlayerSim::new();
    sim.x = 100;
    sim.y = 200;
    assert_eq!(
        sim.environment_probe(&layout, &PlayerKeys::default()),
        Some(15)
    );
    assert!(!sim.exit_phase);
}

/// The arm through the real per-frame entry point: `step` arms the level
/// clear without moving Mario.
#[test]
fn step_arms_the_exit_bell_in_place() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x4A;
    let mut sim = small_sim();
    let keys = PlayerKeys::default();
    sim.step(&keys, false, false, &layout);
    assert!(sim.exit_phase);
    assert_eq!(sim.fade_timer, 192);
    assert_eq!(sim.music_request, 3);
    assert_eq!(sim.exit_tile_flag, 0);
}

// -----------------------------------------------------------------
// $3547's tail ($18): the exit fade's last step runs the BCD level
// tally and hands off to `$14` ($FF9B = 20). Level 1's only `$4A` is
// past every capture's east end, so the whole tail is pinned here.
// -----------------------------------------------------------------

/// A plain `$4A` exit: nothing is credited while the 192-frame fade
/// runs; the call that reaches 0 steps `sLevelTally` 0 -> 1 and marks
/// the `$14` hand-off. Past the tail the step is a no-op.
#[test]
fn exit_fade_tally_counts_a_plain_4a_exit() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x4A;
    let mut sim = small_sim();
    assert!(sim.exit_bell_probe(&layout));
    assert_eq!(sim.fade_timer, 192);
    // The arm frame itself never decrements (VBlank ticked before the
    // in-level update set 192), so the 191 calls leave the timer at 1.
    for _ in 0..191 {
        sim.exit_fade_step();
    }
    assert_eq!(sim.fade_timer, 1);
    assert_eq!(sim.level_tally, 0);
    assert!(!sim.exit_completed);
    // The 192nd call is `$3547`'s tail.
    sim.exit_fade_step();
    assert_eq!(sim.fade_timer, 0);
    assert_eq!(sim.level_tally, 0x01);
    assert!(sim.exit_completed);
    // No second credit once the hand-off is marked.
    sim.exit_fade_step();
    assert_eq!(sim.level_tally, 0x01);
}

/// `add 1 / daa`: the tally is BCD, including the 99 -> 00 wrap (the
/// carry is discarded — `sLevelTally` is a single byte).
#[test]
fn exit_fade_tally_is_bcd() {
    for (from, to) in [(0x00u8, 0x01u8), (0x09, 0x10), (0x19, 0x20), (0x99, 0x00)] {
        let mut sim = small_sim();
        sim.exit_phase = true;
        sim.fade_timer = 1;
        sim.level_tally = from;
        sim.exit_fade_step();
        assert_eq!(
            sim.level_tally, to,
            "tally ${from:02X} must step to ${to:02X}"
        );
        assert!(sim.exit_completed);
    }
}

/// The `$4B` ("route already cleared") exit sets `sExitTileFlag = $FF`,
/// and `$3547`'s `and a / ret nz` guard skips the tally — but the `$14`
/// hand-off still happens.
#[test]
fn exit_fade_tally_skips_the_already_cleared_4b_exit() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x4B;
    let mut sim = small_sim();
    sim.level_tally = 0x07;
    assert!(sim.exit_bell_probe(&layout));
    assert_eq!(sim.exit_tile_flag, 0xFF);
    for _ in 0..192 {
        sim.exit_fade_step();
    }
    assert_eq!(sim.fade_timer, 0);
    assert_eq!(sim.level_tally, 0x07);
    assert!(sim.exit_completed);
}

/// `sLevelProgressFlags` bit 7 (the stage already cleared) guards the
/// tally too — and only bit 7: a flags byte with other bits set and bit
/// 7 clear still counts.
#[test]
fn exit_fade_tally_skips_when_progress_bit7_is_set() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x4A;
    let mut sim = small_sim();
    sim.level_tally = 0x12;
    sim.level_progress_flags = 0x80;
    assert!(sim.exit_bell_probe(&layout));
    for _ in 0..192 {
        sim.exit_fade_step();
    }
    assert_eq!(sim.level_tally, 0x12);
    assert!(sim.exit_completed);
    // Other flags bits are not the guard: bit 0 alone still counts.
    sim.exit_completed = false;
    sim.level_progress_flags = 0x01;
    sim.exit_phase = true;
    sim.fade_timer = 1;
    sim.exit_fade_step();
    assert_eq!(sim.level_tally, 0x13);
}

/// Outside the exit phase the step must not touch the timer (the death
/// fade runs its own `fade_step`), and a zeroed timer must not credit.
#[test]
fn exit_fade_step_needs_the_exit_phase() {
    let mut sim = small_sim();
    sim.fade_timer = 5;
    sim.exit_fade_step();
    assert_eq!(sim.fade_timer, 5);
    assert_eq!(sim.level_tally, 0);
    assert!(!sim.exit_completed);
}

/// `$532D`: while `sSpinJump` is set the pose comes from `$5348`, indexed by
/// `($FF97 & $0C) >> 2` — and it overrides the grounded and air poses, which
/// is the point of the branch. Big Mario (`powerup = 1`) so the `$5447`
/// powerup offset is 0 and the table values are visible as-is.
#[test]
fn the_spin_jump_pose_overrides_every_other_frame() {
    let mut sim = small_sim();
    sim.powerup = 1;
    sim.on_ground = true;
    sim.dir_right = true;
    sim.hvel = 0x80;
    assert_eq!(sim.frame_index(0), 0, "the standing pose, with no spin");

    sim.spin_jump = 0xFF;
    for (counter, want) in [
        (0x00u8, 0x04u8),
        (0x04, 0x10),
        (0x08, 0x00),
        (0x0C, 0x11),
        (0x14, 0x10), // wraps: bit 2 alone would give 1, not 3
    ] {
        sim.frame_counter = counter;
        sim.phase = counter & 7;
        assert_eq!(sim.frame_index(0), want, "$FF97 = ${counter:02X}");
    }

    // It overrides the air pose too, not only the standing one.
    sim.frame_counter = 0;
    sim.phase = 0;
    sim.on_ground = false;
    sim.in_air = true;
    assert_eq!(sim.frame_index(0), 0x04, "still the spin frame");

    // ...but `$530D` tests the pipe direction first, so a spinning Mario in
    // a pipe still shows the pipe frame.
    sim.pipe_dir = 0x20;
    assert_eq!(sim.frame_index(0), 16, "the pipe frame wins");

    // With the spin cleared the normal pose comes back.
    sim.pipe_dir = 0;
    sim.in_air = false;
    sim.on_ground = true;
    sim.spin_jump = 0;
    sim.hvel = 0x80;
    assert_eq!(sim.frame_index(0), 0);
}

/// The write-back list both frame loops call. The play loop's own copy of it
/// was once missing `on_ground_cleared` and `on_ground_obj` outright — a
/// divergence no capture could show, because only `--play` had it. One
/// function is what stops the two drifting apart again.
#[test]
fn the_object_write_back_covers_every_field() {
    let mut sim = small_sim();
    sim.on_ground = true;
    let mut obj = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj.mario_dead = 15;
    obj.pending_item = 9;
    obj.mario_y_adjust = Some(0x1234);
    obj.on_ground_cleared = true;
    obj.on_object = true;
    obj.stomp_jump = true;
    obj.flutter_disarmed = true;
    sim.flutter_armed = true;
    sim.flutter_charge = 5;

    sim.apply_object_result(&mut obj, true);
    assert_eq!(sim.dead, 15, "a harmful hit");
    assert_eq!(sim.pending_item, 9, "$2DC6's staged item");
    assert_eq!(obj.pending_item, 0, "and it is consumed from the engine");
    assert_eq!(sim.y, 0x1234, "the interaction pass moved him");
    assert!(!sim.on_ground, "$2D7D's landing arm cleared it");
    assert!(sim.on_ground_obj, "and the bit5 object raised this");
    assert!(sim.in_air && sim.vvel == 75, "A held: the high stomp jump");
    assert!(!sim.flutter_armed, "$2E23 clears the flutter latch");
    assert_eq!(sim.flutter_charge, 5, "the bounce leaves the charge");

    // Without A held the stomp jump is 48; a bounce is 49 either way.
    let mut sim2 = small_sim();
    let mut obj2 = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj2.stomp_jump = true;
    sim2.apply_object_result(&mut obj2, false);
    assert_eq!(sim2.vvel, 48, "$2E5C without A");
    let mut sim3 = small_sim();
    let mut obj3 = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj3.stomp_bounce = true;
    sim3.apply_object_result(&mut obj3, false);
    assert_eq!(sim3.vvel, 49, "the brick/bounce arm's fixed 49");
    assert!(sim3.no_jump_cut, "$2E5C: the scene bounce ignores the A release");
    assert!(!sim2.no_jump_cut, "the ordinary stomp jump can still be cut");
    // In a boss scene `$2E5C` and `$2D7D`'s stomp tail both run: the stomp's
    // 48 lands last, and the bounce's no-cut flag stays.
    let mut sim5 = small_sim();
    let mut obj5 = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj5.stomp_bounce = true;
    obj5.stomp_jump = true;
    sim5.apply_object_result(&mut obj5, false);
    assert_eq!((sim5.vvel, sim5.no_jump_cut), (48, true));

    // An already-dead Mario keeps his byte: `$8000`'s dispatcher gate read it
    // on this pass, and the write-back does not re-raise it.
    let mut sim4 = small_sim();
    sim4.dead = 1;
    let mut obj4 = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj4.mario_dead = 15;
    sim4.apply_object_result(&mut obj4, false);
    assert_eq!(sim4.dead, 1);
}

// -----------------------------------------------------------------
// $14 — the level-clear/map hand-off ($2C23 -> bank 15 $6F2B).
//
// The gate cannot see any of this: no capture runs past the exit fade
// (`$FF9B = $14` freezes the port), so every branch is pinned here.
// -----------------------------------------------------------------

/// A node table with just what these tests need: node 0 is level 1 with
/// progress slot 0, node 1 has a slot but no level (`$4218 = $FE`, the
/// entry gate's path-node case), node `$05` has slot `$1F`, and everything
/// else is "none". Slot 0's key mask offers every direction and slot `$1F`'s
/// only Right, so the gate's mask test can be told apart from its others.
fn handoff_nodes() -> crate::map::MapNodes {
    use crate::map::MapNodes;
    let mut b = vec![0xFEu8; MapNodes::BLOB_LEN];
    b[0] = 0x00;
    b[0x05] = 0x11; // the real $4218[$05], so $05 is a level node not a path
    b[MapNodes::NODES] = 0x00;
    b[MapNodes::NODES + 1] = 0x00;
    b[MapNodes::NODES + 0x05] = 0x1F;
    b[2 * MapNodes::NODES] = 0xF0;
    b[2 * MapNodes::NODES + 0x1F] = 0x10;
    MapNodes::from_bytes(&b).expect("302-byte blob")
}

/// `$3E1D7`: the entry gate's four conditions in the ROM's order, plus the
/// A/START confirmation `$3C5F4` applies on top of them.
#[test]
fn the_map_entry_gate_needs_the_screen_the_direction_and_an_uncleared_stage() {
    let nodes = handoff_nodes();
    let mut sim = small_sim();
    assert!(sim.map_can_enter(&nodes, 1, 0x40), "screen 1, Up held");
    assert!(!sim.map_can_enter(&nodes, 0, 0x40), "the hub only walks");
    assert!(!sim.map_can_enter(&nodes, 1, 0x00), "nothing held");
    assert!(
        !sim.map_can_enter(&nodes, 1, 0x0F),
        "the low bits are not directions"
    );

    // A stage already cleared is not re-entered through this gate.
    sim.level_progress[0] = 0x80;
    assert!(!sim.map_can_enter(&nodes, 1, 0x40));
    sim.level_progress[0] = 0;

    // `$4218[node] == $FE` is the pure-path-node case, and it is checked
    // before the slot lookup.
    sim.map_node = 1;
    assert!(!sim.map_can_enter(&nodes, 1, 0x40));

    // The mask must offer the direction held (node $05's slot $1F: Right).
    sim.map_node = 0x05;
    assert!(!sim.map_can_enter(&nodes, 1, 0x40), "Up is not offered");
    assert!(sim.map_can_enter(&nodes, 1, 0x10), "Right is");

    // A node with no progress slot at all, and no table loaded.
    sim.map_node = 0x07;
    assert!(!sim.map_can_enter(&nodes, 1, 0x40));
    assert!(!sim.map_can_enter(&crate::map::MapNodes::default(), 1, 0x40));

    // `$3C5F4`: A (bit 0) or START (bit 3), pressed, not held.
    assert!(!PlayerSim::map_confirm(0x00));
    assert!(PlayerSim::map_confirm(0x01), "A");
    assert!(PlayerSim::map_confirm(0x08), "START");
    assert!(!PlayerSim::map_confirm(0x06), "B and Select are not it");
}

/// Level 1's own case: the `$4A` exit bell sets neither `sGoalBell` nor
/// `sExitTileFlag`, and a level entered from the map has node 0 — which is
/// exactly the branch that marks the stage cleared and moves the node on.
/// The `$3EEF0` save and the `$3F03D` save both fire.
#[test]
fn map_handoff_marks_a_plain_level_cleared() {
    let nodes = handoff_nodes();
    let mut sim = small_sim();
    assert_eq!(sim.map_node, 0, "a fresh sim is on node 0");
    assert!(!sim.goal_bell && sim.exit_tile_flag == 0 && sim.dead == 0);

    let out = sim.map_handoff(&nodes);
    assert!(!out.cutscene, "no bell: pass 1 does not run");
    assert_eq!(out.saves, 2, "$3EEF0 then $3F03D");
    assert_eq!(out.node, 0x14);
    assert_eq!(
        sim.level_progress[0] & 0x80,
        0x80,
        "sLevelProgress[0] bit 7 — the persisted byte, not sLevelProgressFlags"
    );
    assert_eq!(sim.map_node, 0x14, "and the node moves to 20");

    let again = sim.map_handoff(&nodes);
    assert_eq!(again.saves, 0, "the ROM sets $FF9B = 12, so it runs once");
    assert_eq!(sim.map_node, 0x14);
}

/// `$3EF2B`'s bell branch: `call $3F4B1` (or `$3F5A4`) then
/// `ld a,12 / ldh [$FF9B],a / ei / ret` — the castle reveal, which CLEARS
/// `sGoalBell` and leaves the level through the state byte without running
/// the map-return half. The port used to fall through into pass 2 here; the
/// ROM returns, and the level's own progress write does not happen on this
/// path (the castle branch does its own bookkeeping in `$3F4B1`/`$3F5A4`).
#[test]
fn map_handoff_bell_path_is_the_castle_branch_and_returns() {
    let nodes = handoff_nodes();

    // `$3EF2B` tests `sGoalBell` first: a ringing bell takes the castle
    // branch (`$3F4B1`, or `$3F5A4` when `sMapNode` bit 0 is set) and RETURNS
    // at `$3EF4F` — no node rewrite, no `$3EEF0`, no tail save, no progress
    // write. The port used to fall through and run the map return for this
    // case; that assertion passed `saves == 0` for node `$48` only because the
    // `$1E`-and-up tail was unmodelled.
    let mut sim = small_sim();
    sim.goal_bell = true;
    let out = sim.map_handoff(&nodes);
    assert!(out.cutscene, "the castle branch ran");
    assert!(!sim.goal_bell, "$3F4B1/$3F5A4 clear sGoalBell");
    assert_eq!(out.saves, 0, "the bell branch returns before any $2934");
    assert_eq!(sim.map_node, 0, "and before the node tail");
    assert_eq!(
        sim.level_progress[0] & 0x80,
        0,
        "nor does it write a progress byte"
    );
    assert_eq!(out.screen, None);

    let mut high = small_sim();
    high.goal_bell = true;
    high.map_node = 0x48;
    let out_high = high.map_handoff(&nodes);
    assert!(out_high.cutscene);
    assert!(!high.goal_bell);
    assert_eq!(
        out_high.saves, 0,
        "the bell branch returns before any $2934"
    );
    assert_eq!(high.map_node, 0x48, "the node is left alone");
    assert_eq!(out_high.screen, None);
}

/// The `$4B` secret exit rewrites the node: that exit opens a different
/// node than the one the level was entered from.
#[test]
fn map_handoff_rewrites_the_node_for_the_secret_exit() {
    let nodes = handoff_nodes();
    for (from, want) in [
        (0x48u8, 0x47u8),
        (0x05, 0x69),
        (0x12, 0x69),
        (0x1A, 0x69),
        (0x29, 0x2C),
        (0x2A, 0x2F),
        (0x3C, 0x43), // $3C -> $40 (pass 2) -> $43 (space zone loader $3D540)
    ] {
        let mut sim = small_sim();
        sim.map_node = from;
        sim.exit_tile_flag = 0xFF;
        let out = sim.map_handoff(&nodes);
        assert_eq!(sim.map_node, want, "node ${from:02X}");
        assert_eq!(out.node, want);
    }
    // A node the chain does not know is left alone; being neither 0 nor
    // $05/$1A it takes none of the tail's branches either.
    let mut sim = small_sim();
    sim.map_node = 0x07;
    sim.exit_tile_flag = 0xFF;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x07);
    assert_eq!(sim.level_progress[0] & 0x80, 0);
    assert_eq!(out.saves, 1, "the $4B arm still saves");
}

/// `$3EEF0`'s three arms, all gated on a live Mario except the first: the
/// `$4B` exit clears bit 3, a plain clear sets bit 7 and clears bit 3, and a
/// dead Mario on the secret route sets bit 3.
#[test]
fn map_handoff_marks_the_nodes_progress_entry() {
    let nodes = handoff_nodes();

    // The $4B exit: bit 3 cleared.
    let mut sim = small_sim();
    sim.exit_tile_flag = 0xFF;
    sim.level_progress[0] = 0x88;
    sim.map_handoff(&nodes);
    assert_eq!(sim.level_progress[0], 0x80, "bit 3 cleared, bit 7 kept");

    // A plain clear: `set 7` then `res 3`, so 0x08 becomes 0x80.
    let mut plain = small_sim();
    plain.level_progress[0] = 0x08;
    plain.map_handoff(&nodes);
    assert_eq!(plain.level_progress[0], 0x80);

    // Dead on the secret route: bit 3 set, and nothing else.
    let mut dead = small_sim();
    dead.dead = 0x0F;
    dead.secret_route = 0xFF;
    dead.map_handoff(&nodes);
    assert_eq!(dead.level_progress[0], 0x08);
    assert_eq!(dead.map_node, 0, "a dead Mario's node does not move");

    // Dead, but not on the secret route: no progress entry at all.
    let mut dead_plain = small_sim();
    dead_plain.dead = 0x80;
    dead_plain.map_handoff(&nodes);
    assert_eq!(dead_plain.level_progress[0], 0);
    assert_eq!(dead_plain.map_node, 0);
}

/// The `$05`/`$1A` nodes move to the castle node `$3F` — but only when the
/// secret-exit rewrite has not already sent them to `$69`.
#[test]
fn map_handoff_moves_the_castle_nodes() {
    let nodes = handoff_nodes();
    for from in [0x05u8, 0x1A] {
        let mut sim = small_sim();
        sim.map_node = from;
        sim.map_handoff(&nodes);
        assert_eq!(sim.map_node, 0x3F, "node ${from:02X} -> the castle");
    }
    let mut secret = small_sim();
    secret.map_node = 0x05;
    secret.exit_tile_flag = 0xFF;
    secret.map_handoff(&nodes);
    assert_eq!(secret.map_node, 0x69, "the $4B rewrite wins");
}

/// `$3F04D`'s bands: every node at or above `$1E` selects a zone screen on the
/// way back to the map, and the ones `$3F142` catches select none. The screens
/// themselves (`$3D0FF`/`$3D208`/… — their gfx, scripts, cursor and camera)
/// are NOT ported, so the selection is reported in `out.screen` rather than
/// entered; the frame loops print a `note:` for it.
#[test]
fn map_handoff_tail_selects_the_zone_screen_bands() {
    let nodes = handoff_nodes();
    for (node, screen) in [
        (0x1Eu8, 2u8),
        (0x27, 2),
        (0x28, 1),
        (0x31, 1),
        (0x32, 3),
        (0x3B, 3),
        (0x3C, 4),
        (0x45, 4),
        (0x46, 5),
        (0x4F, 5),
        (0x50, 6),
        (0x59, 6),
        (0x5A, 7),
        (0x63, 7),
        (0x6A, 3),
        (0x6E, 8),
        (0x77, 8),
        (0x78, 1),
        (0x7A, 1),
    ] {
        let mut sim = small_sim();
        sim.map_node = node;
        let out = sim.map_handoff(&nodes);
        assert_eq!(out.screen, Some(screen), "node ${node:02X}");
    }
    // `$3F142`'s tail — a node with no band of its own.
    for node in [0x64u8, 0x65, 0x67, 0x69, 0x79, 0x7B] {
        let mut sim = small_sim();
        sim.map_node = node;
        let out = sim.map_handoff(&nodes);
        assert_eq!(out.screen, None, "node ${node:02X} has no band");
        assert_eq!(out.saves, 1, "but $3F14C still saves once");
    }
}

/// The four rewriting nodes: `$5B`→`$5C` un-gated, `$66`→`$68` with the
/// Mario-Zone gate, `$78`→`$2E` with Pumpkin gate B and `$7A`→`$31` with gate
/// C. The gate write is `set 7, [hl]` (an OR) and needs a LIVE Mario — but the
/// rewrite and the screen selection happen either way.
#[test]
fn map_handoff_tail_rewrites_the_gate_nodes() {
    use crate::map::progress_offset as po;
    let nodes = handoff_nodes();

    let mut sim = small_sim();
    sim.map_node = 0x5B;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x5C, "ungated rewrite");
    assert_eq!(out.screen, Some(7));

    for (node, want, gate, screen) in [
        (0x66u8, 0x68u8, po::GATE_MARIO, None),
        (0x78, 0x2E, po::GATE_PUMPKIN_B, Some(1u8)),
        (0x7A, 0x31, po::GATE_PUMPKIN_C, Some(1)),
    ] {
        let mut live = small_sim();
        live.map_node = node;
        live.level_progress[gate] = 0x01; // `set 7` must OR, not store
        let live_out = live.map_handoff(&nodes);
        assert_eq!(live.map_node, want, "node ${node:02X}");
        assert_eq!(live.level_progress[gate], 0x81, "set 7, [hl]");
        assert_eq!(live_out.screen, screen, "node ${node:02X}");

        let mut dead = small_sim();
        dead.map_node = node;
        dead.dead = 0x80;
        let dead_out = dead.map_handoff(&nodes);
        assert_eq!(dead.map_node, want, "the rewrite is not gated on death");
        assert_eq!(dead.level_progress[gate], 0, "a dead Mario sets no gate");
        assert_eq!(dead_out.screen, screen, "the screen is selected either way");
    }
}

/// `$3F014`: a live Mario on the `$05`/`$1A` nodes moves to the castle WITH
/// zone screen 4 selected (`$3F07E` → `$3D49F` → `sMapScreenId = 4`); a dead
/// Mario goes straight to `$3F03D`, node unchanged and no screen.
#[test]
fn map_handoff_castle_arm_selects_screen_4_only_for_a_live_mario() {
    let nodes = handoff_nodes();
    for from in [0x05u8, 0x1A] {
        let mut live = small_sim();
        live.map_node = from;
        let out = live.map_handoff(&nodes);
        assert_eq!(live.map_node, 0x3F, "node ${from:02X} -> the castle");
        assert_eq!(out.screen, Some(4));

        let mut dead = small_sim();
        dead.map_node = from;
        dead.dead = 0x80;
        let dead_out = dead.map_handoff(&nodes);
        assert_eq!(dead.map_node, from, "a dead Mario's node does not move");
        assert_eq!(dead_out.screen, None);
        assert_eq!(dead_out.music, Some(6));
    }
}

/// `$3D0FF`..`$3D880`: zone screen loader interactions during level hand-off.
///
/// Verifies that secret clears unlock zone paths, retarget nodes where
/// specified by the ROM loaders, and set the loader-requested music track.
#[test]
fn map_handoff_zone_loaders_paths_retargets_and_music() {
    use crate::map::progress_offset as po;
    let nodes = handoff_nodes();

    // Tree Zone A secret exit ($3D18E): retargets to node 37, sets path 1 bit 0, music 16
    let mut sim = small_sim();
    sim.map_node = 0x23;
    sim.exit_tile_flag = 0x80;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x25, "node 37 in decimal");
    assert_eq!(sim.level_progress[po::TREE_PATH_1] & 1, 1);
    assert_eq!(out.screen, Some(2));
    assert_eq!(out.music, Some(16));

    // Tree Zone A node $26 clear ($3D1B5): retargets to 39, sets dead = $FF
    let mut sim = small_sim();
    sim.map_node = 0x26;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x27, "node 39 in decimal");
    assert_eq!(sim.dead, 0xFF);
    assert_eq!(out.music, Some(16));

    // Pumpkin Zone secret exits ($3D29F): node $2C and $2F
    let mut sim = small_sim();
    sim.map_node = 0x29; // secret rewrite sends $29 -> $2C
    sim.exit_tile_flag = 0x80;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.level_progress[po::PUMPKIN_PATH_1] & 1, 1);
    assert_eq!(out.screen, Some(1));
    assert_eq!(out.music, Some(30));

    let mut sim = small_sim();
    sim.map_node = 0x2A; // secret rewrite sends $2A -> $2F
    sim.exit_tile_flag = 0x80;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.level_progress[po::PUMPKIN_PATH_2] & 1, 1);
    assert_eq!(out.music, Some(30));

    // Macro Zone secret exit ($3D409): node $32 -> node 107 ($6B), path 0 bit 0
    let mut sim = small_sim();
    sim.map_node = 0x32;
    sim.exit_tile_flag = 0x80;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x6B, "node 107 in decimal");
    assert_eq!(sim.level_progress[po::MACRO_PATH_0] & 1, 1);
    assert_eq!(out.screen, Some(3));
    assert_eq!(out.music, Some(5));

    // Macro Zone node $6A ($3D43D): retargets to 108 ($6C), sets gate bit 7, dead = $FF
    let mut sim = small_sim();
    sim.map_node = 0x6A;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x6C, "node 108 in decimal");
    assert_eq!(sim.level_progress[po::GATE_MACRO] & 0x80, 0x80);
    assert_eq!(sim.dead, 0xFF);
    assert_eq!(out.screen, Some(3));
    assert_eq!(out.music, Some(5));

    // Space Zone secret exit ($3D533): node $3C -> $40 (secret rewrite) -> node 67 ($43)
    let mut sim = small_sim();
    sim.map_node = 0x3C;
    sim.exit_tile_flag = 0x80;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x43, "node 67 in decimal");
    assert_eq!(sim.level_progress[po::SPACE_PATH_0] & 1, 1);
    assert_eq!(out.screen, Some(4));
    assert_eq!(out.music, Some(28));

    // Space Zone node $44 ($3D56B): retargets to 69 ($45), dead = $FF
    let mut sim = small_sim();
    sim.map_node = 0x44;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x45, "node 69 in decimal");
    assert_eq!(sim.dead, 0xFF);
    assert_eq!(out.screen, Some(4));
    assert_eq!(out.music, Some(28));

    // Tree Zone B secret exit ($3D69C): path 1 bit 0
    let mut sim = small_sim();
    sim.map_node = 0x48; // secret rewrite sends $48 -> $47
    sim.exit_tile_flag = 0x80;
    let out = sim.map_handoff(&nodes);
    assert_eq!(sim.level_progress[po::TURTLE_PATH_1] & 1, 1);
    assert_eq!(out.screen, Some(5));
    assert_eq!(out.music, Some(18));
}

/// The tail's own `$2934`: every path through `$3F003` saves exactly once
/// (`$3F03D`, `$3F147` and `$3F14C` each end in a `call $2934`), on top of
/// whatever `$3EEF0` did. The port used to save only while the node was still
/// below `$1E` at that point, which dropped the save for every zone node and
/// for the castle move.
#[test]
fn map_handoff_tail_saves_once_for_every_node_class() {
    let nodes = handoff_nodes();
    // Node 0 has slot 0: `$3EEF0` saves once, then `$3F03D` again.
    let mut zero = small_sim();
    assert_eq!(zero.map_handoff(&nodes).saves, 2);
    // A node with no progress slot in this fixture: the tail's save only —
    // whichever band it takes, including the band-less `$3F142` tail.
    for node in [0x1Eu8, 0x50, 0x78, 0x64, 0x7B] {
        let mut zone = small_sim();
        zone.map_node = node;
        assert_eq!(zone.map_handoff(&nodes).saves, 1, "node ${node:02X}");
    }
    // Node `$05` DOES have a slot here (`$1F`): `$3EEF0` + the tail.
    let mut castle = small_sim();
    castle.map_node = 0x05;
    assert_eq!(castle.map_handoff(&nodes).saves, 2);
}

/// The `$650E`/`$3C364` settlement, as a test rather than a note: the byte
/// `map_handoff` writes progress through is the SAME byte the map's entry gate
/// reads back, through `MapNodes::progress_slot`. `$3EEF0` (the writer) and
/// `$3E1D7` (the gate) both index `sLevelProgress` with `$650E[node]` directly
/// — and `$650E` is also the `$6591` key-mask index and the `$3C6xx` node-action
/// index, so it is one slot space. The only ROM site that inserts the `$3C364`
/// diff-table is `$3C32D`, the map's own auto-walk, whose table is
/// non-monotonic (`$3C364[$13] = $10 < $3C364[$12] = $13`) and therefore not a
/// re-indexing of the same space. If one of the three consumers is ever
/// switched to the other convention, this fails.
#[test]
fn the_progress_slot_is_one_convention_for_the_writer_and_the_gate() {
    let nodes = handoff_nodes();
    assert_eq!(nodes.progress_slot(0), Some(0), "node 0 is slot 0");
    let mut sim = small_sim();
    assert!(
        sim.map_can_enter(&nodes, 1, 0x40),
        "enterable while the stage is uncleared"
    );
    sim.map_handoff(&nodes);
    assert_eq!(
        sim.level_progress[0] & 0x80,
        0x80,
        "the writer's byte: sLevelProgress[slot]"
    );
    sim.map_node = 0;
    sim.map_handoff_done = false;
    assert!(
        !sim.map_can_enter(&nodes, 1, 0x40),
        "and the gate reads the same byte back"
    );
}

/// `$2934`'s record: the node at +0, `sLevelProgress` at +8, the five
/// counters, and a checksum that `$28EC` accepts.
#[test]
fn the_save_record_is_the_roms_image() {
    let mut sim = small_sim();
    sim.map_node = 0x14;
    sim.level_progress[0] = 0x80;
    sim.level_progress[1] = 0x08;
    sim.coins_low = 0x23;
    sim.coins_high = 0x01;
    sim.level_tally = 0x07;
    sim.lives = 0x04;

    let rec = sim.save_record(0x02);
    assert_eq!(rec.len(), 0x50);
    assert_eq!(rec[0x00], 0x14, "sMapNode");
    assert_eq!(rec[0x08], 0x80, "sLevelProgress[0]");
    assert_eq!(rec[0x09], 0x08);
    assert_eq!(rec[0x40], 0x23, "sCoinLow");
    assert_eq!(rec[0x41], 0x01, "sCoinHigh");
    assert_eq!(rec[0x42], 0x07, "sLevelTally");
    assert_eq!(rec[0x43], 0x04, "sLives");
    assert_eq!(rec[0x44], 0x02, "sMoneyBagCount, passed in");
    assert!(crate::map::save_record_valid(&rec), "$28EC accepts it");
    // The bytes the port does not model stay zero, golden coins included —
    // which is also what the ROM produces, since nothing sets their bit 7.
    assert!(rec[0x0D..0x26].iter().all(|b| *b == 0));
}

/// A level reload clears the port's own marker but leaves the node and the
/// persisted progress alone: both are progress, not level state.
#[test]
fn a_level_reload_keeps_the_map_progress() {
    let nodes = handoff_nodes();
    let mut sim = small_sim();
    sim.map_handoff(&nodes);
    assert_eq!(sim.map_node, 0x14);
    assert_eq!(sim.level_progress[0] & 0x80, 0x80);
    sim.respawn(112, 240);
    assert!(!sim.map_handoff_done, "the marker is a phase flag");
    assert_eq!(sim.map_node, 0x14, "the node survives the reload");
    assert_eq!(sim.level_progress[0] & 0x80, 0x80, "so does the progress");
}

// -----------------------------------------------------------------
// $3451 / $3534 — the item consumer (powerup transitions).
//
// The gate captures only ever reach id 7 (`traj13`/`traj14`'s `secret`
// check stages a `$87` object; the rest of the table is unexercised), so
// every transition is pinned here: the powerup grants + `$3534` timers,
// the BCD adds, the goal-bell tail, and — because the hazard/ceiling/
// side probes read `sCurPowerup` back — that a mushroom really moves
// the hitboxes.
// -----------------------------------------------------------------

/// Everything `$3451` can write, in one comparable snapshot (sfx0/sfx1,
/// powerup, the timers, the caps and the flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConsumerState {
    powerup: u8,
    invincibility: u8,
    lives: u8,
    coins_low: u8,
    coins_high: u8,
    star_high: u8,
    star_low: u8,
    moon: u8,
    secret_route: u8,
    goal_bell: bool,
    low_time_flag: u8,
    music_request: u8,
    sfx0: u8,
    sfx1: u8,
}

fn consumer_state(s: &PlayerSim) -> ConsumerState {
    ConsumerState {
        powerup: s.powerup,
        invincibility: s.invincibility,
        lives: s.lives,
        coins_low: s.coins_low,
        coins_high: s.coins_high,
        star_high: s.star_high,
        star_low: s.star_low,
        moon: s.moon,
        secret_route: s.secret_route,
        goal_bell: s.goal_bell,
        low_time_flag: s.low_time_flag,
        music_request: s.music_request,
        sfx0: s.sfx0,
        sfx1: s.sfx1,
    }
}

/// The engine's `and a / ret z`: with the byte already 0 nothing runs.
#[test]
fn item_id_zero_is_a_no_op() {
    let mut sim = PlayerSim::new();
    sim.sfx0 = 3;
    sim.sfx1 = 4;
    sim.low_time_flag = 0xFF;
    let before = consumer_state(&sim);
    sim.pending_item = 0;
    sim.item_consumer();
    assert_eq!(sim.pending_item, 0);
    assert_eq!(consumer_state(&sim), before);
}

/// Every id above the table falls straight to `$3539`: the byte clears
/// and nothing else changes.
#[test]
fn unmatched_item_ids_clear_without_effects() {
    for id in [10u8, 0x20, 0x7F] {
        let mut sim = PlayerSim::new();
        let before = consumer_state(&sim);
        sim.pending_item = id;
        sim.item_consumer();
        assert_eq!(sim.pending_item, 0, "id {id:#04X}");
        assert_eq!(consumer_state(&sim), before, "id {id:#04X}");
    }
}

/// `$3456` (id 1): mushroom — tier 1, `sSfxRequest1 = 20` (the engine
/// does *not* write sSfxRequest0 here), and `$3534`'s 64-frame
/// invincibility.
#[test]
fn mushroom_pickup_grants_tier1_with_the_3534_timer() {
    let mut sim = PlayerSim::new();
    sim.powerup = 0;
    sim.sfx0 = 3; // must survive: id 1 writes sSfxRequest1
    sim.pending_item = 1;
    sim.item_consumer();
    assert_eq!(sim.powerup, 1);
    assert_eq!(sim.invincibility, 64);
    assert_eq!(sim.sfx1, 20);
    assert_eq!(sim.sfx0, 3);
    assert_eq!(sim.pending_item, 0);
}

/// `$3467` (id 2): the fire flower is tier 3.
#[test]
fn fire_flower_pickup_grants_tier3() {
    let mut sim = PlayerSim::new();
    sim.powerup = 0;
    sim.pending_item = 2;
    sim.item_consumer();
    assert_eq!(sim.powerup, 3);
    assert_eq!(sim.invincibility, 64);
    assert_eq!(sim.sfx1, 20);
    assert_eq!(sim.sfx0, 0);
    assert_eq!(sim.pending_item, 0);
}

/// `$3478` (id 3): the carrot is the bunny tier 2.
#[test]
fn carrot_pickup_grants_tier2() {
    let mut sim = PlayerSim::new();
    sim.powerup = 0;
    sim.pending_item = 3;
    sim.item_consumer();
    assert_eq!(sim.powerup, 2);
    assert_eq!(sim.invincibility, 64);
    assert_eq!(sim.sfx1, 20);
    assert_eq!(sim.pending_item, 0);
}

/// `$3489` (id 4): timers 4/64 and `sMusicRequest = 4`; the star never
/// touches the invincibility blink.
#[test]
fn star_pickup_sets_the_timers_and_the_music_request() {
    let mut sim = PlayerSim::new();
    sim.invincibility = 128;
    sim.pending_item = 4;
    sim.item_consumer();
    assert_eq!((sim.star_high, sim.star_low), (4, 64));
    assert_eq!(sim.music_request, 4);
    assert_eq!(sim.invincibility, 128);
    assert_eq!(sim.pending_item, 0);
}

/// The star's `sStompChainCount` clear is conditional —
/// `(sStarTimerLow ^ $FF) | sStarTimerHigh == 0` — so only the exhausted
/// (high 0, low `$FF`) pair the `$5D39` decrement stops at resets the
/// chain. A timer that never ran (0, 0) or one still counting leaves it
/// alone.
#[test]
fn star_pickup_resets_the_stomp_chain_only_on_an_expired_timer() {
    for (high, low) in [
        (0u8, 0u8),
        (0, 1),
        (0, 0xFE),
        (1, 0xFF),
        (4, 0x40),
        (0, 0xFF),
    ] {
        let mut sim = PlayerSim::new();
        sim.star_high = high;
        sim.star_low = low;
        sim.pending_item = 4;
        sim.item_consumer();
        let expected = high == 0 && low == 0xFF;
        assert_eq!(
            sim.stomp_chain_reset, expected,
            "timer ({high}, {low:#04X})"
        );
        // The runtime drains the flag — it is a one-shot.
        assert_eq!(sim.take_stomp_chain_reset(), expected);
        assert!(!sim.take_stomp_chain_reset());
    }
}

/// `$5D44`: the 16-bit star countdown. From the pickup's `(4, 64)` it takes
/// 833 ticks to reach the `(0, $FF)` marker the decrement stops at, and the
/// borrow that clears the high byte restores `sLevelSong`.
#[test]
fn star_timer_counts_down_and_restores_the_level_song_on_expiry() {
    let mut sim = PlayerSim::new();
    sim.star_high = 4;
    sim.star_low = 64;
    sim.level_song = 5;
    sim.current_song = 5;
    sim.music_request = 0;
    sim.tick_star_timer();
    assert_eq!((sim.star_high, sim.star_low), (4, 63));
    assert_eq!(sim.music_request, 0, "no restore while the high byte runs");
    for _ in 1..64 {
        sim.tick_star_timer();
    }
    assert_eq!((sim.star_high, sim.star_low), (4, 0));
    sim.tick_star_timer();
    assert_eq!((sim.star_high, sim.star_low), (3, 0xFF), "$5D4E borrow");
    assert_eq!(sim.music_request, 0);
    let mut ticks = 65;
    while sim.star_high != 0 {
        sim.tick_star_timer();
        ticks += 1;
    }
    assert_eq!(ticks, 833);
    assert_eq!((sim.star_high, sim.star_low), (0, 0xFF));
    assert_eq!(sim.music_request, 5, "$5D5B restores the level song");
    // The stopped pair is inert: `$5D44` gates on the high byte.
    sim.music_request = 0;
    for _ in 0..8 {
        sim.tick_star_timer();
    }
    assert_eq!((sim.star_high, sim.star_low), (0, 0xFF));
    assert_eq!(sim.music_request, 0);
}

/// `$5D5B`: the restore is skipped while `sCurSong` is 9 (the low-time
/// warning), and a timer that never ran does not move at all.
#[test]
fn star_timer_restore_is_skipped_while_song_9_plays() {
    let mut sim = PlayerSim::new();
    sim.star_high = 1;
    sim.star_low = 0;
    sim.level_song = 5;
    sim.current_song = 9;
    sim.music_request = 0;
    sim.tick_star_timer();
    assert_eq!((sim.star_high, sim.star_low), (0, 0xFF));
    assert_eq!(sim.music_request, 0, "song 9 is left alone");

    let mut idle = PlayerSim::new();
    idle.level_song = 5;
    idle.current_song = 5;
    idle.tick_star_timer();
    assert_eq!((idle.star_high, idle.star_low), (0, 0));
    assert_eq!(idle.music_request, 0);
}

/// `$54C8`: `$FFC7` is the star countdown's low-byte bit 2, gated on a
/// running high byte.
#[test]
fn star_flash_is_bit_2_of_a_running_low_byte() {
    let mut sim = PlayerSim::new();
    for (high, low, want) in [
        (4u8, 0x40u8, false),
        (4, 0x44, true),
        (4, 0x04, true),
        (4, 0x03, false),
        (0, 0x44, false),
        (0, 0xFF, false),
    ] {
        sim.star_high = high;
        sim.star_low = low;
        assert_eq!(sim.star_flash(), want, "timer ({high}, {low:#04X})");
    }
}

/// Object engine `award_lives` transfers through `apply_object_result` and increments lives (BCD).
#[test]
fn award_lives_from_objects_increments_lives() {
    let mut sim = PlayerSim::new();
    sim.lives = 5;
    let mut obj = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj.award_lives = 1;
    sim.apply_object_result(&mut obj, false);
    assert_eq!(sim.lives, 6);
    assert_eq!(obj.award_lives, 0);

    // Multi-award
    obj.award_lives = 2;
    sim.apply_object_result(&mut obj, false);
    assert_eq!(sim.lives, 8);
    assert_eq!(obj.award_lives, 0);
}

/// `$34AF` (id 5): `inc a / daa` on the BCD lives byte, clamped at 99
/// when the increment wraps.
#[test]
fn one_up_heart_bcd_increments_lives_up_to_99() {
    for (before, after) in [
        (0x00u8, 0x01u8),
        (0x05, 0x06),
        (0x09, 0x10),
        (0x98, 0x99),
        (0x99, 0x99),
    ] {
        let mut sim = PlayerSim::new();
        sim.lives = before;
        sim.pending_item = 5;
        sim.item_consumer();
        assert_eq!(sim.lives, after, "lives {before:#04X} + 1");
        assert_eq!(sim.sfx0, 9);
        assert_eq!(sim.pending_item, 0);
    }
}

/// `$34EC` (id 6): sfx0 12, `sGoalBell` set (the port's bool is compared
/// as 12), the low-time latch cleared, and vvel lifted to 24 only when
/// it was below `$18` — `$18` and above stay put.
#[test]
fn bell_pickup_clears_the_low_time_latch_and_lifts_vvel_to_24() {
    for (before, after) in [
        (0u8, 24u8),
        (0x17, 24),
        (0x18, 0x18),
        (0x33, 0x33),
        (0xFF, 0xFF),
    ] {
        let mut sim = PlayerSim::new();
        sim.vvel = before;
        sim.low_time_flag = 0xFF;
        sim.pending_item = 6;
        sim.item_consumer();
        assert_eq!(sim.vvel, after, "vvel {before:#04X}");
        assert!(sim.goal_bell);
        assert_eq!(sim.low_time_flag, 0);
        assert_eq!(sim.sfx0, 12);
        assert_eq!(sim.pending_item, 0);
    }
}

/// `$34C9` (id 7): the secret-route flag — the one transition the gate
/// captures actually replay (traj13/traj14's `secret` check).
#[test]
fn secret_route_pickup_sets_the_flag() {
    let mut sim = PlayerSim::new();
    sim.pending_item = 7;
    sim.item_consumer();
    assert_eq!(sim.secret_route, 0xFF);
    assert_eq!(sim.sfx0, 21);
    assert_eq!(sim.pending_item, 0);
}

/// `$34D9` (id 8): `set 2` on `sMoonPhysics` — bit 2 only, other bits
/// kept, and idempotent on a repeat pickup.
#[test]
fn moon_pickup_sets_the_physics_bit_and_keeps_the_rest() {
    let mut sim = PlayerSim::new();
    sim.moon = 0x03;
    sim.pending_item = 8;
    sim.item_consumer();
    assert_eq!(sim.moon, 0x07);
    assert_eq!(sim.sfx1, 20);
    assert_eq!(sim.sfx0, 0);
    assert_eq!(sim.pending_item, 0);
    sim.pending_item = 8;
    sim.item_consumer();
    assert_eq!(sim.moon, 0x07);
}

// -----------------------------------------------------------------
// $0FF2 — the moon jump extension (`space_physics_seeding`)
// -----------------------------------------------------------------

/// The seed is the raw BITFIELD byte, not a bool: `LevelPropertiesSpace`
/// (`$1F91`) → `sMoonPhysics` ($A287), whole. Level 19 is `$08`, which a
/// bool fold would turn into `$01` — and the manifest is the port's only
/// source for it (the replay path must never take it from the record).
#[test]
fn the_moon_physics_seed_copies_the_raw_bitfield_byte() {
    let mut sim = PlayerSim::new();
    assert_eq!(sim.moon, 0, "the default the suite has been running on");
    for byte in [0x08u8, 0x01, 0x08, 0x00] {
        sim.seed_moon_physics(byte);
        assert_eq!(sim.moon, byte, "byte ${byte:02X} copied whole");
    }
}

/// `$0FF2` reaches the per-frame entry. Level 20's byte (`$01`, bit 0) makes
/// an airborne Mario holding A gain 4 vvel *before* the frame's own −1
/// gravity, so the net after `step` is +3, and the press edge asks sfx 18.
/// The control is the same frame with the default byte: the arm is dead and
/// only the gravity moves vvel — which is how every level but 19/20/29 runs.
#[test]
fn the_bit0_float_runs_inside_the_frame_and_the_default_byte_kills_it() {
    let layout = blank_layout();
    let keys = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let mut sim = small_sim();
    sim.seed_moon_physics(0x01);
    sim.on_ground = false;
    sim.vvel = 40;
    sim.step(&keys, true, false, &layout);
    assert_eq!(sim.vvel, 43, "$108A's +4 then the frame's own -1");
    assert_eq!(sim.sfx0, 18, "$107F: the A press edge");
    let mut plain = small_sim();
    plain.on_ground = false;
    plain.vvel = 40;
    plain.step(&keys, true, false, &layout);
    assert_eq!(plain.vvel, 39, "no space physics: gravity alone");
    assert_eq!(plain.sfx0, 0);
}

/// `$106F` — level 20's hold-A float: airborne + A *held* → `vvel += 4`,
/// capped at `$48`; grounded returns first (`ret nz`), A released returns
/// (`ret z`), and only the press edge requests sfx 18.
#[test]
fn the_bit0_float_caps_at_48_and_needs_a_held() {
    let held = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let mut sim = PlayerSim::new();
    sim.moon = 0x01;
    sim.on_ground = false;
    sim.vvel = 0x47;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.vvel, 0x4B, "47 + 4");
    sim.vvel = 0x48;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.vvel, 0x48, "the cap: cp $48 / ret nc");
    sim.vvel = 40;
    sim.sfx0 = 0;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.vvel, 44, "40 + 4");
    assert_eq!(sim.sfx0, 0, "a held is not a press — no sfx");
    sim.vvel = 40;
    sim.moon_jump_extension(&PlayerKeys::default(), false);
    assert_eq!(sim.vvel, 40, "A not held: ret z");
    sim.on_ground = true;
    sim.moon_jump_extension(&held, true);
    assert_eq!(sim.vvel, 40, "grounded: ret nz");
    assert_eq!(sim.sfx0, 0, "and no sfx on the ground");
}

/// `$1029` — the bit-3 timed float of the two `$08` levels: armed to 50
/// frames on the ground (`$104A`), dropped when A is released (`$1050`),
/// one frame of budget spent per float frame, `vvel += 2` capped at `$4A`.
#[test]
fn the_bit3_timed_float_arms_on_the_ground_and_spends_its_budget() {
    let held = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let mut sim = PlayerSim::new();
    sim.moon = 0x08;
    sim.on_ground = true;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.moon_float_timer, 50, "$104A");
    assert_eq!(sim.vvel, 24, "the ground arm never touches vvel");
    sim.on_ground = false;
    sim.vvel = 40;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.moon_float_timer, 49, "one frame spent");
    assert_eq!(sim.vvel, 42, "$1046's +2");
    sim.vvel = 0x4A;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.vvel, 0x4A, "cp $4A / ret nc");
    assert_eq!(sim.moon_float_timer, 48, "the budget is spent anyway");
    sim.moon_float_timer = 0;
    sim.vvel = 40;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.vvel, 40, "an empty budget: and a / ret z");
    sim.moon_float_timer = 20;
    sim.moon_jump_extension(&PlayerKeys::default(), false);
    assert_eq!(sim.moon_float_timer, 0, "$1050: A released");
    assert_eq!(sim.vvel, 40);
}

/// `$1055` — inside a moon zone (bit 1) an A *press* sets vvel 52 and arms
/// the 8-frame pose timer once; without a press the whole arm returns.
#[test]
fn the_bit1_zone_relaunch_fires_on_the_press_edge_only() {
    let held = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let mut sim = PlayerSim::new();
    sim.moon = 0x02;
    sim.on_ground = false;
    sim.vvel = 10;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.vvel, 10, "hKeysPressed bit 0 clear: ret z");
    assert_eq!(sim.moon_jump_pose_timer, 0);
    sim.moon_jump_extension(&held, true);
    assert_eq!(sim.vvel, 52, "$105C");
    assert_eq!(sim.moon_jump_pose_timer, 8, "$1066");
    assert_eq!(sim.sfx0, 3);
    sim.vvel = 10;
    sim.moon_jump_extension(&held, true);
    assert_eq!(sim.vvel, 52, "a second press re-launches");
    assert_eq!(
        sim.moon_jump_pose_timer, 8,
        "but the pose timer is one-shot"
    );
}

/// `$1008` — the bit-2-only arm (the moon item is held, not in a zone): the
/// same +2/cap-$4A/sfx-18 shape with no timer, and its own `ret` means it
/// never spends the `$08` arm's float budget.
#[test]
fn the_bit2_only_arm_adds_2_without_touching_the_float_budget() {
    let held = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let mut sim = PlayerSim::new();
    sim.moon = 0x04;
    sim.on_ground = false;
    sim.moon_float_timer = 30;
    sim.vvel = 40;
    sim.moon_jump_extension(&held, true);
    assert_eq!(sim.vvel, 42, "$1025's +2");
    assert_eq!(
        sim.moon_float_timer, 30,
        "the budget belongs to the $08 arm"
    );
    assert_eq!(sim.sfx0, 18, "$1018");
    sim.vvel = 0x4A;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.vvel, 0x4A, "cp $4A / ret nc");
    sim.on_ground = true;
    sim.vvel = 40;
    sim.moon_jump_extension(&held, false);
    assert_eq!(sim.vvel, 40, "grounded: ret nz");
}

/// `$0FF2`'s two head guards: a ringing goal bell returns before anything,
/// and `moon == 0` makes the whole routine a no-op — every level but the
/// three whose `LevelPropertiesSpace` byte is nonzero.
#[test]
fn the_moon_extension_returns_on_a_goal_bell_and_on_a_plain_level() {
    let held = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let mut sim = PlayerSim::new();
    sim.moon = 0x01;
    sim.on_ground = false;
    sim.vvel = 40;
    sim.goal_bell = true;
    sim.moon_jump_extension(&held, true);
    assert_eq!(sim.vvel, 40, "sGoalBell: ret nz");
    assert_eq!(sim.sfx0, 0);
    sim.goal_bell = false;
    sim.moon = 0x00;
    sim.moon_jump_extension(&held, true);
    assert_eq!(sim.vvel, 40, "moon == 0: ret z");
    assert_eq!(sim.sfx0, 0);
}

/// `$1CFB`–`$1D71`: entering the bit-1 moon zone via tiles `$52..=$57`.
/// Sets bit 1, clears bit 2 (the 3-up moon shield), clears flutter_armed,
/// and clamps horizontal speed to [72, 191] on transition.
#[test]
fn moon_zone_entry_and_hvel_clamp() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x54; // inside $52..=$57
    let mut sim = small_sim();
    sim.moon = 0x04; // moon shield armed
    sim.flutter_armed = true;
    sim.flutter_charge = 4;
    sim.hvel = 0xC5; // > 191 -> clamped to 191
    sim.vvel = 24;
    sim.in_air = false;

    sim.environment_probe(&layout, &PlayerKeys::default());

    assert_eq!(sim.moon & 0x02, 0x02, "bit 1 set on entering moon zone");
    assert_eq!(
        sim.moon & 0x04,
        0,
        "bit 2 moon shield cleared on entering moon zone"
    );
    assert!(
        !sim.flutter_armed,
        "flutter_armed cleared on entering moon zone"
    );
    assert_eq!(sim.flutter_charge, 4, "$1CFB never writes sFlutterCharge");
    assert_eq!(sim.hvel, 191, "hvel clamped from 0xC5 to 191 ($BF)");
    assert_eq!(sim.vvel, 24, "entry transition does not alter vvel");
    assert!(!sim.in_air, "entry transition does not set in_air");

    // Entering with low hvel (< 72) clamps to 72:
    let mut sim_slow = small_sim();
    sim_slow.hvel = 50;
    sim_slow.environment_probe(&layout, &PlayerKeys::default());
    assert_eq!(sim_slow.hvel, 72, "hvel clamped from 50 to 72 ($48)");
}

/// `$1CFB`: remaining inside the moon zone does not re-trigger transition clamps.
#[test]
fn moon_zone_stay_does_not_retrigger_transition() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x52;
    let mut sim = small_sim();
    sim.moon = 0x02; // already in moon zone
    sim.hvel = 200;
    sim.environment_probe(&layout, &PlayerKeys::default());
    assert_eq!(
        sim.hvel, 200,
        "no state change on bit 1 means no transition clamp"
    );
}

/// `$1D48`–`$1D71`: exiting the bit-1 moon zone launches Mario with vvel 70 ($46),
/// sets in_air = true, and latches no_jump_cut if UP was held.
#[test]
fn moon_zone_exit_launches_mario_and_handles_up_key() {
    let layout = blank_layout(); // cell is blank air ($60), not $52..=$57
    let mut sim = small_sim();
    sim.moon = 0x02; // leaving moon zone
    sim.hvel = 0xC8; // 200
    sim.vvel = 10;
    sim.in_air = false;
    let keys_no_up = PlayerKeys::default();

    sim.environment_probe(&layout, &keys_no_up);

    assert_eq!(sim.moon & 0x02, 0, "bit 1 cleared on exit");
    assert_eq!(sim.hvel, 191, "exit transition clamps hvel");
    assert_eq!(sim.vvel, 70, "$1D50: vvel set to 70 on exit");
    assert!(sim.in_air, "in_air set to true on exit");
    assert!(!sim.no_jump_cut, "no_jump_cut remains false without UP");

    // Exiting with UP held:
    let mut sim_up = small_sim();
    sim_up.moon = 0x02;
    let keys_up = PlayerKeys {
        up: true,
        ..Default::default()
    };
    sim_up.environment_probe(&layout, &keys_up);
    assert_eq!(sim_up.vvel, 70);
    assert!(sim_up.in_air);
    assert!(
        sim_up.no_jump_cut,
        "$1D64: UP sets no_jump_cut on moon zone exit"
    );
}

/// `$1C82`/`$1CAA`: water tile clears moon bit 1 directly and jumps to $1D71
/// without running the exit-launch or hvel clamping.
#[test]
fn water_tile_clears_moon_bit1_and_bypasses_transition() {
    let mut layout = blank_layout();
    layout.cells[EXIT_CELL] = 0x4C; // water
    let mut sim = small_sim();
    sim.moon = 0x02;
    sim.hvel = 200;
    sim.vvel = 10;

    sim.environment_probe(&layout, &PlayerKeys::default());

    assert!(sim.swim);
    assert_eq!(sim.moon & 0x02, 0, "water clears bit 1");
    assert_eq!(sim.hvel, 200, "bypassed $1CFB, so hvel was not clamped");
    assert_eq!(sim.vvel, 10, "bypassed $1CFB, so vvel was not set to 70");
}

/// `$14FA`: flutter timer is bypassed when in a bit-1 moon zone.
/// Outside moon zone, flutter timer decrements and returns before gravity runs.
#[test]
fn flutter_timer_vs_moon_physics() {
    let layout = blank_layout();
    // Outside moon zone: flutter timer active decrements and skips gravity
    let mut sim_flutter = small_sim();
    sim_flutter.moon = 0;
    sim_flutter.on_ground = false;
    sim_flutter.flutter = 5;
    sim_flutter.vvel = 10;
    sim_flutter.y = 100;
    sim_flutter.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(sim_flutter.flutter, 4, "flutter decremented by 1");
    assert_eq!(sim_flutter.y, 100, "gravity was bypassed");

    // Inside bit-1 moon zone: all tiles $54, flutter timer is ignored and gravity runs
    let moon_layout = LevelLayout::from_bytes(&[0x54; LAYOUT_SIZE]).unwrap();
    let mut sim_moon = small_sim();
    sim_moon.moon = 0x02;
    sim_moon.on_ground = false;
    sim_moon.flutter = 5;
    sim_moon.vvel = 10;
    sim_moon.y = 100;
    sim_moon.step(&PlayerKeys::default(), false, false, &moon_layout);
    assert_eq!(
        sim_moon.flutter, 5,
        "flutter timer bypassed under moon physics bit 1"
    );
    assert_ne!(sim_moon.y, 100, "gravity ran");
}

/// `$10DC`: an airborne A press as Bunny Mario starts the flutter, which
/// requests the ear-flap sound on channel 1; the held-A recharge does too.
#[test]
fn bunny_flutter_requests_its_sound() {
    let layout = blank_layout();
    let held = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let mut sim = small_sim();
    sim.powerup = 2;
    sim.on_ground = false;
    sim.in_air = true;
    sim.vvel = 10;
    sim.step(&held, true, false, &layout);
    assert!(sim.flutter_armed);
    assert_eq!(sim.flutter, 7, "set to 8, then $14FA's decrement");
    assert_eq!(sim.sfx1, 6, "the press arm");

    sim.sfx1 = 0;
    sim.flutter_charge = 11;
    sim.vvel = 10;
    sim.step(&held, false, false, &layout);
    assert_eq!(sim.flutter_charge, 0);
    assert_eq!(sim.sfx1, 6, "the recharge arm");
}

/// `$547A`: Bunny Mario shows the ears-up pose (+26) while the flutter
/// timer's bit 2 is set.
#[test]
fn bunny_flutter_pose_follows_timer_bit_2() {
    let mut sim = small_sim();
    sim.powerup = 2;
    sim.on_ground = false;
    sim.in_air = true;
    sim.dir_right = true;
    for (timer, want) in [(0, 8 + 69), (7, 8 + 69 + 26), (4, 8 + 69 + 26), (3, 8 + 69)] {
        sim.flutter = timer;
        assert_eq!(sim.frame_index(0), want, "flutter timer {timer}");
    }
}

/// `$14D5`: walk animation cadence.
/// Under moon physics (or goal bell), cadence is always +1 even with B held.
#[test]
fn walk_anim_cadence_under_moon_physics() {
    let layout = blank_layout();
    let keys_b = PlayerKeys {
        b: true,
        ..Default::default()
    };

    // Normal Mario with B: advances by 2
    let mut sim_normal = small_sim();
    sim_normal.walk_timer = 0;
    sim_normal.step(&keys_b, false, false, &layout);
    assert_eq!(sim_normal.walk_timer, 2);

    // Moon Mario with B: advances by 1
    let mut sim_moon = small_sim();
    sim_moon.moon = 0x01;
    sim_moon.walk_timer = 0;
    sim_moon.step(&keys_b, false, false, &layout);
    assert_eq!(sim_moon.walk_timer, 1);

    // Goal bell with B: advances by 1
    let mut sim_bell = small_sim();
    sim_bell.goal_bell = true;
    sim_bell.walk_timer = 0;
    sim_bell.step(&keys_b, false, false, &layout);
    assert_eq!(sim_bell.walk_timer, 1);
}

/// `$534C` / `$53CF`: airborne poses in bit-1 moon zone.
/// Big Mario (powerup = 1):
/// Right: 18 normally, 19 while moon_jump_pose_timer is active (and decrements).
/// Left: 20 normally, 21 while moon_jump_pose_timer is active (and decrements).
/// Small Mario (powerup = 0):
/// Adds +32 powerup offset: 50/51 right, 52/53 left.
#[test]
fn bank1_moon_jump_poses() {
    let mut sim = small_sim();
    sim.powerup = 1; // big Mario (offset 0)
    sim.moon = 0x02;
    sim.on_ground = false;

    // Facing right
    sim.dir_right = true;
    sim.moon_jump_pose_timer = 8;
    assert_eq!(sim.frame_index(0), 19);
    assert_eq!(sim.moon_jump_pose_timer, 7);
    sim.moon_jump_pose_timer = 0;
    assert_eq!(sim.frame_index(0), 18);
    assert_eq!(sim.moon_jump_pose_timer, 0);

    // Facing left
    sim.dir_right = false;
    sim.moon_jump_pose_timer = 8;
    assert_eq!(sim.frame_index(0), 21);
    assert_eq!(sim.moon_jump_pose_timer, 7);
    sim.moon_jump_pose_timer = 0;
    assert_eq!(sim.frame_index(0), 20);
    assert_eq!(sim.moon_jump_pose_timer, 0);

    // Small Mario gets +32 offset
    sim.powerup = 0;
    sim.dir_right = true;
    sim.moon_jump_pose_timer = 8;
    assert_eq!(sim.frame_index(0), 51); // 19 + 32
    sim.moon_jump_pose_timer = 0;
    assert_eq!(sim.frame_index(0), 50); // 18 + 32
}

/// `$54FB`: 3-up moon shield satellite frame alternating 220/221 on a 16-frame cycle.
/// `$2E7C`: object hit consumes bit 2 and grants invincibility.
#[test]
fn moon_shield_frame_and_consumption() {
    let mut sim = small_sim();
    sim.moon = 0x04; // bit 2 set
    sim.frame_counter = 0x00;
    assert_eq!(sim.moon_shield_frame(), Some(220));
    sim.frame_counter = 0x10;
    assert_eq!(sim.moon_shield_frame(), Some(221));

    sim.moon = 0x00;
    assert_eq!(sim.moon_shield_frame(), None);

    // $2E7C consumption through apply_object_result
    let mut sim_hit = small_sim();
    sim_hit.moon = 0x04;
    let mut obj = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj.mario.moon_physics = 0; // shield consumed by interaction.rs:366
    obj.mario.invincibility = 0x80;
    sim_hit.apply_object_result(&mut obj, false);
    assert_eq!(sim_hit.moon & 0x04, 0, "shield bit consumed");
    assert_eq!(sim_hit.invincibility, 0x80, "invincibility granted");
    assert_eq!(sim_hit.dead, 0, "Mario survives hit");
}

/// `$350A` (id 9): `add $50`/`daa` — fifty coins in BCD, the decimal
/// carry into sCoinHigh, and the `cp $0A` clamp to 999.
#[test]
fn coin_bag_adds_50_in_bcd_with_carry_and_caps_at_999() {
    for ((hi, lo), (want_hi, want_lo)) in [
        ((0x00u8, 0x00u8), (0x00u8, 0x50u8)),
        ((0x00, 0x45), (0x00, 0x95)), // 45 + 50 = 95: no carry
        ((0x00, 0x49), (0x00, 0x99)), // 49 + 50 = 99: still no carry
        ((0x00, 0x50), (0x01, 0x00)), // 50 + 50 = 100: carry
        ((0x00, 0x99), (0x01, 0x49)), // 99 + 50 = 149
        ((0x08, 0x99), (0x09, 0x49)), // 899 + 50 = 949: high 9, no clamp
        ((0x09, 0x50), (0x09, 0x99)), // 950 + 50 = 1000 -> 999
        ((0x09, 0x99), (0x09, 0x99)), // already 999
    ] {
        let mut sim = PlayerSim::new();
        sim.coins_high = hi;
        sim.coins_low = lo;
        sim.pending_item = 9;
        sim.item_consumer();
        assert_eq!(
            (sim.coins_high, sim.coins_low),
            (want_hi, want_lo),
            "coins {hi:#04X}{lo:02X} + 50"
        );
        assert_eq!(sim.sfx0, 7);
        assert_eq!(sim.pending_item, 0);
    }
}

/// `step` runs the consumer at the update tail: the id staged by the
/// *previous* frame's object pass lands and the byte clears.
#[test]
fn step_consumes_the_staged_item_at_the_update_tail() {
    let layout = blank_layout();
    let keys = PlayerKeys::default();
    let mut sim = small_sim();
    sim.pending_item = 7;
    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.secret_route, 0xFF);
    assert_eq!(sim.sfx0, 21);
    assert_eq!(sim.pending_item, 0);
}

/// The tail order is `call $319F / call $3451`: a powerup-loss dispatch
/// (tier down, 128-frame blink) is overwritten by the item the same
/// frame — the mushroom wins, with the item's 64-frame timer.
#[test]
fn the_item_consumer_runs_after_the_death_dispatch() {
    let layout = blank_layout();
    let keys = PlayerKeys::default();
    let mut sim = PlayerSim::new(); // big
    sim.dead = 15; // a harmful contact this frame
    sim.pending_item = 1;
    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.dead, 0); // $319F consumed the hit...
    assert_eq!(sim.powerup, 1); // ...and $3451's mushroom re-granted tier 1
    assert_eq!(sim.invincibility, 64); // $3534's 64, not the dispatch's 128
}

/// Travelling replaces the whole update (`$0FBB` calls `$10EC` and
/// returns), so the tail — and with it the consumer — never runs: the
/// staged id waits for the exit.
#[test]
fn a_travelling_frame_defers_the_pending_item() {
    let layout = blank_layout();
    let keys = PlayerKeys::default();
    let mut sim = small_sim();
    sim.pipe_dir = 0x10;
    sim.pending_item = 7;
    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.pending_item, 7);
    assert_eq!(sim.secret_route, 0);
}

/// `$1DFE`'s probe height is `sCurPowerup`-dependent (`$1E17`: y+18 when
/// nonzero, y+28 at 0) — a mushroom moves the ceiling sample up one row.
/// (116, 218) is row 12 / col 6; (116, 228) is row 13 / col 6.
#[test]
fn mushroom_raises_the_ceiling_probe_from_y28_to_y18() {
    let mut layout = blank_layout();
    layout.cells[12 * 256 + 6] = 0x10; // solid only at the big-Mario sample
    let mut sim = small_sim(); // powerup 0
    assert!(!sim.ceiling_hit(&layout));
    assert_eq!(sim.probe_point, (116, 228));
    sim.pending_item = 1;
    sim.item_consumer();
    assert_eq!(sim.powerup, 1);
    assert!(sim.ceiling_hit(&layout));
    assert_eq!(sim.probe_point, (116, 218));
}

/// `$1E9C`: the y+20 side sample only exists while `sCurPowerup != 0`
/// (and Mario is not ducking) — small Mario walks over a coin there, big
/// Mario collects it. (120, 220) is row 12 / col 7.
#[test]
fn mushroom_enables_the_y20_wall_probe_and_its_coin_touch() {
    let mut layout = blank_layout();
    layout.cells[12 * 256 + 7] = 0x48; // a coin at the big-only sample
    let mut sim = small_sim();
    assert_eq!(sim.wall_tile(&layout, 120), None);
    assert_eq!(sim.coins_low, 0);
    assert!(sim.layout_writes.is_empty());
    sim.pending_item = 3; // the carrot raises the same probe
    sim.item_consumer();
    assert_eq!(sim.powerup, 2);
    assert_eq!(sim.wall_tile(&layout, 120), None); // a coin is not solid
    assert_eq!(sim.coins_low, 0x01); // but it is now collected
    assert_eq!(sim.layout_writes, vec![(cell_index(12, 7), 0x60)]);
}

/// The same y+20 sample becomes a *wall* once the powerup is nonzero:
/// small Mario's shorter hitbox passes over the solid cell, tier 1 stops.
#[test]
fn mushroom_turns_the_y20_sample_into_a_wall() {
    let mut layout = blank_layout();
    layout.cells[12 * 256 + 7] = 0x10;
    let mut sim = small_sim();
    assert_eq!(sim.wall_tile(&layout, 120), None);
    sim.pending_item = 1;
    sim.item_consumer();
    assert_eq!(sim.wall_tile(&layout, 120), Some(0x10));
}

/// `$1C39`: the big-Mario hazard probe moves up to (x+16, y+22), so a
/// `$50` only the tall hitbox reads kills after the powerup — and a
/// fresh powerup grants the 64-frame blink the `$1C6D` guard reads.
/// (116, 222) is row 12 / col 6.
#[test]
fn mushroom_raises_the_hazard_probe_from_y28_to_y22() {
    let mut layout = blank_layout();
    layout.cells[12 * 256 + 6] = 0x50; // a spike at the big-only sample
    let mut sim = small_sim();
    assert_eq!(sim.hazard_probe(&layout), (false, None));
    sim.pending_item = 1;
    sim.item_consumer();
    assert_eq!(sim.powerup, 1);
    // The arm now hits the big-Mario sample — but the fresh mushroom's
    // 64-frame blink is exactly the `$1C6D` guard: it enters and writes
    // no death byte.
    assert_eq!(sim.hazard_probe(&layout), (true, None));
    sim.invincibility = 0; // once the blink lapses the spike kills
    assert_eq!(sim.hazard_probe(&layout), (true, Some(15)));
}

// -----------------------------------------------------------------
// $2F72 / $19D2 — the below-hit pass's two halves
// -----------------------------------------------------------------

/// `$01` bonk: the spin flag, the sfx and the type-3 bump slot land
/// immediately, but the coin waits on the object side's `$2F72` — the
/// alloc arms the pass with the probe's low bytes and `resolve_bump_coin`
/// is the `$19D2` tail that credits it once the special flag is known.
#[test]
fn the_01_bonk_defers_its_coin_to_the_below_hit_pass() {
    let mut sim = PlayerSim::new();
    sim.probe_point = (360, 479); // (px, py): the traj15 $01 block
    sim.block_bonk(0x01);
    assert_eq!(sim.spin_jump_bounce, 0xFF);
    assert_eq!(sim.sfx0, 2);
    assert_eq!(sim.block_slots[0][0], 3, "the $5B06 bump slot");
    assert_eq!(sim.coins_low, 0x00, "the coin is deferred");
    assert!(sim.bump_coin_pending);
    assert_eq!(sim.below_bonk, Some((0xDF, 0x68))); // 479, 360 low bytes
    sim.resolve_bump_coin(false);
    assert_eq!(sim.coins_low, 0x01);
    assert_eq!(sim.sfx0, 7, "$591F's credit sfx");
    assert_eq!(sim.block_slots[1][0], 4, "the coin-pop slot");
    assert_eq!(sim.object_bump_special, 0);
    assert!(!sim.bump_coin_pending);
}

/// The pass's `sObjectBumpSpecial` (a type-`$80` pose inside the bonk
/// window) withholds the coin *and* the type-4 slot — the ROM's
/// `and a / ret nz` ahead of `$2C77`.
#[test]
fn a_pose_in_the_bonk_window_withholds_the_coin() {
    let mut sim = PlayerSim::new();
    sim.probe_point = (360, 479);
    sim.block_bonk(0x01);
    sim.resolve_bump_coin(true);
    assert_eq!(sim.coins_low, 0x00, "no credit");
    assert_eq!(sim.sfx0, 2, "and no $591F sfx");
    assert_eq!(sim.block_slots[0][0], 3, "the bump itself still ran");
    assert_eq!(sim.block_slots[1][0], 0, "no coin slot");
    assert_eq!(sim.object_bump_special, 0xFF);
    assert!(!sim.bump_coin_pending);
}

/// Only the `$01` bonk owes a coin: the other classes arm the pass but
/// `resolve_bump_coin` is a plain no-op for them, and a bump whose
/// `$5960` walk found no free slot never runs `$2F72` at all.
#[test]
fn other_bump_classes_owe_no_coin_and_a_full_table_arms_nothing() {
    let mut sim = PlayerSim::new();
    sim.probe_point = (360, 479);
    sim.block_bonk(0x7E); // type 3, no coin
    assert_eq!(sim.below_bonk, Some((0xDF, 0x68)));
    assert!(!sim.bump_coin_pending);
    sim.resolve_bump_coin(false);
    assert_eq!(sim.coins_low, 0);
    assert_eq!(sim.block_slots[0][0], 3);
    // The 8-slot table full: the alloc's `cp $00 / ret z` bails before
    // the tail, so nothing is armed.
    let mut sim = PlayerSim::new();
    sim.probe_point = (360, 479);
    for s in sim.block_slots.iter_mut() {
        s[0] = 1;
    }
    sim.block_bonk(0x02);
    assert_eq!(sim.below_bonk, None);
    assert!(!sim.bump_coin_pending);
}

// -----------------------------------------------------------------
// $5960/$5997 — the bump slots' per-type lifecycles (`bonk_*` items)
// -----------------------------------------------------------------

/// The three offset tables against the ROM bytes re-read here (a byte dump of
/// `baserom.gb`, not the port's own constants), so a hand-edit of an array
/// literal cannot silently move a sprite. `$5A4A` (40 entries, the type-1/5
/// item pop) and `$5AE0` (25, the type-4 coin pop) are indexed by the timer —
/// the coin's by `timer >> 1` — and `$5BB4` (8) by the timer for the
/// type-2/3/6 rattle.
#[test]
fn the_bump_offset_tables_are_the_rom_bytes() {
    fn rom_hex(s: &str) -> Vec<i8> {
        s.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).expect("hex byte") as i8)
            .collect()
    }
    let pop = rom_hex(
        "FC F8 F4 F0 ED EA E7 E5 E3 E2 E0 DF DF DF E0 E2 E3 E5 E7 EA ED F0 F4 F8 \
         FC 00 08 0C 10 15 1A 1F 25 2B 32 39 3F 46 4E 50",
    );
    let coin =
        rom_hex("F4 F0 EC E8 E5 E2 DF DD DC DA D8 D7 D7 D7 D8 DA DB DD DF E2 E5 E8 EC F0 F4");
    let bob = rom_hex("00 00 FE FC FA FA FC FE");
    assert_eq!(ITEM_POP_OFFSETS_5A4A.to_vec(), pop, "$5A4A");
    assert_eq!(COIN_OFFSETS_5AE0.to_vec(), coin, "$5AE0");
    assert_eq!(BOB_OFFSETS_5BB4.to_vec(), bob, "$5BB4");
    assert_eq!(ITEM_POP_OFFSETS_5A4A.len(), 40, "the table's own extent");
    assert_eq!(COIN_OFFSETS_5AE0.len(), 25);
}

/// `$19B0`'s big arm (`bonk_big_mario_types_1_5`): the `$00` brick is
/// DESTROYED — `sSfxRequest2` (the destruction sound) *and* the shared
/// rattle sfx — into a type-1 slot whose `$5CF1` write turns the cell into
/// the `$60` empty metatile at timer 2 and again when the slot frees at
/// `$28`. Between those, the powerup pops out as the `$5A07` pair: 101 first,
/// then 102, so 102 is what `$FFC6` holds (`3E 65 E0 C6` then `3E 66 E0 C6`),
/// both at `slotY - sScrollY + $5A4A[timer]` and `slotX - sScrollX ∓ timer`
/// (the second `[$A23F]` read in that routine is `sBlockAnimTimer`, i.e. the
/// X pair is ∓ the TIMER, not ∓ the offset).
#[test]
fn big_mario_destroys_the_brick_and_pops_the_item_out() {
    let mut sim = PlayerSim::new();
    sim.powerup = 1;
    sim.probe_point = (360, 479);
    sim.block_bonk(0x00);
    assert_eq!(
        sim.sfx2, 1,
        "$19B0's big arm requests the destruction sound"
    );
    assert_eq!(sim.sfx0, 2, "the shared tail's rattle sfx still fires");
    assert_eq!(sim.spin_jump_bounce, 0xFF);
    assert_eq!(
        &sim.block_slots[0][..6],
        &[1, 1, 0xD8, 0x01, 0x70, 0x01],
        "type 1, timer 1, the probe's cell (479, 360) rounded"
    );
    // Timer 2: `$5A72` writes 96 (`$60`) into the CELL; no draw.
    sim.block_update();
    assert_eq!(sim.block_slots[0][1], 2);
    assert_eq!(sim.block_frame, None);
    assert!(sim.block_draw.is_empty());
    assert_eq!(sim.layout_writes, vec![(7190, 0x60)], "row 28 / col 22");
    // Timer 3: the item-pop pair. `$5A4A[3] = -16`; the Y is `slotY + off -
    // 26` (the port's render anchor absorbs the engine's `- sScrollY`) and
    // the X is `slotX - 16 ∓ 3`.
    sim.block_update();
    assert_eq!(sim.block_frame, Some(102), "the second draw leaves 102");
    assert_eq!(
        sim.block_draw,
        vec![(349, 430, 101), (355, 430, 102)],
        "368 - 16 ∓ 3, 472 - 16 - 26"
    );
    // The write repeats and the slot frees at `$28`.
    while sim.block_slots[0][0] != 0 {
        sim.block_update();
    }
    assert_eq!(sim.block_slots[0][1], 0x28, "the item pop lasts 40 ticks");
    assert_eq!(sim.layout_writes, vec![(7190, 0x60)], "one cell, twice");
}

/// `$19C1`: the `$03` bounce block splits on Mario's size exactly like `$00`
/// — small → type 6 (cell 24 at timer 2, the `$03` used tile `$07` at 8), big
/// → type 5 (destroyed: sfx2, the `$7F` used tile 127 at timer 2 and again at
/// `$28`). The `$00` small case (type 2, cell 24 then 0) is the control.
#[test]
fn the_03_block_splits_into_type_6_for_small_and_type_5_for_big() {
    let mut small = PlayerSim::new();
    small.powerup = 0;
    small.probe_point = (360, 479);
    small.block_bonk(0x03);
    assert_eq!(small.sfx2, 0, "small Mario only rattles it");
    assert_eq!(small.block_slots[0][0], 6);
    small.block_update();
    assert_eq!(small.layout_writes, vec![(7190, 24)], "timer 2: $5BFC");
    for _ in 0..6 {
        small.block_update();
    }
    assert_eq!(small.block_slots[0][1], 8);
    assert_eq!(small.block_slots[0][0], 0, "freed at 8");
    assert_eq!(
        small.layout_writes,
        vec![(7190, 3)],
        "$5BC8: the `$03` block's used tile, from its own `3E 03 / call $5CF1`"
    );

    let mut big = PlayerSim::new();
    big.powerup = 1;
    big.probe_point = (360, 479);
    big.block_bonk(0x03);
    assert_eq!(big.sfx2, 1, "$19C1's big arm: the destruction sound");
    assert_eq!(big.block_slots[0][0], 5);
    big.block_update();
    assert_eq!(
        big.layout_writes,
        vec![(7190, 127)],
        "$5A7A: the `$7F` used tile"
    );
    while big.block_slots[0][0] != 0 {
        big.block_update();
    }
    assert_eq!(big.block_slots[0][1], 0x28, "type 5 lives 40 ticks");
    assert_eq!(
        big.layout_writes,
        vec![(7190, 127)],
        "written at 2 and at $28"
    );

    // The `$00` control: small Mario's brick only rattles (type 2, the brick
    // tile itself returns at 8).
    let mut brick = PlayerSim::new();
    brick.powerup = 0;
    brick.probe_point = (360, 479);
    brick.block_bonk(0x00);
    assert_eq!(brick.block_slots[0][0], 2);
    assert_eq!(brick.sfx2, 0);
    for _ in 0..7 {
        brick.block_update();
    }
    assert_eq!(brick.block_slots[0][0], 0);
    assert_eq!(brick.layout_writes, vec![(7190, 0)], "$5BBC: back to empty");
}

/// The single-slot draw `bonk_single_slot_draw` was filed for. traj15's `$01`
/// bonk allocates a type 3 AND a type 4, and the coin cycle (slot 4) draws
/// after the bonk slot, so the console never leaves 66 in `$FFC6` there. A
/// LONE type-3 slot does: `$5B8A` is `ld a,66 / ldh [$FFC6],a` (bytes
/// `3E 42 E0 C6`) at `slotY - sScrollY + $5BB4[timer]`, with no timer term on
/// X.
#[test]
fn a_lone_bump_slot_leaves_frame_66_and_the_coin_slot_overrides_it() {
    let mut sim = PlayerSim::new();
    sim.block_slots[0] = [3, 1, 0xD8, 0x01, 0x70, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    sim.block_update(); // timer 2: `$5BFC` writes 24, no draw
    assert_eq!(sim.block_slots[0][1], 2);
    assert_eq!(sim.block_frame, None);
    assert_eq!(sim.layout_writes, vec![(7190, 24)]);
    sim.block_update(); // timer 3: the 66 draw, `$5BB4[3] = -4`
    assert_eq!(sim.block_frame, Some(66));
    assert_eq!(
        sim.block_draw,
        vec![(352, 442, 66)],
        "368 - 16, 472 - 4 - 26"
    );
    // A later slot's coin cycle draws AFTER it and wins `$FFC6` — which is
    // why the capture's two-slot bump shows the coin cycle and not 66.
    sim.block_slots[1] = [4, 1, 0xD8, 0x01, 0x70, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    sim.block_update(); // slot 0 at timer 4 (66), slot 1 at timer 2 ($6D)
    assert_eq!(sim.block_frame, Some(0x6D), "the last drawing slot wins");
    assert_eq!(sim.block_draw.len(), 2);
    assert_eq!(sim.block_draw[0].2, 66, "the bonk slot did draw");
    assert_eq!(sim.block_draw[1], (352, 430, 0x6D), "$5AE0[1] = -16");
    // Isolate the type-3 slot again and run it to its removal.
    sim.block_slots[1][0] = 0;
    while sim.block_slots[0][0] != 0 {
        sim.block_update();
    }
    assert_eq!(sim.block_slots[0][1], 8, "types 2/3/6 free at 8");
    assert_eq!(
        sim.layout_writes,
        vec![(7190, 7)],
        "$5B3C: the `$07` used tile"
    );
}

/// The level reload clears the bump scratch — including the pass's arming
/// and its pending coin.
/// `$0F2A` and `$04F1` leave the flutter timer, latch, and charge alone.
#[test]
fn respawn_keeps_the_flutter_state() {
    let mut sim = small_sim();
    sim.flutter = 3;
    sim.flutter_armed = true;
    sim.flutter_charge = 4;
    sim.respawn(34, 468);
    assert_eq!((sim.flutter, sim.flutter_armed, sim.flutter_charge), (3, true, 4));
}

#[test]
fn respawn_clears_the_below_hit_scratch() {
    let mut sim = PlayerSim::new();
    sim.probe_point = (360, 479);
    sim.block_bonk(0x01);
    assert!(sim.below_bonk.is_some() && sim.bump_coin_pending);
    sim.respawn(34, 468);
    assert_eq!(sim.below_bonk, None);
    assert!(!sim.bump_coin_pending);
    assert_eq!(sim.object_bump_special, 0);
    assert_eq!(sim.block_slots, [[0; 16]; 8]);
}

/// The `$2F72` hand-off reads the *previous* frame's scroll — it runs in
/// `$0F71`, before `$0B2A`/`$2062`. Pinned here as the exact byte math the
/// replay loop feeds to `ObjectSim::below_bonk`: the traj15 bonk (camera
/// 448/377, probe 360/479) lands on the block's centre (248, 151).
#[test]
fn the_bonk_point_maps_to_the_block_centre_in_screen_bytes() {
    let cam_x = 448u16;
    let cam_y = 377u16;
    let (px, py) = (360u16, 479u16);
    let scroll_y = cam_y.wrapping_sub(72) as u8; // no pan in-level
    let scroll_x = cam_x.wrapping_sub(80) as u8;
    assert_eq!(scroll_y, 0x31); // 305 & $FF
    assert_eq!(scroll_x, 0x70); // 368 & $FF
    let by = (py as u8 & 0xF0)
        .wrapping_add(8)
        .wrapping_sub(scroll_y)
        .wrapping_sub(16);
    let bx = ((px as u8).wrapping_add(8) & 0xF0)
        .wrapping_sub(scroll_x)
        .wrapping_sub(8);
    assert_eq!((by, bx), (0x97, 0xF8), "the block's centre (151, 248)");
}

// -----------------------------------------------------------------
// $126F / $1327 — the spin jump: the engage toggle, the landing bonk
// ($1993 off the floor probe) and the bounce.
//
// No capture holds DOWN mid-air on an odd powerup, so the console-side
// route never engages a spin jump in the suite; the branch is pinned
// here instead (the wiring into the object engine's stomp dispatch is
// `MarioContact.spin_jump`, fed from `sim.spin_jump` by the runtime).
// -----------------------------------------------------------------

/// `$126F`, through the real per-frame entry: DOWN held, odd powerup,
/// airborne and no moon physics engages (`$FF`) with the engage-edge
/// spin sfx 14 on the first frame only; releasing DOWN — or an even
/// powerup (small/bunny) — clears it.
#[test]
fn spin_jump_engages_on_down_in_air_only_for_odd_powerups() {
    let layout = blank_layout();
    let down = PlayerKeys {
        down: true,
        ..Default::default()
    };
    let mut sim = PlayerSim::new(); // powerup 1 (big): odd
    sim.on_ground = false;
    sim.step(&down, false, false, &layout);
    assert_eq!(sim.spin_jump, 0xFF);
    assert_eq!(sim.sfx0, 14, "the engage-edge spin sfx");
    // Still spinning next frame: the sfx does not re-fire.
    sim.sfx0 = 0;
    sim.step(&down, false, false, &layout);
    assert_eq!(sim.spin_jump, 0xFF);
    assert_eq!(sim.sfx0, 0);
    // DOWN released: $129A clears it.
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(sim.spin_jump, 0);
    // Small Mario (powerup 0, even — the bit-0 test) never engages.
    let mut sim = small_sim();
    sim.on_ground = false;
    sim.step(&down, false, false, &layout);
    assert_eq!(sim.spin_jump, 0);
    assert_eq!(sim.sfx0, 0);
}

/// `$1327`: spin active, but the landing tile is not a bump class —
/// `$1993`'s `cp $04 / ret nc` leaves `sSpinJumpBounce` clear, so the
/// ordinary `$134F` landing runs (on-ground set, vvel untouched).
#[test]
fn spin_jump_lands_normally_on_solid_ground() {
    let keys = PlayerKeys::default();
    for tile in [0x04, 0x0F] {
        let mut sim = small_sim();
        sim.spin_jump = 0xFF;
        sim.vvel = 10;
        sim.landing(tile, &keys);
        assert!(sim.on_ground, "tile ${tile:02X} lands");
        assert!(!sim.in_air);
        assert_eq!(sim.vvel, 10, "the bounce never touched vvel");
        assert_eq!(sim.spin_jump_bounce, 0x00);
        assert!(sim.block_slots.iter().all(|s| s[0] == 0));
        assert!(sim.below_bonk.is_none());
    }
}

/// `$1327` + `$1993`: small Mario spinning onto a `$00` brick bumps type
/// 2 (`$19B0`'s small arm — the brick is only rattled) and bounces:
/// in-air, vvel 48, or 75 while A is held (the `bit 0` read on
/// hKeysHeld). The `$134F` landing is skipped, so on-ground stays clear.
#[test]
fn spin_jump_on_a_brick_bumps_type_2_and_bounces_for_small_mario() {
    let mut sim = small_sim(); // powerup 0
    sim.spin_jump = 0xFF;
    sim.vvel = 10;
    sim.landing(0x00, &PlayerKeys::default());
    assert_eq!(sim.block_slots[0][0], 2, "the $19B0 small bump");
    assert_eq!(sim.block_slots[0][1], 1, "a fresh slot's timer");
    assert_eq!(sim.spin_jump_bounce, 0xFF);
    assert!(sim.in_air);
    assert_eq!(sim.vvel, 48);
    assert!(!sim.on_ground, "the $134F landing must not run");
    assert_eq!(sim.sfx0, 2, "$19F1's bump sfx");
    assert_eq!(sim.sfx2, 0, "small Mario destroys nothing");
    // The higher bounce: A held at the landing.
    let keys_a = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let mut sim = small_sim();
    sim.spin_jump = 0xFF;
    sim.vvel = 10;
    sim.landing(0x00, &keys_a);
    assert_eq!(sim.block_slots[0][0], 2);
    assert_eq!(sim.spin_jump_bounce, 0xFF);
    assert_eq!(sim.vvel, 75);
    assert!(!sim.on_ground);
}

/// `$1327` + `$19B0`: big Mario spinning onto a `$00` brick destroys it —
/// `sSfxRequest2` (sfx2) and the type-1 bump whose `$5CF1` write turns
/// the cell into the `$60` empty metatile once the slot ticks — and
/// bounces exactly like the small case.
#[test]
fn spin_jump_breaks_a_brick_for_big_mario_with_sfx2() {
    let mut sim = PlayerSim::new(); // powerup 1: big
    sim.x = 100;
    sim.y = 200;
    sim.spin_jump = 0xFF;
    sim.vvel = 10;
    sim.probe_point = (360, 479); // slot Y/X round to cell (28, 22)
    sim.landing(0x00, &PlayerKeys::default());
    assert_eq!(sim.block_slots[0][0], 1, "the $19B0 destruction bump");
    assert_eq!(sim.sfx2, 1, "the break sound");
    assert_eq!(sim.spin_jump_bounce, 0xFF);
    assert!(sim.in_air);
    assert_eq!(sim.vvel, 48);
    assert!(!sim.on_ground);
    // The destruction shows when the slot ticks: timer 2 writes $60.
    sim.block_update();
    assert_eq!(sim.layout_writes, vec![(cell_index(28, 22), 0x60)]);
}

/// Without a spin, the landing never calls `$1993`: a `$00` brick
/// underfoot is inert and Mario lands on it like any solid ground.
#[test]
fn normal_landing_never_bonks_the_tile_below() {
    let mut sim = small_sim(); // spin_jump 0
    sim.vvel = 10;
    sim.landing(0x00, &PlayerKeys::default());
    assert!(sim.on_ground);
    assert_eq!(sim.spin_jump_bounce, 0);
    assert!(sim.block_slots.iter().all(|s| s[0] == 0));
    assert_eq!(sim.sfx0, 0);
    assert_eq!(sim.sfx2, 0);
    assert_eq!(sim.vvel, 10, "vvel is not a bounce's");
}

/// The whole `$129F`→`$1327` path through `step`: a spinning big Mario
/// falling onto a `$00` floor brick bumps it, bounces with the A-held 75
/// — this frame's own gravity already decremented it once, and the hold
/// suppresses the jump-cut — and never touches down (`$134F`'s
/// on-ground write is skipped). The floor probe reads (x+12, y+44) =
/// (112, 240) = row 14, col 6; camx is pinned so `$0FBF` does not wrap.
#[test]
fn step_spin_landing_bounces_off_the_brick() {
    let mut layout = blank_layout();
    layout.cells[14 * 256 + 6] = 0x00;
    let mut sim = PlayerSim::new(); // big
    sim.x = 100;
    sim.y = 196;
    sim.camx = 100;
    sim.vvel = 10; // falling: the $129F gate is open
    sim.spin_jump = 0xFF;
    let keys = PlayerKeys {
        down: true,
        a: true,
        ..Default::default()
    };
    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.collision_tile, 0x00);
    assert_eq!(sim.block_slots[0][0], 1, "the brick was destroyed");
    assert_eq!(sim.sfx2, 1);
    assert_eq!(sim.spin_jump_bounce, 0xFF);
    assert!(sim.in_air);
    assert!(!sim.on_ground, "$134F must not run on the bounce");
    assert_eq!(sim.vvel, 74, "the 75 bounce minus gravity's decrement");
}

// -----------------------------------------------------------------
// $311A / $27AF / $287E — the zero-lives game over.
//
// No capture reaches zero lives (every gate capture survives), so the
// whole path is pinned here: the `sub 1 / daa` borrow that selects state
// `$05` (and the tens that must *not* select it), `$27AF`'s state writes,
// and `$287E`'s 255-frame fade + START wait.
// -----------------------------------------------------------------

/// The borrow is `sLives == 0`: `sub 1 / daa` on BCD 00 gives $99 with
/// carry, which is `$314B`'s `ld a, 5 / ldh [$FF9B], a`.
#[test]
fn death_fade_borrow_on_zero_lives_raises_the_game_over_flag() {
    let mut sim = PlayerSim::new();
    sim.lives = 0;
    sim.fade_timer = 1;
    sim.fade_step();
    assert_eq!(sim.lives, 0x99);
    assert!(sim.game_over, "the borrow selects state $05");
}

/// `daa`'s low-nibble correction (0x06, taken when H is set) does not
/// touch C, so a ten decrements to $09 and the game continues.
#[test]
fn death_fade_on_a_ten_lives_decrements_without_ending_the_game() {
    let mut sim = PlayerSim::new();
    sim.lives = 0x10;
    sim.fade_timer = 1;
    sim.fade_step();
    assert_eq!(sim.lives, 0x09);
    assert!(!sim.game_over, "$10 -> $09 carries nothing into the branch");
}

#[test]
fn death_fade_on_a_plain_life_still_decrements_by_one() {
    let mut sim = PlayerSim::new();
    sim.lives = 0x05;
    sim.fade_timer = 1;
    sim.fade_step();
    assert_eq!(sim.lives, 0x04);
    assert!(!sim.game_over);
}

/// `$27AF`: lives back to 5, the coin counters cleared, the 255-frame
/// fade armed, the music request set before the `$07` hand-off.
#[test]
fn game_over_enter_writes_the_05_fields() {
    let mut sim = PlayerSim::new();
    sim.lives = 0x99;
    sim.coins_low = 0x42;
    sim.coins_high = 0x01;
    sim.fade_timer = 0;
    sim.game_over_enter();
    assert_eq!(sim.lives, 5);
    assert_eq!((sim.coins_high, sim.coins_low), (0, 0));
    assert_eq!(sim.fade_timer, 255);
    assert_eq!(sim.music_request, 7);
}

/// `$27AF`'s fold order — coins 1, 2, 6, 5, 4, 3 into bits 5..0 — is *not* the
/// order the bytes sit in memory, and getting it wrong would silently
/// shuffle the castle screen's icons.
#[test]
fn golden_coin_flags_fold_in_the_roms_order() {
    for (idx, want) in [
        (0usize, 0x20u8),
        (1, 0x10),
        (2, 0x08),
        (3, 0x04),
        (4, 0x02),
        (5, 0x01),
    ] {
        let mut sim = small_sim();
        sim.level_progress[PlayerSim::GOLDEN_COINS[idx]] = 0x80;
        assert_eq!(sim.golden_coin_flags(), want, "fold slot {idx}");
        assert_eq!(sim.golden_coin_count(), 1);
    }

    let mut all = small_sim();
    for off in PlayerSim::GOLDEN_COINS {
        all.level_progress[off] = 0x80;
    }
    assert_eq!(all.golden_coin_flags(), 0x3F, "all six");
    assert_eq!(all.golden_coin_count(), 6);

    // Every consumer tests bit 7 alone: a byte with the rest set is not a
    // collected coin.
    let mut other = small_sim();
    other.level_progress[PlayerSim::GOLDEN_COINS[0]] = 0x7F;
    assert_eq!(other.golden_coin_flags(), 0);
    assert_eq!(other.golden_coin_count(), 0);
}

/// `$27AF`'s tail is not just a reset: it folds the coins, charges the BCD
/// tally for each one, wipes the six bytes, and keeps the flags for the
/// `$30884` icon draw.
#[test]
fn game_over_charges_the_tally_and_wipes_the_golden_coins() {
    let mut sim = small_sim();
    sim.level_tally = 0x12;
    sim.level_progress[PlayerSim::GOLDEN_COINS[0]] = 0x80; // coin 1
    sim.level_progress[PlayerSim::GOLDEN_COINS[1]] = 0x80; // coin 2
    sim.game_over_enter();
    assert_eq!(sim.level_tally, 0x10, "12 - 2, in BCD");
    assert_eq!(sim.golden_coin_flags, 0x30, "folded before the wipe");
    assert_eq!(
        sim.golden_coin_flags.count_ones(),
        2,
        "$27AF recomputes the count from the flags byte"
    );
    assert_eq!(
        sim.golden_coin_count(),
        0,
        "while $307BC's byte-derived count is 0 once the bytes are wiped — \
         the two derivations agree before the clear, which is when $27AF uses it"
    );
    assert!(
        PlayerSim::GOLDEN_COINS
            .iter()
            .all(|o| sim.level_progress[*o] == 0),
        "all six cleared"
    );

    // `daa` wraps rather than going negative: 05 - 6 = $99.
    let mut wrap = small_sim();
    wrap.level_tally = 0x05;
    for off in PlayerSim::GOLDEN_COINS {
        wrap.level_progress[off] = 0x80;
    }
    wrap.game_over_enter();
    assert_eq!(wrap.level_tally, 0x99);

    // With no coins — the only case a game started from scratch can reach,
    // since nothing in the ROM sets the bits — nothing changes.
    let mut none = small_sim();
    none.level_tally = 0x07;
    none.game_over_enter();
    assert_eq!(none.level_tally, 0x07);
    assert_eq!(none.golden_coin_flags, 0);
}

/// `$17C5`: while the bell rings, the input block is *replaced*, not skipped.
/// Mario faces right and his horizontal velocity is re-armed every frame, so
/// `$16CC`'s friction leaves him coasting into the exit door; airborne it is
/// `$80` — no drift — so the coast is grounded-only.
///
/// No capture can see this: level 1's bell sits at anchor y 360 and the
/// measured jump apex on that floor is y 399, 44 px short of the touch window
/// (`marioY - objectY` in `[-43, -13)`) — so no shipped capture ever rings a
/// bell, and this test is the only coverage the behaviour has.
#[test]
fn the_goal_bell_replaces_the_input_with_a_rightward_coast() {
    let keys = PlayerKeys::default();
    let layout = blank_layout();

    let mut sim = small_sim();
    sim.hvel = 0x80;
    // `on_ground_obj` is what makes `$129F` skip the floor probe, so he stays
    // grounded on this floor-less layout; otherwise he is airborne and the
    // bell takes the *other* arm.
    sim.on_ground = true;
    sim.on_ground_obj = true;
    sim.step(&keys, false, false, &layout);
    assert!(sim.on_ground, "still grounded");
    assert_eq!(sim.hvel, 0x80, "no bell, no input: he stands");

    sim.goal_bell = true;
    sim.hvel = 0x80;
    sim.step(&keys, false, false, &layout);
    assert!(sim.dir_right, "$17C5 faces him right");
    assert_eq!(
        sim.hvel, 0xB4,
        "$B8 re-armed, then $16CC's friction takes 4"
    );

    // Airborne: `$80`, and friction does not run, so there is no drift.
    sim.on_ground_obj = false; // else `$129F` re-grounds him on the next probe
    sim.on_ground = false;
    sim.in_air = true;
    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.hvel, 0x80, "$17C5's airborne arm");

    // The bell arm is the first branch, so it also ignores the duck and
    // push gates the normal arm has (`$17C5` is reached before either).
    sim.on_ground_obj = true;
    sim.on_ground = true;
    sim.in_air = false;
    sim.ducking = true;
    sim.step(&keys, false, false, &layout);
    assert_eq!(
        sim.hvel, 0xB4,
        "still walking: the bell arm has no duck gate"
    );
    assert!(sim.dir_right, "and no direction change");
}

/// The six bytes live inside the block `$2934` saves, so a collected coin
/// shows up in the record at the offsets the save layout documents. This pins
/// [`PlayerSim::GOLDEN_COINS`] to the save layout as well as to `$27AF`.
#[test]
fn the_golden_coin_bytes_are_the_save_records_offsets() {
    let mut sim = small_sim();
    for (off, rec) in [
        (5usize, 0x0Dusize),
        (9, 0x11),
        (14, 0x16),
        (17, 0x19),
        (23, 0x1F),
        (29, 0x25),
    ] {
        assert_eq!(sim.save_record(0)[rec], 0);
        sim.level_progress[off] = 0x80;
        assert_eq!(
            sim.save_record(0)[rec],
            0x80,
            "level_progress[{off}] -> record ${rec:02X}"
        );
        sim.level_progress[off] = 0;
    }
    assert_eq!(
        PlayerSim::GOLDEN_COINS,
        [5, 9, 29, 23, 17, 14],
        "fold order: coins 1, 2, 6, 5, 4, 3"
    );
}

/// `$287E`: the fade ticks down for 255 frames, then the screen waits for
/// START, which hands back to state `$00` with `sMusicRequest = $FF`. The
/// compare is exact (`cp $08`), so START *plus* anything else is not START.
#[test]
fn game_over_step_ticks_the_fade_then_waits_for_start() {
    let mut sim = PlayerSim::new();
    sim.lives = 0;
    sim.fade_timer = 1;
    sim.fade_step();
    sim.game_over_enter();
    // The fade runs out over 255 frames; START during it does nothing.
    assert!(
        !sim.game_over_step(0x08),
        "START on the fade's first frame is ignored"
    );
    for _ in 0..254 {
        assert!(!sim.game_over_step(0x00));
    }
    assert_eq!(sim.fade_timer, 0);
    assert!(!sim.game_over_step(0x00), "no START, no hand-off");
    assert_eq!(sim.fade_timer, 0);
    // `$287E` is `cp $08` — an exact compare, so START held with A or SELECT
    // does not advance, which a `bool` parameter would have accepted.
    for other in [0x01u8, 0x04, 0x09, 0x0C] {
        assert!(
            !sim.game_over_step(other),
            "hKeysPressed ${other:02X} is not exactly START"
        );
    }
    assert!(sim.game_over_step(0x08), "the $00 hand-off frame");
    assert!(!sim.game_over);
    assert_eq!(sim.music_request, 0xFF);
    // The hand-off is one-shot: the phase is over.
    assert!(!sim.game_over_step(0x08));
}

/// VBlank decrements `sFadeTimer` before `$287E` tests it, so START on the
/// frame the fade reaches 0 already hands off.
#[test]
fn game_over_accepts_start_on_the_frame_the_fade_reaches_zero() {
    let mut sim = PlayerSim::new();
    sim.game_over = true;
    sim.fade_timer = 2;
    assert!(!sim.game_over_step(0x08), "fade 2 -> 1 still runs");
    assert!(sim.game_over_step(0x08), "fade 1 -> 0 and START");
    assert_eq!(sim.fade_timer, 0);
}

/// Outside the game-over phase the poll is inert (the flag gates it), and
/// a level reload clears the flag like the other phase flags.
#[test]
fn game_over_step_needs_the_phase_and_respawn_clears_it() {
    let mut sim = PlayerSim::new();
    sim.fade_timer = 0;
    assert!(!sim.game_over_step(0x08));
    sim.game_over = true;
    sim.respawn(112, 240);
    assert!(!sim.game_over);
}

// -----------------------------------------------------------------
// $273E / $2756 — the START pause and its palette flash.
//
// No capture carries START (the replay `Frame` has no field for it), so
// the handler is pinned here; what the gate cannot cover is flagged in the
// docs rather than assumed.
// -----------------------------------------------------------------

/// The flash is bit 4 of the `$FF97` counter read every frame, not a timer
/// with a reload: 16 frames of `$E7`, then 16 of `$93`, repeating.
#[test]
fn pause_flash_comes_off_bit_4_of_the_frame_counter() {
    let mut sim = PlayerSim::new();
    let seen: Vec<u8> = (0..32u8)
        .map(|i| {
            sim.frame_counter = i;
            sim.pause_flash()
        })
        .collect();
    assert_eq!(seen[..16], [0xE7; 16], "bit 4 clear -> $E7");
    assert_eq!(seen[16..], [0x93; 16], "bit 4 set -> $93");
    // It is the counter, not a toggle: the same value is back 16 later.
    sim.frame_counter = 32;
    assert_eq!(sim.pause_flash(), 0xE7);
    sim.frame_counter = 0x30;
    assert_eq!(sim.pause_flash(), 0x93);
}

/// `$273E` enters and `$2756` leaves; the flag *is* the port's model of
/// `$FF9B = 8`, and each handler's `sSoundMode` write rides along.
#[test]
fn pause_enters_and_exits() {
    let mut sim = PlayerSim::new();
    assert!(!sim.paused);
    assert_eq!(sim.sound_mode, 0);
    sim.pause_enter();
    assert!(sim.paused);
    assert_eq!(sim.sound_mode, 1, "$273E writes sSoundMode = 1");
    sim.pause_exit();
    assert!(!sim.paused);
    assert_eq!(sim.sound_mode, 2, "$278E writes sSoundMode = 2");
}

/// The SELECT branch needs `sLevelProgressFlags` bit 7 — the ROM's
/// `and a / ret z` shape — and with it the level is abandoned through the
/// `$14` hand-off rather than resumed.
#[test]
fn pause_select_exit_needs_the_cleared_level_flag() {
    let mut sim = PlayerSim::new();
    sim.pause_enter();
    sim.exit_tile_flag = 0xFF;
    sim.tile_anim_config = 0x11;
    assert!(
        !sim.pause_select_exit(),
        "bit 7 clear: SELECT leaves the handler alone"
    );
    assert!(sim.paused, "and the pause stands");

    sim.level_progress_flags = 0x80;
    assert!(sim.pause_select_exit(), "bit 7 set: the level is abandoned");
    assert!(!sim.paused);
    assert_eq!(sim.sound_mode, 2, "$2760 writes sSoundMode = 2");
    assert_eq!(sim.exit_tile_flag, 0);
    assert_eq!(sim.tile_anim_config, 0);
    assert!(sim.exit_completed, "the $14 hand-off is marked reached");
}

/// A level reload clears the pause like the other phase flags.
#[test]
fn respawn_clears_the_pause() {
    let mut sim = PlayerSim::new();
    sim.pause_enter();
    sim.respawn(112, 240);
    assert!(!sim.paused);
}

// -----------------------------------------------------------------
// $32C1 / $331C / $324F — the fireball family
// -----------------------------------------------------------------

/// `$3439` against the ROM's own bytes (re-read here, not the port's
/// constant), so a hand-edit of the array cannot quietly change the arc.
#[test]
fn the_fireball_phase_table_is_the_rom_bytes() {
    let rom: Vec<u8> = "03 03 03 03 02 03 02 02 01 02 01 00 00 01 02 01 02 02 03 02 03 03 03 03"
        .split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
        .collect();
    assert_eq!(FIRE_PHASE_STEP.to_vec(), rom, "$3439");
    assert_eq!(FIRE_PHASE_STEP.len(), 24, "the table's own extent");
}

/// Slot 0 built by hand: the mover tests set the six bytes the spawn's
/// init writes and nothing else.
fn fireball(sim: &mut PlayerSim, x: u16, y: u16, dir: u8, phase: u8) {
    let s = &mut sim.fireballs[0];
    *s = [0; 16];
    s[F_ACTIVE] = 1;
    s[F_YLO] = y as u8;
    s[F_YHI] = (y >> 8) as u8;
    s[F_XLO] = x as u8;
    s[F_XHI] = (x >> 8) as u8;
    s[F_DIR] = dir;
    s[F_PHASE] = phase;
}

/// `$32C1`'s init: the ball at Mario's feet (Y + 28) and right edge
/// (X + 16), both with carry, facing with him, phase 0, the throw pose
/// armed and sfx 0. Only `+0..+6` are written — nothing in this family
/// ever reads or writes `+7..+15`.
#[test]
fn fire_spawn_writes_the_full_slot() {
    let mut sim = PlayerSim::new();
    sim.powerup = 3;
    sim.x = 0x00F8;
    sim.y = 0x01F8;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][..7], [1, 0x14, 2, 0x08, 1, 0x01, 0]);
    assert_eq!(sim.fireballs[0][7..], [0; 9], "+7..+15 stay untouched");
    assert_eq!(
        sim.fireballs[1], [0; 16],
        "the scan took the first free slot"
    );
    assert_eq!(sim.throw_timer, 8);
    assert_eq!(sim.sfx0, 11);
}

/// The gates, in the ROM's order: no press edge, a non-flower tier,
/// ducking, or the reaching goal bell each leave both slots alone. A held
/// B is not an edge — the press is the whole rate limit.
#[test]
fn fire_spawn_gates() {
    let mut sim = PlayerSim::new();
    sim.powerup = 3;
    sim.fire_spawn(false);
    assert_eq!(sim.fireballs[0][F_ACTIVE], 0, "no edge, no ball");
    assert_eq!(sim.sfx0, 0);
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_ACTIVE], 1, "the tier-3 press spawns");

    let mut sim = PlayerSim::new();
    sim.powerup = 2; // the bunny tier cannot fire
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_ACTIVE], 0);
    sim.powerup = 3;
    sim.ducking = true;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_ACTIVE], 0, "ducking");
    sim.ducking = false;
    sim.goal_bell = true;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_ACTIVE], 0, "the goal bell");
}

/// The facing byte: Mario's own direction — or, mid spin jump, the rDiv
/// bit (`$FF97 & 2` clear -> `$FF`, set -> `$01`), which is why a spin jump
/// can fire *backwards*.
#[test]
fn fire_spawn_facing_and_spin_jump() {
    let mut sim = PlayerSim::new();
    sim.powerup = 3;
    sim.dir_right = false;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_DIR], 0xFF, "facing left");

    let mut sim = PlayerSim::new();
    sim.powerup = 3;
    sim.spin_jump = 0xFF;
    sim.phase = 2;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_DIR], 0x01, "rDiv bit 1 set");

    let mut sim = PlayerSim::new();
    sim.powerup = 3;
    sim.spin_jump = 0xFF;
    sim.phase = 0;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_DIR], 0xFF, "rDiv bit 1 clear");
}

/// Two slots, first free wins, and a press with both live is eaten in
/// silence: no sfx and no throw pose.
#[test]
fn fire_spawn_uses_the_first_free_slot_and_eats_the_third() {
    let mut sim = PlayerSim::new();
    sim.powerup = 3;
    sim.x = 100;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_XLO], 116);
    assert_eq!(sim.fireballs[1][F_ACTIVE], 0);
    sim.x = 200;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[1][F_XLO], 216, "the second slot");
    assert_eq!(sim.fireballs[0][F_XLO], 116, "the first is untouched");
    sim.sfx0 = 0;
    sim.throw_timer = 0;
    sim.x = 300;
    sim.fire_spawn(true);
    assert_eq!(sim.fireballs[0][F_XLO], 116);
    assert_eq!(sim.fireballs[1][F_XLO], 216);
    assert_eq!(sim.sfx0, 0, "silent");
    assert_eq!(sim.throw_timer, 0);
}

/// `$332E`: X steps ±3 by the facing byte, Y follows `$3439` — 3 px down
/// per frame while the phase is 0 (`$33F1` skips the decrement at 0, so a
/// falling ball's phase never moves) and *up* by the table once the phase
/// is >= `$0C`.
#[test]
fn fire_move_moves_three_px_and_steps_the_phase_table() {
    let layout = blank_layout();
    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 0x0140, 1, 0);
    sim.fire_move(&layout);
    let f = &sim.fireballs[0];
    assert_eq!(f[F_XLO], 103, "right: X + 3");
    assert_eq!(
        (f[F_YHI] as u16) << 8 | f[F_YLO] as u16,
        0x0143,
        "phase 0 descends by $3439[0] = 3"
    );
    assert_eq!(f[F_PHASE], 0, "and stays 0");

    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 0x0140, 0xFF, 0);
    sim.fire_move(&layout);
    assert_eq!(sim.fireballs[0][F_XLO], 97, "left: X - 3");
    assert_eq!(
        sim.fireballs[0][F_DIR], 0xFF,
        "the facing byte is never rewritten"
    );

    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 0x0140, 1, 0x0F);
    sim.fire_move(&layout);
    let f = &sim.fireballs[0];
    assert_eq!(
        (f[F_YHI] as u16) << 8 | f[F_YLO] as u16,
        0x013F,
        "phase $0F rises by $3439[$0F] = 1"
    );
    assert_eq!(f[F_PHASE], 0x0E);
}

/// Probe 2's pop: a ball that moves *into* a solid (probe 1 still in air,
/// probe 2 in the tile) is thrown back up — phase `$18`, Y -3 — and
/// `$33F1` decrements it to `$17` in the same frame.
#[test]
fn fire_move_pops_on_a_solid_cell() {
    let mut layout = blank_layout();
    layout.cells[20 * 256 + 5] = 0x27; // solid, but not $06
    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 334, 1, 0);
    sim.fire_move(&layout);
    let f = &sim.fireballs[0];
    assert_eq!(f[F_ACTIVE], 1, "a pop is not a kill");
    assert_eq!(f[F_PHASE], 0x17, "$18, then the same frame's decrement");
    assert_eq!(
        (f[F_YHI] as u16) << 8 | f[F_YLO] as u16,
        334,
        "Y - 3 undoes the step"
    );
    assert_eq!(f[F_XLO], 103);
}

/// Probe 1's break: `$06` is the only tile a ball can break and it takes
/// the hardcoded type-1 bump at the ball's *own* cell — the same slot the
/// big-Mario brick bonk allocates, whose tick empties the cell.
#[test]
fn fire_move_breaks_the_06_cell() {
    let mut layout = blank_layout();
    layout.cells[19 * 256 + 5] = 0x06;
    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 320, 1, 0);
    sim.fire_move(&layout);
    assert_eq!(sim.sfx2, 1, "the smash sfx");
    assert_eq!(sim.sfx0, 2, "and the bump sfx");
    assert_eq!(sim.spin_jump_bounce, 0xFF);
    assert_eq!(
        sim.block_slots[0][..6],
        [1, 1, 0x48, 1, 0x60, 0],
        "type 1, timer 1, the ball's cell"
    );
    assert_eq!(
        sim.fire_below_bonk,
        Some((0x40, 103)),
        "the $2F72 pass is armed"
    );
    assert_eq!(sim.fireballs[0][F_ACTIVE], 0, "the ball dies on the break");
    assert_eq!(
        sim.fireballs[0][F_XLO], 100,
        "the ROM never writes the move back on a kill"
    );
    // `$2C82` — the same frame, after the update — turns the cell into the
    // empty metatile every later probe reads.
    sim.block_update();
    assert_eq!(sim.block_slots[0][1], 2);
    assert_eq!(sim.layout_writes, vec![(cell_index(19, 5), 0x60)]);
}

/// Probe 1 is terminal for every other low tile: the `$00` brick, the `$01`
/// item block and the `$03` bridge block all kill the ball outright — no
/// bump slot, no smash sfx, no below-hit pass.
#[test]
fn fire_kills_on_other_special_tiles() {
    for tile in [0x00u8, 0x01, 0x03] {
        let mut layout = blank_layout();
        layout.cells[19 * 256 + 5] = tile;
        let mut sim = PlayerSim::new();
        fireball(&mut sim, 100, 320, 1, 0);
        sim.fire_move(&layout);
        assert_eq!(sim.fireballs[0][F_ACTIVE], 0, "tile {tile:#04X} kills");
        assert_eq!(sim.sfx0, 2);
        assert_eq!(sim.sfx2, 0, "no smash");
        assert_eq!(sim.spin_jump_bounce, 0);
        assert_eq!(sim.block_slots[0][0], 0, "no bump slot");
        assert_eq!(sim.fire_below_bonk, None);
    }
}

/// `$3261`'s cull: a pair whose high nibble is `$C0` kills the ball before
/// the object pass, silently, and nothing is staged. A surviving pair is
/// staged for the frame loop's `$2FED` walk.
#[test]
fn fire_cull_masks() {
    // sx = Xlo - camX + 80; camX 0 and Xlo $75 give $C5.
    let mut sim = PlayerSim::new();
    fireball(&mut sim, 0x75, 0x0140, 1, 0);
    sim.fire_pass();
    assert_eq!(sim.fireballs[0][F_ACTIVE], 0, "the X pair culls");
    assert_eq!(sim.fire_scan[0], None);
    assert!(sim.fire_draw.is_empty(), "a culled ball draws nothing");
    assert_eq!(sim.sfx0, 0, "silently");

    // sy = Ylo - camY + 70; Ylo $7F gives the same $C5.
    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 0x007F, 1, 0);
    sim.fire_pass();
    assert_eq!(sim.fireballs[0][F_ACTIVE], 0, "the Y pair culls");

    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 0x0140, 1, 0);
    sim.fire_pass();
    assert_eq!(sim.fireballs[0][F_ACTIVE], 1);
    assert_eq!(sim.fire_scan[0], Some((0x86, 0xB4)));
    assert_eq!(sim.fire_scan[1], None);
}

/// `$3261`'s tail: a non-zero object verdict kills the ball; a `$00` leaves
/// it flying.
#[test]
fn fire_hit_kills_the_ball() {
    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 0x0140, 1, 0);
    sim.resolve_fire_hit(0, true);
    assert_eq!(sim.fireballs[0][F_ACTIVE], 0);

    let mut sim = PlayerSim::new();
    fireball(&mut sim, 100, 0x0140, 1, 0);
    sim.resolve_fire_hit(0, false);
    assert_eq!(sim.fireballs[0][F_ACTIVE], 1);
}

/// The wiring end to end through the frame entry point: one `step` with a B
/// edge at the flower tier spawns, moves (X + 3) and stages the pair — the
/// ROM's own order inside `$0F71`'s tail.
#[test]
fn a_b_press_through_step_spawns_moves_and_stages_the_ball() {
    let layout = blank_layout();
    let mut sim = PlayerSim::new();
    sim.powerup = 3;
    sim.x = 50;
    sim.y = 320;
    let keys = PlayerKeys {
        b: true,
        ..PlayerKeys::default()
    };
    sim.step(&keys, false, true, &layout);
    let f = &sim.fireballs[0];
    assert_eq!(f[F_ACTIVE], 1, "the tail spawned");
    assert_eq!(
        (f[F_XHI] as u16) << 8 | f[F_XLO] as u16,
        sim.x.wrapping_add(16).wrapping_add(3),
        "X + 16 at the spawn, then the move's +3"
    );
    assert_eq!(
        (f[F_YHI] as u16) << 8 | f[F_YLO] as u16,
        sim.y.wrapping_add(28).wrapping_add(3),
        "Y + 28, then the arc's first step"
    );
    assert_eq!(f[F_PHASE], 0);
    // The staged pair is the ball's own low bytes, `- camX + 80` / `- camY + 70` with the
    // cameras still at 0 here — `fire_cull_masks` pins the arithmetic itself.
    assert_eq!(
        sim.fire_scan[0],
        Some((f[F_YLO].wrapping_add(70), f[F_XLO].wrapping_add(80))),
        "the pair $2FED tests"
    );
    // And the sprite: `$B4` (facing right) + the rDiv flicker bits, anchored
    // so `draw_cells`' `+96/+98` lands it on the staged pair.
    assert_eq!(
        sim.fire_draw,
        vec![(
            ((f[F_XHI] as u16) << 8 | f[F_XLO] as u16).wrapping_sub(16),
            ((f[F_YHI] as u16) << 8 | f[F_YLO] as u16).wrapping_sub(28),
            0xB4 + ((sim.phase & 0x06) >> 1),
        )]
    );
}

/// `$1E3B` stages the shared `$FFB7`/`$FFB9` pair to the point it read:
/// `(x+12, y+44)`, or `(x+20, y+44)` when the first found no floor — and
/// nothing at all when the `(y-4) & $F` gate bails.
#[test]
fn the_floor_probe_stages_the_pair_it_read() {
    let mut layout = blank_layout();
    layout.cells[22 * 256 + 6] = 0x20; // under the x+12 probe only
    let mut sim = PlayerSim::new();
    sim.x = 100;
    sim.y = 324;
    sim.probe_point = (1, 2);
    assert_eq!(sim.floor_probe(&layout), Some(0x20));
    assert_eq!(sim.probe_point, (112, 368), "the first probe's point");

    layout.cells[22 * 256 + 6] = 0x60;
    layout.cells[22 * 256 + 7] = 0x20; // only the x+20 probe finds one
    sim.probe_point = (1, 2);
    assert_eq!(sim.floor_probe(&layout), Some(0x20));
    assert_eq!(sim.probe_point, (120, 368), "the second probe's point");

    sim.y = 320; // (320-4) & $F = 12: the gate bails before any staging
    sim.probe_point = (1, 2);
    assert_eq!(sim.floor_probe(&layout), None);
    assert_eq!(sim.probe_point, (1, 2), "the gate stages nothing");
}

/// The spin landing (`$1327`): a spinning Mario who lands on a brick runs
/// the same `$1993` classifier a ceiling bonk does and BOUNCES (vvel 48, 75
/// while A is held, `$134F` skipped) — and the bump it allocates must sit on
/// the *floor* probe's cell. This is the play-test bug: the alloc read a
/// stale ceiling staging, so the brick under him was never the cell that
/// became `$60` (the breakable block "didn't break") and the shard sprites
/// drew at the stale point instead.
#[test]
fn a_spinning_landing_bonks_and_breaks_the_block_under_him() {
    let mut layout = blank_layout();
    layout.cells[22 * 256 + 6] = 0x00; // the brick at the floor probe's cell
    let mut sim = PlayerSim::new();
    sim.powerup = 1;
    sim.x = 100;
    sim.y = 324;
    sim.spin_jump = 0xFF;
    sim.probe_point = (360, 479); // a stale ceiling bonk: the play-test case
    let t = sim.floor_probe(&layout).expect("the brick is under him");
    assert_eq!(t, 0x00);
    assert_eq!(sim.probe_point, (112, 368), "$1E3B restages FFB7/FFB9");
    sim.landing(t, &PlayerKeys::default());
    assert_eq!(sim.spin_jump_bounce, 0xFF);
    assert_eq!(sim.sfx2, 1, "the smash");
    assert_eq!(
        sim.block_slots[0][..6],
        [1, 1, 0x78, 1, 0x70, 0],
        "the BRICK's own cell, not the stale ceiling point"
    );
    assert!(sim.in_air, "and the landing bounces");
    assert_eq!(sim.vvel, 48);
    assert!(!sim.on_ground, "the $134F arm is skipped");
    sim.block_update();
    assert_eq!(
        sim.layout_writes,
        vec![(cell_index(22, 6), 0x60)],
        "the brick is emptied"
    );

    // The same landing with A held takes the higher bounce ($1335).
    let mut sim = PlayerSim::new();
    sim.powerup = 1;
    sim.x = 100;
    sim.y = 324;
    sim.spin_jump = 0xFF;
    sim.floor_probe(&layout);
    let keys = PlayerKeys {
        a: true,
        ..PlayerKeys::default()
    };
    sim.landing(0x00, &keys);
    assert_eq!(sim.vvel, 75);
}

/// `$548A`: the fire tier's throw pose — base +112, plus a further 32 while
/// `sMarioThrowTimer` runs, decremented once per draw. The spawn sets 8, so
/// the pose holds eight frames and then the plain fire pose returns. No
/// other tier touches the timer.
#[test]
fn the_throw_pose_runs_for_eight_frames() {
    let mut sim = PlayerSim::new();
    sim.powerup = 3;
    sim.throw_timer = 8;
    assert_eq!(sim.frame_index(0), 144, "the throw pose on the spawn frame");
    for _ in 0..7 {
        assert_eq!(sim.frame_index(0), 144);
    }
    assert_eq!(sim.throw_timer, 0);
    assert_eq!(sim.frame_index(0), 112, "back to the normal fire pose");

    let mut sim = PlayerSim::new();
    sim.powerup = 1;
    sim.throw_timer = 8;
    assert_eq!(sim.frame_index(0), 0);
    assert_eq!(sim.throw_timer, 8, "tier 1 leaves the timer alone");
}

/// The replay gate's guarantee: with no live slot and no press edge the
/// whole family is inert — a frame touches no fireball byte, stages
/// nothing and plays no sfx.
#[test]
fn the_fireball_calls_are_inert_without_slots() {
    let layout = blank_layout();
    let mut sim = PlayerSim::new();
    sim.powerup = 3; // even a flower, with no press edge, does nothing
    let before = sim.fireballs;
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(sim.fireballs, before);
    assert_eq!(sim.fire_scan, [None; FIRE_SLOTS]);
    assert_eq!(sim.fire_below_bonk, None);
    assert!(sim.fire_draw.is_empty(), "nothing was drawn either");
    assert_eq!(sim.throw_timer, 0);
    assert_eq!(sim.sfx0, 0);
    assert_eq!(sim.sfx2, 0);
    assert_eq!(sim.block_slots[0][0], 0);
}

#[test]
fn pan_timer_blocks_jump_and_acceleration_but_preserves_coasting_and_goal_priority() {
    let layout = blank_layout();
    let keys = PlayerKeys {
        a: true,
        right: true,
        b: true,
        ..Default::default()
    };
    let mut sim = small_sim();
    sim.on_ground_obj = true;
    sim.pan_timer = 2;
    sim.dir_right = false;
    sim.step(&keys, true, false, &layout);
    assert_eq!((sim.hvel, sim.vvel, sim.pan_timer), (0x80, 24, 2));
    assert!(sim.on_ground);
    assert!(!sim.jump_flag);
    assert!(!sim.dir_right);
    assert_ne!(sim.walk_timer, 0, "the rest of the frame still runs");

    let mut coast = small_sim();
    coast.on_ground_obj = true;
    coast.camx = 100;
    coast.phase = 2; // ROM speed table $19FF + $9E = 1 pixel
    coast.hvel = 0xA0;
    coast.pan_timer = 1;
    coast.step(
        &PlayerKeys {
            left: true,
            ..Default::default()
        },
        false,
        false,
        &layout,
    );
    assert_eq!(
        coast.hvel, 0x9C,
        "friction still follows the skipped acceleration"
    );
    assert_eq!(coast.x, 101, "existing momentum is integrated");

    let mut goal = small_sim();
    goal.on_ground_obj = true;
    goal.pan_timer = 2;
    goal.goal_bell = true;
    goal.step(&keys, true, false, &layout);
    assert_eq!(
        goal.hvel, 0xB4,
        "$1701 checks the goal bell before the pan timer"
    );
}

#[test]
fn bunny_flutter_airborne_trigger_and_charge_cycle() {
    let layout = blank_layout();
    let mut sim = small_sim();
    sim.powerup = 2; // Bunny Mario
    sim.on_ground = false;
    sim.in_air = true;
    sim.vvel = 24; // apex / below threshold 0x48

    // Frame 0: A newly pressed in midair initiates flutter
    sim.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        true, // a_pressed
        false,
        &layout,
    );
    assert_eq!(
        sim.flutter, 7,
        "flutter decremented by 1 in gravity step (8 - 1)"
    );
    assert!(sim.flutter_armed, "flutter armed latch set");
    assert_eq!(sim.flutter_charge, 0);
    assert_eq!(sim.y, 200, "gravity bypassed during flutter");

    // Frames 1..=7: flutter timer decays each frame (7 down to 0) while charge accumulates (1 to 7)
    for (idx, remaining) in (0..=6).rev().enumerate() {
        sim.step(
            &PlayerKeys {
                a: true,
                ..Default::default()
            },
            false, // held, not newly pressed
            false,
            &layout,
        );
        assert_eq!(sim.flutter, remaining);
        assert_eq!(sim.flutter_charge, (idx + 1) as u8);
        assert_eq!(sim.y, 200, "gravity still bypassed");
    }

    // Frame 8: flutter has reached 0. Charge reaches 8, first falling frame
    sim.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        false,
        false,
        &layout,
    );
    assert_eq!(sim.flutter, 0);
    assert_eq!(sim.flutter_charge, 8, "charge counter reaches 8");
    assert_eq!(sim.vvel, 23, "gravity ticks downward");
    assert_eq!(sim.y, 201, "mario falls according to SPEED_V[23]");

    // Frames 9..=11: charge increments from 9 up to 11 while falling
    for expected_charge in 9..=11 {
        sim.step(
            &PlayerKeys {
                a: true,
                ..Default::default()
            },
            false,
            false,
            &layout,
        );
        assert_eq!(sim.flutter_charge, expected_charge);
        assert_eq!(sim.flutter, 0);
    }

    // Frame 12: 12th charge frame triggers re-flutter!
    let y_before = sim.y;
    sim.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        false,
        false,
        &layout,
    );
    assert_eq!(sim.flutter_charge, 0, "charge counter resets to 0");
    assert_eq!(
        sim.flutter, 7,
        "flutter resets to 8, decremented to 7 in gravity"
    );
    assert_eq!(sim.vvel, 24, "vvel resets to 24");
    assert_eq!(sim.y, y_before, "gravity bypassed on re-flutter frame");
}

#[test]
fn pan_timer_blocks_flutter_arming_and_charging() {
    let layout = blank_layout();
    let mut sim = small_sim();
    sim.powerup = 2;
    sim.on_ground = false;
    sim.in_air = true;
    sim.vvel = 24;
    sim.pan_timer = 2;

    // Try to trigger flutter with pan_timer != 0
    sim.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        true, // a_pressed
        false,
        &layout,
    );
    assert_eq!(sim.flutter, 0, "flutter not armed when pan_timer != 0");
    assert!(!sim.flutter_armed);
    assert_eq!(sim.flutter_charge, 0);
    assert_eq!(sim.vvel, 23, "gravity ran normally");

    // Try to charge flutter with pan_timer != 0
    sim.flutter_armed = true;
    sim.flutter_charge = 5;
    sim.pan_timer = 1;
    sim.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        false,
        false,
        &layout,
    );
    assert_eq!(
        sim.flutter_charge, 5,
        "charge cannot accumulate when pan_timer != 0"
    );
}

#[test]
fn flutter_gated_by_goal_bell_moon_and_vvel_threshold() {
    let layout = blank_layout();

    // 1. Gated by goal bell
    let mut sim_bell = small_sim();
    sim_bell.powerup = 2;
    sim_bell.on_ground = false;
    sim_bell.in_air = true;
    sim_bell.vvel = 24;
    sim_bell.goal_bell = true;
    sim_bell.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        true,
        false,
        &layout,
    );
    assert_eq!(sim_bell.flutter, 0, "goal bell blocks flutter");

    // 2. Gated by moon physics
    let mut sim_moon = small_sim();
    sim_moon.powerup = 2;
    sim_moon.on_ground = false;
    sim_moon.in_air = true;
    sim_moon.vvel = 24;
    sim_moon.moon = 1;
    sim_moon.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        true,
        false,
        &layout,
    );
    assert_eq!(sim_moon.flutter, 0, "moon physics blocks flutter");

    // 3. Gated by ascending vvel >= 0x48
    let mut sim_asc = small_sim();
    sim_asc.powerup = 2;
    sim_asc.on_ground = false;
    sim_asc.in_air = true;
    sim_asc.vvel = 75; // ascending jump
    sim_asc.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        true,
        false,
        &layout,
    );
    assert_eq!(sim_asc.flutter, 0, "ascending vvel >= 72 blocks flutter");

    // 4. Non-bunny powerup (e.g. fire/super)
    let mut sim_fire = small_sim();
    sim_fire.powerup = 3;
    sim_fire.on_ground = false;
    sim_fire.in_air = true;
    sim_fire.vvel = 24;
    sim_fire.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        true,
        false,
        &layout,
    );
    assert_eq!(sim_fire.flutter, 0, "non-bunny cannot flutter");
}

#[test]
fn flutter_charge_persists_across_release_landing_and_jump() {
    let layout = blank_layout();
    let mut sim = small_sim();
    sim.powerup = 2;
    sim.on_ground = false;
    sim.in_air = true;
    sim.flutter_armed = true;
    sim.flutter_charge = 5;

    // `$10EB`: releasing A leaves the counter where it is.
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(sim.flutter_charge, 5, "releasing A keeps the charge");
    assert!(sim.flutter_armed);

    // Landing does not write `$A27C` either.
    sim.flutter_charge = 7;
    sim.flutter_armed = false;
    sim.on_ground = true;
    sim.on_ground_obj = true;
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(sim.flutter_charge, 7, "ground contact keeps the charge");

    // `$1675` clears the armed latch and leaves the charge.
    sim.flutter_armed = true;
    sim.flutter_charge = 11;
    sim.on_ground = true;
    sim.on_ground_obj = true;
    sim.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        true,
        false,
        &layout,
    );
    assert!(!sim.flutter_armed, "ground jump clears flutter_armed");
    assert_eq!(sim.flutter_charge, 11, "ground jump keeps flutter_charge");
}

/// Level 10 input 9733: charge is already 11 and disarmed, so the A press
/// starts an 8-frame hover and the next held frame immediately restarts it.
#[test]
fn flutter_charge_of_11_retriggers_on_the_frame_after_the_press() {
    let layout = blank_layout();
    let mut sim = small_sim();
    sim.powerup = 2;
    sim.on_ground = false;
    sim.in_air = true;
    sim.vvel = 0x0B;
    sim.flutter_armed = false;
    sim.flutter_charge = 11;

    sim.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        true,
        false,
        &layout,
    );
    assert_eq!(sim.flutter, 7);
    assert_eq!(sim.flutter_charge, 11, "the press does not increment charge");
    assert_eq!(sim.vvel, 24);
    assert_eq!(sim.y, 200);

    sim.step(
        &PlayerKeys {
            a: true,
            ..Default::default()
        },
        false,
        false,
        &layout,
    );
    assert_eq!(sim.flutter_charge, 0, "11 + 1 reaches the restart");
    assert_eq!(sim.flutter, 7, "timer reloads to 8 and gravity decrements it");
    assert_eq!(sim.vvel, 24);
    assert_eq!(sim.y, 200, "the restarted hover does not fall");
}

#[test]
fn powerup_loss_keeps_flutter_state() {
    // `$319F` drops a tier and arms invincibility. It never writes
    // `sFlutterTimer`, `sFlutterArmed`, or `sFlutterCharge`.
    let mut sim = small_sim();
    sim.powerup = 2;
    sim.flutter = 5;
    sim.flutter_armed = true;
    sim.flutter_charge = 8;
    sim.dead = 15;
    sim.death_dispatch();
    assert_eq!(sim.powerup, 1);
    assert_eq!(sim.flutter, 5);
    assert!(sim.flutter_armed);
    assert_eq!(sim.flutter_charge, 8);
    assert_eq!(sim.invincibility, 128);
}

#[test]
fn side_pipe_entry_supports_both_right_and_left_facing_pipes() {
    let mut sim = small_sim();
    sim.on_ground = true;
    sim.pipe_dir = 3;

    // Pressing RIGHT enters rightward travel ($30)
    let keys_right = PlayerKeys {
        right: true,
        ..Default::default()
    };
    sim.pipe_entry(&keys_right);
    assert_eq!(
        sim.pipe_dir, 0x30,
        "pressing right on side pipe mouth enters $30"
    );
    assert_eq!(sim.sfx0, 5, "pipe entry sfx requested");
    assert_eq!(sim.hvel, 0x80);

    // Reset and test pressing LEFT enters leftward travel ($40)
    sim.pipe_dir = 3;
    sim.sfx0 = 0;
    let keys_left = PlayerKeys {
        left: true,
        ..Default::default()
    };
    sim.pipe_entry(&keys_left);
    assert_eq!(
        sim.pipe_dir, 0x40,
        "pressing left on side pipe mouth enters $40"
    );
    assert_eq!(sim.sfx0, 5, "pipe entry sfx requested");
    assert_eq!(sim.hvel, 0x80);

    // Neither direction held -> no entry
    sim.pipe_dir = 3;
    sim.sfx0 = 0;
    sim.pipe_entry(&PlayerKeys::default());
    assert_eq!(
        sim.pipe_dir, 3,
        "idle Mario stays armed at 3 without entering"
    );

    // Airborne Mario cannot enter pipe with button press
    sim.on_ground = false;
    sim.pipe_entry(&keys_right);
    assert_eq!(sim.pipe_dir, 3, "airborne Mario cannot enter right pipe");
    sim.pipe_entry(&keys_left);
    assert_eq!(sim.pipe_dir, 3, "airborne Mario cannot enter left pipe");

    // Side push conveyor force enters without key
    sim.on_ground = true;
    sim.side_push_dir = 0xFF;
    sim.pipe_entry(&PlayerKeys::default());
    assert_eq!(sim.pipe_dir, 0x40, "side push 0xFF enters $40 left travel");

    sim.pipe_dir = 3;
    sim.side_push_dir = 1;
    sim.pipe_entry(&PlayerKeys::default());
    assert_eq!(sim.pipe_dir, 0x30, "side push 1 enters $30 right travel");
}

#[test]
fn pipe_travel_horizontal_sets_x_sub_for_camera_follow() {
    let mut layout_bytes = vec![0x60; 256 * 48];
    // Surround x=100 with wall tiles at x=90..110 so pipe travel remains active
    for y in 0..48 {
        for x in 0..16 {
            layout_bytes[y * 256 + x] = 0x18; // solid wall
        }
    }
    let layout = LevelLayout::from_bytes(&layout_bytes).unwrap();

    let mut sim = small_sim();
    sim.x = 0x0080;
    sim.y = 0x0080;
    sim.pipe_dir = 0x30;
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(sim.x, 0x0082, "pipe travel 0x30 moves +2");
    assert_eq!(
        sim.x_sub, 1,
        "pipe travel 0x30 sets x_sub = 1 for camera follow"
    );

    sim.x = 0x0080;
    sim.pipe_dir = 0x40;
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(sim.x, 0x007E, "pipe travel 0x40 moves -2");
    assert_eq!(
        sim.x_sub, 0xFF,
        "pipe travel 0x40 sets x_sub = 0xFF for camera follow"
    );
}

/// Fast unit coverage for the coin touch, the level-map overlay and the BCD
/// credit — the semantics the `traj16` replay verifies against the console,
/// pinned here so a regression fails in `cargo test` without PyBoy.
/// The flag reaches the sim from the object pass the way the sfx/fade bytes
/// do (`$9285` -> `$A2E3` -> this).
#[test]
fn the_boss_clear_flag_crosses_from_the_object_pass() {
    let mut sim = PlayerSim::new();
    let mut obj = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj.boss_clear_flag = 1;
    sim.apply_object_result(&mut obj, false);
    assert!(sim.boss_clear);
}

/// `$1637`..`$1674`: the jump requests its own sound. Plain jumps use SFX 1;
/// moon physics swaps in 18 (3 inside a bit-1 zone), swimming uses 3, and
/// holding UP cancels the request for everyone but Bunny Mario.
#[test]
fn the_jump_arm_requests_sfx_1_with_the_moon_and_swim_variants() {
    let layout = blank_layout();
    let press = PlayerKeys {
        a: true,
        ..Default::default()
    };
    let grounded = || {
        let mut sim = small_sim();
        // The blank map is air, so the landing probe would clear `on_ground`
        // before `$160A`; standing on an object keeps the jump arm reachable.
        sim.on_ground_obj = true;
        sim
    };
    let mut sim = grounded();
    sim.step(&press, true, false, &layout);
    assert_eq!(sim.sfx0, 1, "$164C: the plain jump sound");

    // Moon physics bit 1 (the in-zone re-launch) uses SFX 3. The zone tile
    // under the `$1CFB` sample keeps bit 1 from being cleared by the probe.
    let mut moon_layout = blank_layout();
    moon_layout.cells[EXIT_CELL] = 0x54;
    let mut moon3 = grounded();
    moon3.moon = 0x02;
    moon3.step(&press, true, false, &moon_layout);
    assert_eq!(moon3.sfx0, 3, "$165F: bit-1 moon zone");

    // Any other nonzero moon nibble uses SFX 18.
    let mut moon18 = grounded();
    moon18.moon = 0x01;
    moon18.step(&press, true, false, &layout);
    assert_eq!(moon18.sfx0, 18, "$164C: the moon jump");

    // Swimming (and not on the ground) also uses SFX 3. `$1BFF` clears the
    // flag first each frame, so the `$4C` water tile has to be under the probe.
    let mut water = blank_layout();
    water.cells[EXIT_CELL] = 0x4C;
    let mut swim = small_sim();
    swim.on_ground = false;
    swim.step(&press, true, false, &water);
    assert_eq!(swim.sfx0, 3, "$1659: swimming");

    // UP held silences a normal jump, but Bunny Mario keeps it.
    let up_press = PlayerKeys {
        a: true,
        up: true,
        ..Default::default()
    };
    let mut up = grounded();
    up.step(&up_press, true, false, &layout);
    assert_eq!(up.sfx0, 0, "$1674: UP cancels the jump sound");

    let mut bunny = grounded();
    bunny.powerup = 2;
    bunny.step(&up_press, true, false, &layout);
    assert_eq!(bunny.sfx0, 1, "Bunny Mario skips the UP cancel");
}

/// `$5384` / `$53FE`: the walk frame selection also re-requests SFX 4 while
/// Mario moves in the direction he faces; the slot-0 busy gate turns the
/// per-frame request into the walk patter. Standing and skidding are silent.
#[test]
fn walking_requests_sfx_4_from_the_frame_selection() {
    let mut sim = PlayerSim::new();
    sim.on_ground = true;
    sim.dir_right = true;
    sim.hvel = 0x50;
    assert_eq!(sim.frame_index(0), 13, "$5384: the walk-right frame");
    assert_eq!(sim.sfx0, 4, "the walk-right step");

    let mut sim = PlayerSim::new();
    sim.on_ground = true;
    sim.dir_right = false;
    sim.hvel = 0xB0;
    assert_eq!(sim.frame_index(0), 12, "$53FE: the walk-left frame");
    assert_eq!(sim.sfx0, 4, "the walk-left step");

    let mut standing = PlayerSim::new();
    standing.on_ground = true;
    standing.dir_right = true;
    standing.hvel = 0x80;
    assert_eq!(standing.frame_index(0), 0);
    assert_eq!(standing.sfx0, 0, "standing requests nothing");

    let mut skid = PlayerSim::new();
    skid.on_ground = true;
    skid.dir_right = true;
    skid.hvel = 0xB0;
    assert_eq!(skid.frame_index(0), 1, "the skid frame");
    assert_eq!(skid.sfx0, 0, "a skid is not a step");
}

/// $2E8E: harmful object contact sets sMarioDead = 15, and requests SFX 20
/// (power-down) if Mario is powered up, or SFX 15 (death) if small.
#[test]
fn harm_or_star_emits_powerdown_sfx_for_powered_mario_and_death_sfx_for_small_mario() {
    let mut obj = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj.slots[0].b[crate::objects::B_TYPE] = 0x40; // harmful
    obj.mario.powerup = 1; // Big Mario
    obj.harm_or_star(0);
    assert_eq!(obj.mario_dead, 15);
    assert_eq!(obj.sfx0, 20, "powered Mario emits SFX 20 (power-down)");

    let mut obj_small = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj_small.slots[0].b[crate::objects::B_TYPE] = 0x40;
    obj_small.mario.powerup = 0; // Small Mario
    obj_small.harm_or_star(0);
    assert_eq!(obj_small.mario_dead, 15);
    assert_eq!(obj_small.sfx0, 15, "small Mario emits SFX 15 (death)");

    let mut obj_moon = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj_moon.slots[0].b[crate::objects::B_TYPE] = 0x40;
    obj_moon.mario.moon_physics = 0x04; // bit 2 shield
    obj_moon.harm_or_star(0);
    assert_eq!(obj_moon.mario.moon_physics & 0x04, 0, "shield consumed");
    assert_eq!(obj_moon.mario.invincibility, 0x80);
    assert_eq!(obj_moon.mario_dead, 0);
    assert_eq!(obj_moon.sfx0, 0);

    let mut obj_inv = crate::objects::ObjectSim::new(Vec::new(), 0);
    obj_inv.slots[0].b[crate::objects::B_TYPE] = 0x40;
    obj_inv.mario.invincibility = 64;
    obj_inv.harm_or_star(0);
    assert_eq!(obj_inv.mario_dead, 0);
    assert_eq!(obj_inv.sfx0, 0);
}

/// $5447: when powered-down to small Mario (powerup = 0), the 128 frames of
/// invincibility alternate between Small Mario (+32) when bit 2 of the timer is
/// set, and Big Mario (+0) when bit 2 is clear. Once the timer reaches 0, Small
/// Mario frame (+32) remains active.
#[test]
fn power_down_invincibility_flicker_alternates_small_and_big_mario_frames() {
    let mut sim = PlayerSim::new();
    sim.powerup = 0;
    sim.invincibility = 128;
    sim.on_ground = true;
    sim.dir_right = true;
    sim.hvel = 0x80; // standing pose base frame = 0

    for frame in 0..128 {
        let expected_inv_after_dec = 127 - frame;
        let expected_frame = if expected_inv_after_dec & 0x04 != 0 {
            32 // Small Mario frame
        } else {
            0 // Big Mario frame
        };
        let actual = sim.frame_index(0);
        assert_eq!(
            actual, expected_frame,
            "frame {frame}: expected frame {expected_frame} (inv after dec = {expected_inv_after_dec})"
        );
    }
    assert_eq!(sim.invincibility, 0);
    // After invincibility has fully expired, Mario stays Small Mario (+32)
    assert_eq!(sim.frame_index(0), 32);
}

/// $1C4E: spike hazard damage requests SFX 20 and initiates power-down for
/// Big Mario, transitioning powerup to 0 and invincibility to 128.
#[test]
fn spike_hazard_power_down_emits_sfx_and_animates() {
    let mut layout = blank_layout();
    layout.cells[12 * 256 + 6] = 0x50; // a spike at the big-only sample
    let mut sim = small_sim();
    sim.powerup = 1; // Big Mario
    let keys = PlayerKeys::default();

    sim.step(&keys, false, false, &layout);
    assert_eq!(sim.sfx0, 20, "emitted SFX 20 on damage from spikes");
    assert_eq!(sim.powerup, 0, "powerup lost");
    assert_eq!(sim.invincibility, 128, "granted 128 frames of invincibility");

    // First frame of invincibility: decrements to 127 (bit 2 is 1) -> Small Mario (base 10 + 32 = 42)
    assert_eq!(sim.frame_index(0), 42);

    // After 4 frames (inv = 123, bit 2 is 0) -> Big Mario (base 10 + 0 = 10)
    for _ in 0..3 {
        sim.frame_index(0);
    }
    assert_eq!(sim.invincibility, 124);
    assert_eq!(sim.frame_index(0), 10, "alternates back to Big Mario frame");
}

/// $3186 requests song 2 only when the death arc enters screen band $B0.
#[test]
fn death_arc_requests_jingle_on_fade_entry() {
    let mut sim = PlayerSim::new();
    sim.dead = 0xFF;
    sim.vvel = 1; // MarioSpeedTableDead[1] = 3.
    sim.y = 0x014A;
    sim.powerup = 3;
    sim.invincibility = 64;

    sim.death_arc(0x0100);
    assert_eq!(sim.y, 0x014D); // screen Y = $AF: still in the arc.
    assert!(!sim.dead_fade);
    assert_eq!(sim.music_request, 0);

    sim.death_arc(0x0100);
    assert_eq!(sim.y, 0x0150); // screen Y = $B2: enter $3186.
    assert!(sim.dead_fade);
    assert_eq!(sim.music_request, 2);
    assert_eq!(sim.fade_timer.to_le_bytes(), [0xC0, 0]);
    assert_eq!([sim.powerup, sim.invincibility], [0, 0]);
}

