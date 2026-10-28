use super::trajectory_child::{
    update_trajectory_child, TrajectoryChildContext, TrajectoryChildEffects, HORIZONTAL_PROFILE,
    VERTICAL_PROFILE,
};
use crate::actor::{Actor, ContactKind, FrameTick};
use crate::level::LevelLayout;
use crate::objects::{
    ObjectSim, ObjectTables, Staging, B_DEF, B_FLIP, B_FSTATE, B_PHASE, B_SCRX, B_SCRY, B_SPRITE,
};

fn blank_layout() -> LevelLayout {
    LevelLayout {
        cells: vec![0; 256 * 32],
        world: crate::world::World::ROM,
    }
}

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn dirty_slot() -> [u8; 32] {
    [
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09,
    ]
}

#[test]
fn state0_initialization_direction_flip_and_unwritten_field_preservation() {
    for (flip_h, expected_state) in [(false, 1), (true, 2)] {
        let mut slot = dirty_slot();
        slot[B_FSTATE] = 0;
        slot[B_FLIP] = if flip_h { 0x20 } else { 0 };
        slot[B_SCRY] = 40;

        let mut actor = Actor::from_raw_slot(&slot);
        let result = update_trajectory_child(
            &mut actor,
            TrajectoryChildContext {
                frame: FrameTick::new(1), // odd tick: no 2-cadence advance
            },
        );

        assert_eq!(result, Ok(TrajectoryChildEffects::None));
        assert_eq!(actor.contact_kind, ContactKind::WARIO_CHILD);
        assert_eq!(actor.state, expected_state);
        assert_eq!(actor.sprite, 104);
        assert_eq!(actor.animation_frame, 0);
        assert_eq!(actor.screen.y, 24); // 40 - 16

        let out = actor.to_raw_slot();
        // Check preserved fields from dirty slot
        assert_eq!(out[0], 0x12);
        assert_eq!(out[1], 0x34);
        assert_eq!(out[7], 0xF0);
        assert_eq!(out[17], 0xAA);
    }
}

#[test]
fn every_trajectory_pair_and_restart_sentinel() {
    for state in [1, 2] {
        for phase in 0..68u8 {
            let mut actor = Actor::default();
            actor.state = state;
            actor.motion_phase = phase;
            actor.screen.x = 80;
            actor.screen.y = 100;

            let result = update_trajectory_child(
                &mut actor,
                TrajectoryChildContext {
                    frame: FrameTick::new(1),
                },
            );

            assert_eq!(result, Ok(TrajectoryChildEffects::None));
            let dx = HORIZONTAL_PROFILE[phase as usize];
            let dy = VERTICAL_PROFILE[phase as usize];
            let expected_x = if state == 1 {
                80u8.wrapping_sub(dx)
            } else {
                80u8.wrapping_add(dx)
            };
            let expected_y = 100u8.wrapping_add(dy as u8);
            assert_eq!(actor.screen.x, expected_x, "state {state} phase {phase} X");
            assert_eq!(actor.screen.y, expected_y, "state {state} phase {phase} Y");
            assert_eq!(
                actor.motion_phase,
                phase + 1,
                "state {state} phase {phase} advance"
            );
        }

        // Phase 68: sentinel 0x80 -> phase reset to 58, no movement
        let mut actor = Actor::default();
        actor.state = state;
        actor.motion_phase = 68;
        actor.screen.x = 80;
        actor.screen.y = 100;

        let result = update_trajectory_child(
            &mut actor,
            TrajectoryChildContext {
                frame: FrameTick::new(1),
            },
        );

        assert_eq!(result, Ok(TrajectoryChildEffects::None));
        assert_eq!(actor.motion_phase, 58);
        assert_eq!(actor.screen.x, 80, "restart sentinel does not move X");
        assert_eq!(actor.screen.y, 100, "restart sentinel does not move Y");
    }
}

#[test]
fn offscreen_boundary_triggers_immediate_counted_removal_before_y_and_phase() {
    for (state, start_x) in [(1, 1), (2, 174), (2, 175)] {
        let mut actor = Actor::default();
        actor.state = state;
        actor.screen.x = start_x;
        actor.screen.y = 50;

        // Phase 1 has dx = 2 in HORIZONTAL_PROFILE
        actor.motion_phase = 1;
        let dx = HORIZONTAL_PROFILE[1];
        let result = update_trajectory_child(
            &mut actor,
            TrajectoryChildContext {
                frame: FrameTick::new(1),
            },
        );

        assert_eq!(result, Ok(TrajectoryChildEffects::CountedRemove));
        let expected_x = if state == 1 {
            start_x.wrapping_sub(dx)
        } else {
            start_x.wrapping_add(dx)
        };
        assert!(expected_x >= 176);
        assert_eq!(actor.screen.x, expected_x);
        // Crucial ROM behavior: Y lookup and phase increment do NOT happen on removal!
        assert_eq!(actor.screen.y, 50, "Y untouched on counted removal");
        assert_eq!(actor.motion_phase, 1, "phase untouched on counted removal");
    }
}

#[test]
fn adapter_counted_removal_decrements_def_count_with_underflow_sets_done_and_clears_slot() {
    let mut sim = ObjectSim::new(Vec::new(), 0);
    sim.boss_def_count = 0;
    sim.done = 0;

    let mut st = Staging {
        b: dirty_slot(),
        ..Default::default()
    };
    st.b[B_FSTATE] = 2; // state 2: move right
    st.b[B_PHASE] = 0; // dx = 1
    st.b[B_SCRX] = 175; // 175 + 1 = 176 -> >= 176 triggers CountedRemove

    sim.handler_wario27(&mut st);

    assert_eq!(sim.boss_def_count, 255, "underflow wrapping 0 -> 255");
    assert_eq!(sim.done, 1, "sObjectDone set to 1");
    assert_eq!(st.b, [0u8; 32], "entire 32-byte slot cleared");
}

#[test]
fn frame_cadence_sequential_sprite_advance() {
    let mut actor = Actor::default();
    actor.state = 1;
    actor.motion_phase = 0;
    actor.sprite = 104;
    actor.animation_frame = 0;

    // Odd tick: no advance
    let _ = update_trajectory_child(
        &mut actor,
        TrajectoryChildContext {
            frame: FrameTick::new(1),
        },
    );
    assert_eq!(actor.animation_frame, 0);
    assert_eq!(actor.sprite, 104);

    // Even tick: advances
    let _ = update_trajectory_child(
        &mut actor,
        TrajectoryChildContext {
            frame: FrameTick::new(2),
        },
    );
    assert_eq!(actor.animation_frame, 1);
    assert_eq!(actor.sprite, 105);
}

#[test]
fn invalid_state_and_phase_report_diagnostic_after_animation_prefix() {
    let mut sim = ObjectSim::new(Vec::new(), 0);
    let mut st = Staging {
        b: dirty_slot(),
        ..Default::default()
    };
    st.b[B_FSTATE] = 5; // invalid state
    st.b[B_SPRITE] = 104;
    st.b[B_PHASE] = 0;
    sim.fff0 = 2; // even tick advances sprite prefix

    sim.handler_wario27(&mut st);

    assert_eq!(sim.unported.as_slice(), &[0xAFE0]);
    // Animation prefix ran before error
    assert_eq!(st.b[B_SPRITE], 105);

    // Test invalid phase >= 69
    let mut sim2 = ObjectSim::new(Vec::new(), 0);
    let mut st2 = Staging {
        b: dirty_slot(),
        ..Default::default()
    };
    st2.b[B_FSTATE] = 1;
    st2.b[B_PHASE] = 70;
    sim2.fff0 = 1;

    sim2.handler_wario27(&mut st2);
    assert_eq!(sim2.unported.as_slice(), &[0xAFE0]);
}

#[test]
fn boss_table_dispatch_and_dead_freeze() {
    let tables = tables();
    let layout = blank_layout();
    let mut sim = ObjectSim::new(Vec::new(), 0);
    sim.slots[0].b[B_DEF] = 27;
    sim.slots[0].b[B_FSTATE] = 0;
    sim.slots[0].b[B_FLIP] = 0;
    sim.slots[0].b[B_SCRY] = 60;

    // Active Mario: dispatch runs handler, state 0 -> state 1
    sim.boss_engine(80, 72, &layout, &tables);
    assert_eq!(sim.slots[0].b[B_FSTATE], 1);
    assert_eq!(sim.slots[0].b[B_SCRY], 44);
    assert!(sim.unported.is_empty());

    // Dead Mario: slot is frozen, handler does not run
    sim.mario.dead = 0xFF;
    let before = sim.slots[0].b;
    sim.boss_engine(80, 72, &layout, &tables);
    assert_eq!(sim.slots[0].b[B_FSTATE], before[B_FSTATE]);
    assert_eq!(sim.slots[0].b[B_SCRY], before[B_SCRY]);
}

#[test]
fn shared_spawn_helper_creates_child27_then_boss_driver_runs_it() {
    let tables = tables();
    let layout = blank_layout();
    let mut sim = ObjectSim::new(Vec::new(), 0);

    // Set parent in slot 0
    sim.slots[0].b[B_DEF] = 26; // Wario form 3
    sim.slots[0].b[B_SCRX] = 100;
    sim.slots[0].b[B_SCRY] = 80;

    // Stage spawn of child 27 via apply_boss_spawn
    sim.spawn_anim = 27;
    sim.spawn_sprite = 104;
    sim.spawn_attr = 0x20; // horizontal flip

    let parent_st = Staging {
        b: sim.slots[0].b,
        ..Default::default()
    };
    sim.apply_boss_spawn(&parent_st);

    // Child is spawned into slot 1
    assert_eq!(sim.slots[1].b[B_DEF], 27);
    assert_eq!(sim.slots[1].b[B_SPRITE], 104);
    assert_eq!(sim.slots[1].b[B_FSTATE], 0);

    // Clear parent in slot 0 so unported anim 26 does not flag
    sim.slots[0].b[B_DEF] = 0;

    // Now run boss_engine to let handler_wario27 execute
    sim.boss_engine(80, 72, &layout, &tables);

    // Child in slot 1 initialized: state 0 -> state 2 (flipped)
    assert_eq!(sim.slots[1].b[B_FSTATE], 2);
    assert!(sim.unported.is_empty());
}
