//! Start from a persisted map position; never inject a castle state or Mario pose.
use crate::session::{FrameInput, Session, SessionAssets, SessionState, StepOutcome};
use sml2_core::castle_sequence::{GOLDEN_COIN_OFFSETS, STATE_REVEAL_B};
use sml2_core::map_sim::MapSim;
use sml2_core::player_sim::PlayerKeys;

fn step(session: &mut Session, keys: PlayerKeys) -> crate::session::FrameEffects {
    let effects = session.advance(FrameInput { keys, div: 0 });
    assert!(effects.diagnostics.is_empty(), "{:?}", effects.diagnostics);
    effects
}

fn at_castle_node(coins: usize) -> Session {
    let mut session = Session::new(SessionAssets::load().unwrap());
    let mut map = MapSim::new();
    map.node = 0x5C; // $4218[$5C] = $FF -> $3573 -> $1B
    for off in GOLDEN_COIN_OFFSETS.into_iter().take(coins) {
        map.level_progress[off] = 0x80;
    }
    map.level_progress[2] = 0x08; // unrelated secret-entry progress must survive
    map.coins_low = 0x42;
    assert!(session.apply_save_record(&map.save_record(3, 0x12, 0)));
    let SessionState::Overworld(map) = &session.state else {
        panic!("map")
    };
    assert_eq!((map.node, map.screen_id), (0x5C, 7));
    session
}

fn arrive(session: &mut Session) {
    step(
        session,
        PlayerKeys {
            a: true,
            ..Default::default()
        },
    );
    assert!(
        matches!(session.state, SessionState::Castle(_)),
        "$FF must not load a manifest"
    );
    for _ in 0..65 {
        step(session, PlayerKeys::default());
    }
    let SessionState::Castle(seq) = &session.state else {
        panic!("castle")
    };
    assert_eq!(
        (seq.state, seq.menu, seq.walk_x, seq.walk_y),
        (STATE_REVEAL_B, 2, 88, 136)
    );
}

#[test]
fn saved_map_walks_to_locked_castle_then_down_returns_and_reloads() {
    let mut session = at_castle_node(5);
    let before = session.save_record();
    arrive(&mut session);
    for keys in [
        PlayerKeys {
            a: true,
            ..Default::default()
        },
        PlayerKeys {
            b: true,
            ..Default::default()
        },
    ] {
        step(&mut session, keys);
        assert!(matches!(&session.state, SessionState::Castle(seq) if seq.menu == 2));
    }
    assert_eq!(
        session.save_record(),
        before,
        "visiting does not award a sixth coin or erase the map position"
    );
    step(
        &mut session,
        PlayerKeys {
            down: true,
            ..Default::default()
        },
    );
    let mut saved = None;
    for _ in 0..64 {
        let effects = step(&mut session, PlayerKeys::default());
        if let Some(record) = effects.saves.last() {
            saved = Some(*record);
        }
    }
    let SessionState::Overworld(map) = &session.state else {
        panic!("returned map")
    };
    assert_eq!((map.screen_id, map.node), (7, 0x5C));
    let record = saved.expect("return emits the actual saved record");
    assert_eq!(record, before);
    let mut reloaded = Session::new(SessionAssets::load().unwrap());
    assert!(reloaded.apply_save_record(&record));
    arrive(&mut reloaded);
}

#[test]
fn six_coin_map_visit_accepts_a_start_or_up_and_enters_real_wario_level() {
    for key in [
        PlayerKeys {
            a: true,
            ..Default::default()
        },
        PlayerKeys {
            start: true,
            ..Default::default()
        },
        PlayerKeys {
            up: true,
            ..Default::default()
        },
    ] {
        let mut session = at_castle_node(6);
        arrive(&mut session);
        step(
            &mut session,
            PlayerKeys {
                b: true,
                ..Default::default()
            },
        );
        assert!(matches!(&session.state, SessionState::Castle(seq) if seq.menu == 2));
        step(&mut session, key);
        for _ in 0..63 {
            step(&mut session, PlayerKeys::default());
        }
        assert!(matches!(&session.state, SessionState::Castle(seq) if seq.fade_low == 1));
        assert!(matches!(
            step(&mut session, PlayerKeys::default()).outcome,
            StepOutcome::LevelEntered(24)
        ));
        let SessionState::InLevel(ctx) = &session.state else {
            panic!("Wario level")
        };
        assert_eq!(
            (
                ctx.manifest.level_id,
                ctx.sim.map_node,
                ctx.sim.lives,
                ctx.sim.level_tally
            ),
            (24, 0x5C, 3, 0x12)
        );
        for off in GOLDEN_COIN_OFFSETS {
            assert_eq!(ctx.sim.level_progress[off], 0x80);
        }
        assert_eq!(ctx.sim.level_progress[2], 0x08);
    }
}

#[test]
fn holding_the_map_entry_button_cannot_also_confirm_the_castle_door() {
    let mut session = at_castle_node(6);
    let a = PlayerKeys {
        a: true,
        ..Default::default()
    };
    for _ in 0..90 {
        step(&mut session, a);
    }
    assert!(matches!(&session.state, SessionState::Castle(seq) if seq.menu == 2));
    step(&mut session, PlayerKeys::default());
    step(&mut session, a);
    assert!(matches!(&session.state, SessionState::Castle(seq) if seq.menu == 3));
}
