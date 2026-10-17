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
    assert_eq!(sim.level_progress[29] & 0x80, 0x80, "coin 6 bit set at offset 29");
    assert_eq!(sim.golden_coin_count(), 1);

    let handoff = sim.map_handoff(&nodes);
    assert_eq!(handoff.node, 0x53);
    assert_eq!(handoff.screen, Some(6), "Mario Zone screen 6");
    assert_eq!(handoff.saves, 2, "saved to SRAM (progress + tail)");
    assert!(sim.map_handoff_done);
}

#[test]
fn boss_clear_fade_timer_expiration_also_triggers_boss_handoff() {
    let mut sim = dirty_player();
    sim.boss_clear = true;
    sim.fade_timer = 0; // 640-frame fade expired
    sim.map_node = 0x53;

    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(13);
    obj.boss_def_index = 2; // active boss fight

    sim.apply_object_result(&mut obj, false);
    assert!(sim.exit_phase);
    assert!(sim.exit_completed);
    assert_eq!(sim.level_progress[29] & 0x80, 0x80);
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
