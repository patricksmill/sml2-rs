use std::path::PathBuf;

use sml2_core::{map::save_record_valid, player_sim::PlayerKeys};
use sml2_runtime::{
    content::{ExtractedContent, SessionAssets},
    session::{FrameInput, Session, SessionState, StepOutcome},
};

fn new_session() -> Session {
    let source =
        ExtractedContent::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted"));
    Session::new(SessionAssets::load_from(&source).unwrap())
}

#[test]
fn advance_emits_save_bytes_and_consumes_the_supplied_div_sample() {
    let mut session = new_session();
    session.lives = 7;
    session.level_tally = 2;
    session.money_bag = 9;
    let SessionState::Overworld(map) = &mut session.state else {
        panic!("expected map")
    };
    map.coins_low = 0x23;
    map.coins_high = 1;
    map.pending_save = true;
    map.tick_timer = 1; // The next bookkeeping pass consumes DIV into the RNG latch.

    let effects = session.advance(FrameInput {
        div: 0x5A,
        ..Default::default()
    });
    assert_eq!(effects.outcome, StepOutcome::Continue);
    assert!(effects.diagnostics.is_empty());
    assert_eq!(effects.saves.len(), 1);
    assert!(effects.persist_slots);
    let record = &effects.saves[0];
    assert!(save_record_valid(record));
    assert_eq!(&record[0x40..0x45], &[0x23, 1, 2, 7, 9]);
    assert_eq!(&record[0x46..0x4A], &[0x12, 0x34, 0x56, 0x78]);
    let SessionState::Overworld(map) = &session.state else {
        panic!("expected map")
    };
    assert_eq!(map.rand_latch, 0x5A);
    assert!(!map.pending_save);
    let idle = session.advance(FrameInput::default());
    assert!(idle.saves.is_empty());
    assert!(!idle.persist_slots);

    // A different host may persist the effect or immediately restore it in memory.
    let mut restored = new_session();
    assert!(restored.apply_save_record(record));
    assert_eq!(&restored.save_record()[0x40..0x4A], &record[0x40..0x4A]);
}

#[test]
fn gameplay_and_sound_sequencing_are_independent_of_render_frequency_and_size() {
    let mut plain = new_session();
    let mut displayed = new_session();
    for tick in 0..120u8 {
        let input = FrameInput {
            div: tick.wrapping_mul(17),
            keys: PlayerKeys {
                a: tick == 0 || (70..80).contains(&tick),
                right: tick > 40,
                ..Default::default()
            },
        };
        let a = plain.advance(input);
        let b = displayed.advance(input);
        // Presentation can run repeatedly at a different size between ticks.
        if tick % 11 == 0 {
            displayed.render_frame((320, 224));
            displayed.render_frame((160, 144));
        }
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.saves, b.saves);
        assert_eq!(a.persist_slots, b.persist_slots);
        assert_eq!(a.diagnostics, b.diagnostics);
        assert_eq!(plain.sound_driver, displayed.sound_driver);
        assert_eq!(plain.save_record(), displayed.save_record());
        match (&plain.state, &displayed.state) {
            (SessionState::Overworld(a), SessionState::Overworld(b)) => assert_eq!(a, b),
            (SessionState::InLevel(a), SessionState::InLevel(b)) => {
                assert_eq!(format!("{:?}", a.sim), format!("{:?}", b.sim));
                assert_eq!(a.layout_live.cells, b.layout_live.cells);
                for (a, b) in a.obj.slots.iter().zip(&b.obj.slots) {
                    assert_eq!(a.b, b.b);
                }
            }
            _ => panic!("session state diverged"),
        }
    }
    assert!(matches!(plain.state, SessionState::InLevel(_)));
    assert_eq!(
        plain.render_frame((160, 144)),
        displayed.render_frame((160, 144))
    );
}
