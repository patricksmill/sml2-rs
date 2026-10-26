use super::*;
use sml2_core::player_sim::PlayerKeys;

fn tick(session: &mut Session, right: bool) {
    let effects = session.advance(FrameInput {
        keys: PlayerKeys {
            right,
            ..Default::default()
        },
        div: 0,
    });
    assert!(
        effects.saves.is_empty(),
        "the character door never saves a slot"
    );
    assert!(effects.diagnostics.is_empty());
    assert!(!effects.persist_slots);
}

fn menu(session: &Session) -> &FileSelect {
    let SessionState::FileSelect(file_select) = &session.state else {
        panic!("character selection must stay on the file-select screen")
    };
    file_select
}

#[test]
fn file_select_character_door_updates_character_and_sound_without_saving() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    assert_eq!(session.character, Character::Mario);
    session.enter_file_select();
    if let SessionState::FileSelect(file_select) = &mut session.state {
        file_select.set_character_door_enabled(true);
    }
    let slots = session.slots.image();
    for _ in 0..64 {
        if menu(&session).menu_state() == 1 {
            break;
        }
        tick(&mut session, false);
    }
    assert_eq!(menu(&session).menu_state(), 1);
    for _ in 0..3 {
        tick(&mut session, true);
        for _ in 0..32 {
            tick(&mut session, false);
        }
    }
    assert_eq!(menu(&session).cursor(), (0x88, 0x7C));

    for expected in [Character::Luigi, Character::Mario] {
        tick(&mut session, true);
        for _ in 0..64 {
            tick(&mut session, false);
        }
        assert_eq!(menu(&session).cursor(), (0xC8, 0x7C));
        assert_ne!(
            session.character, expected,
            "arrival runs the following frame"
        );
        tick(&mut session, false);
        assert_eq!(session.character, expected);
        assert_eq!(session.sound_driver.pulse_effect.request, 0x0E);
        assert_eq!(
            (menu(&session).menu_state(), menu(&session).timer()),
            (3, 64)
        );
        for _ in 0..64 {
            tick(&mut session, false);
        }
        assert_eq!(menu(&session).cursor(), (0x88, 0x7C));
        assert_eq!(session.character, expected, "return must not toggle again");
        assert_eq!(session.slots.image(), slots);
        assert_eq!(session.active_slot, None);
    }
}
