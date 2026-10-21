use super::*;
use crate::level::{LevelLayout, LAYOUT_SIZE};
use crate::level_timer::LevelTimer;
use crate::objects::ObjectSim;

fn dirty_player() -> PlayerSim {
    let mut sim = PlayerSim::new();
    sim.x = 100;
    sim.y = 100;
    sim.music_request = 24;
    sim.fade_timer = 128;
    sim.sfx0 = 0xA5;
    sim.sfx1 = 0x5A;
    sim.sfx2 = 0x33;
    sim.low_time_flag = 0x77;
    sim.secret_route = 0x55;
    sim.tile_anim_config = 0x66;
    sim.level_timer = LevelTimer {
        high: 1,
        low: 0,
        tick_counter: 1,
    };
    sim
}

#[test]
fn boss_clear_level_reload_resumes_countdown_without_replaying_reward() {
    let mut sim = dirty_player();
    sim.level_tally = 5;
    sim.boss_clear = true;
    sim.level_progress[23] = 0x80;
    let progress = sim.level_progress;
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.boss_clear_flag = 1;
    obj.boss_clear_handoff = true;
    sim.respawn(36, 436);
    obj.reset_level_start();
    assert!(!sim.boss_clear);
    assert_eq!(obj.boss_clear_flag, 0);
    assert!(!obj.boss_clear_handoff);
    sim.apply_object_result(&mut obj, false);
    assert!(!sim.boss_clear);
    assert!(!sim.exit_completed);
    assert_eq!(sim.level_tally, 5);
    assert_eq!(sim.level_progress, progress);
    sim.update_level_timer();
    assert_eq!((sim.level_timer.high, sim.level_timer.low), (0, 0x99));
}

/// `$5D6E..$5D77` reads the flag only to skip countdown writes. The
/// `$5E12` continuation checks boss phase, not this flag; it is no bell exit.
#[test]
fn boss_clear_freezes_countdown_without_fabricating_a_bell_exit() {
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    let mut sim = dirty_player();
    sim.boss_clear = true;
    let timer = sim.level_timer;
    sim.update_level_timer();
    assert_eq!(sim.level_timer, timer);
    assert_eq!(sim.environment_probe(&layout, &PlayerKeys::default()), None);
    assert_eq!(
        (sim.music_request, sim.fade_timer, sim.exit_phase),
        (24, 128, false)
    );
    assert_eq!((sim.sfx0, sim.sfx1, sim.sfx2), (0xA5, 0x5A, 0x33));
    assert_eq!(
        (sim.low_time_flag, sim.secret_route, sim.tile_anim_config),
        (0x77, 0x55, 0x66)
    );
    assert!(!sim.goal_bell);
}

/// Object effects reach the same live flag used by the frame-head countdown.
/// An empty environment must still permit the ordinary player frame to run.
#[test]
fn object_boss_clear_transfer_preserves_normal_player_frames() {
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    let mut sim = dirty_player();
    let mut control = dirty_player();
    let mut objects = ObjectSim::new(Vec::new(), 0);
    objects.boss_clear_flag = 1;
    sim.apply_object_result(&mut objects, false);
    assert!(sim.boss_clear);
    let timer = sim.level_timer;
    let keys = PlayerKeys {
        right: true,
        ..PlayerKeys::default()
    };
    for _ in 0..3 {
        sim.update_level_timer();
        sim.step(&keys, false, false, &layout);
        control.step(&keys, false, false, &layout);
        assert_eq!(sim.level_timer, timer);
        assert_eq!(
            (sim.music_request, sim.fade_timer, sim.exit_phase),
            (
                control.music_request,
                control.fade_timer,
                control.exit_phase
            )
        );
        assert_eq!(
            (sim.x, sim.y, sim.hvel, sim.vvel, sim.phase),
            (
                control.x,
                control.y,
                control.hvel,
                control.vvel,
                control.phase
            )
        );
    }
    assert_eq!(sim.fade_timer, 125);
    assert_eq!(sim.music_request, 24);
    assert!(!sim.exit_phase);
}

#[test]
fn boss_clear_handoff_transitions_to_exit_completed_and_saves_coin() {
    let nodes = crate::map::MapNodes::from_bytes(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/map/nodes.bin"
        ))
        .unwrap(),
    )
    .unwrap();

    let mut sim = dirty_player();
    sim.map_node = 0x53; // Mario Zone boss node
    sim.level_tally = 0x05;
    assert_eq!(sim.level_progress[29], 0);

    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(13); // Mario Zone boss level
    obj.boss_clear_handoff = true;

    sim.apply_object_result(&mut obj, false);
    assert!(!obj.boss_clear_handoff, "consumed by player sim");
    assert!(sim.exit_phase, "level exit phase armed");
    assert!(sim.exit_completed, "handoff completed");
    assert_eq!(sim.music_request, 15, "ROM line 16765 boss cutscene track");
    assert_eq!(sim.level_tally, 0x06, "BCD tally incremented");
    assert_eq!(
        sim.level_progress[29] & 0x80,
        0x80,
        "coin 6 bit set at offset 29"
    );
    assert_eq!(sim.golden_coin_count(), 1);

    sim.apply_object_result(&mut obj, false);
    assert_eq!(
        sim.level_tally, 0x06,
        "consumed reward is applied only once"
    );
    assert_eq!(sim.golden_coin_count(), 1);

    let handoff = sim.map_handoff(&nodes);
    assert_eq!(handoff.node, 0x1D);
    assert_eq!(handoff.screen, Some(0), "boss return goes to the hub");
    assert_eq!(handoff.cursor_move, Some(2));
    assert_eq!(handoff.saves, 1, "hub branch skips the ordinary tail save");
    assert!(sim.map_handoff_done);
}

#[test]
fn live_boss_returns_use_the_hub_branch_before_the_zone_bands() {
    let nodes = crate::map::MapNodes::from_bytes(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/map/nodes.bin"
    )))
    .unwrap();
    // `$3EFD8..$3F000` progress slots and `$3C6BD..$3C77C` hub targets.
    for (node, progress, hub, direction, zone) in [
        (0x20, 5, 0x19, 8, 2),
        (0x2B, 9, 0x1C, 8, 1),
        (0x35, 14, 0x1B, 34, 3),
        (0x3D, 17, 0x1A, 2, 4),
        (0x49, 23, 0x1E, 4, 5),
        (0x53, 29, 0x1D, 2, 6),
    ] {
        assert_eq!(nodes.progress_slot(node), Some(progress as u8));
        let mut sim = PlayerSim::new();
        sim.map_node = node;
        sim.level_progress[progress] = 8;
        let out = sim.map_handoff(&nodes);
        assert_eq!((sim.map_node, out.node, out.screen), (hub, hub, Some(0)));
        assert_eq!(out.cursor_move, Some(direction));
        assert_eq!((out.music, out.saves), (Some(6), 1));
        assert_eq!(sim.level_progress[progress], 0x80);

        // `$3EFDA` skips all six branches for a dead Mario.
        let mut dead = PlayerSim::new();
        dead.map_node = node;
        dead.dead = 0xFF;
        let out = dead.map_handoff(&nodes);
        assert_eq!((out.node, out.screen, out.cursor_move), (node, Some(zone), None));
        assert_eq!(dead.level_progress[progress], 0);
    }
}

#[test]
fn boss_clear_flag_with_expired_fade_preserves_wario_ending() {
    for definition in [6, 7, 8] {
        assert_flag_only_preserves_player(definition);
    }
}

#[test]
fn boss_clear_flag_with_expired_fade_does_not_award_normal_boss_coin() {
    for definition in 1..=5 {
        assert_flag_only_preserves_player(definition);
    }
}

/// `$5D6E` suppresses countdown; `$5E12` tests fight phase. Neither creates
/// the explicit terminal reward event emitted by Mario Zone's reward actor.
fn assert_flag_only_preserves_player(definition: u8) {
    let mut sim = dirty_player();
    sim.fade_timer = 0;
    sim.map_node = 0x53;
    sim.level_tally = 0x05;
    let progress = sim.level_progress;
    let timer = sim.level_timer;

    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(13);
    obj.boss_def_index = definition;
    obj.boss_clear_flag = 1;

    for _ in 0..2 {
        sim.apply_object_result(&mut obj, false);
        assert!(sim.boss_clear);
        assert_eq!(sim.music_request, 24, "definition {definition}");
        assert_eq!(sim.fade_timer, 0);
        assert!(!sim.exit_phase, "definition {definition}");
        assert!(!sim.exit_completed);
        assert_eq!(sim.level_tally, 0x05);
        assert_eq!(sim.level_progress, progress);
        assert_eq!(sim.golden_coin_count(), 0);
        assert_eq!(sim.tile_anim_config, 0x66);
        assert!(!obj.boss_clear_handoff);
        sim.update_level_timer();
        assert_eq!(sim.level_timer, timer);
    }
}

#[test]
fn boss_clear_sixth_coin_requests_castle_fanfare() {
    let mut sim = dirty_player();
    // Pre-populate 5 other golden coins: 5, 9, 23, 17, 14
    for off in [5, 9, 23, 17, 14] {
        sim.level_progress[off] = 0x80;
    }
    assert_eq!(sim.golden_coin_count(), 5);

    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(13);
    obj.boss_clear_handoff = true;

    sim.apply_object_result(&mut obj, false);
    assert_eq!(sim.golden_coin_count(), 6);
    assert_eq!(sim.music_request, 26, "ROM line 16771 castle fanfare track");
}
