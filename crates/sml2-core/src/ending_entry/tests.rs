use super::*;

fn raw(k: PlayerKeys) -> u8 {
    u8::from(k.a)
        | u8::from(k.b) << 1
        | u8::from(k.select) << 2
        | u8::from(k.start) << 3
        | u8::from(k.right) << 4
        | u8::from(k.left) << 5
        | u8::from(k.up) << 6
        | u8::from(k.down) << 7
}

fn dirty() -> EndingInput {
    let all = PlayerKeys {
        a: true,
        b: true,
        start: true,
        select: true,
        right: true,
        left: true,
        up: true,
        down: true,
    };
    EndingInput {
        held: all,
        pressed: all,
    }
}

#[test]
fn source_05e12_raw_keys_boundaries_wrap_and_repeated_press() {
    for (x, y, held, pressed, state) in [
        (0, 255, 0x12, 0x00, 4),
        (159, 255, 0x12, 0x00, 4),
        (160, 255, 0x11, 0x01, 4),
        (161, 255, 0x11, 0x01, 4),
        (162, 223, 0x11, 0x00, 4),
        (162, 224, 0x11, 0x00, 35),
        (255, 255, 0x11, 0x00, 35),
        (0x100, 0x1FF, 0x12, 0x00, 4),
        (0x1A1, 0x1FF, 0x11, 0x01, 4),
        (0x1A2, 0x100, 0x11, 0x00, 4),
        (0x1A2, 0x1E0, 0x11, 0x00, 35),
    ] {
        let mut ending = EndingEntry::default();
        for _ in 0..2 {
            let EndingFrame::Gameplay(input) = ending.frame_head(&mut 3, x, y, &mut 99, dirty())
            else {
                panic!("gameplay required");
            };
            assert_eq!((raw(input.held), raw(input.pressed)), (held, pressed));
            assert_eq!(ending.state() as u8, state);
            if state == 35 {
                break;
            }
        }
    }
    for phase in [0, 1, 2, 4, 255] {
        let mut ending = EndingEntry::default();
        let EndingFrame::Gameplay(input) =
            ending.frame_head(&mut { phase }, 162, 224, &mut 0, dirty())
        else {
            panic!()
        };
        assert_eq!((raw(input.held), raw(input.pressed)), (0xFF, 0xFF));
        assert_eq!(ending.state() as u8, 4);
    }
}

#[test]
fn source_003af_palette_bytes_and_full_fade_word() {
    for (fade, expected) in [
        (0x0001, [0, 0, 0]),
        (0x003F, [0, 0, 0]),
        (0x0040, [0x50, 0x40, 0x14]),
        (0x007F, [0x50, 0x40, 0x14]),
        (0x0080, [0x94, 0x90, 0x24]),
        (0x00BF, [0x94, 0x90, 0x24]),
        (0x00C0, [0xE4, 0xD0, 0x38]),
        (0x00FF, [0xE4, 0xD0, 0x38]),
        (0x0100, [0, 0, 0]),
        (0xFFC0, [0xE4, 0xD0, 0x38]),
    ] {
        let mut ending = EndingEntry {
            state: EndingEntryState::Fade,
            ..Default::default()
        };
        let mut timer = fade;
        let EndingFrame::Fade(p) = ending.frame_head(&mut 3, 0, 0, &mut timer, dirty()) else {
            panic!()
        };
        assert_eq!([p.background, p.object0, p.object1], expected);
        assert_eq!(ending.palettes(), Some(p));
        assert_eq!((timer, ending.state() as u8), (fade, 35));
    }
}

#[test]
fn source_02cae_deferred_states_35_18_19_and_pending_sequence() {
    let mut ending = EndingEntry {
        ending_state: 0xAB,
        ..Default::default()
    };
    let mut phase = 3;
    let mut fade = 0;
    assert!(matches!(
        ending.frame_head(&mut phase, 162, 224, &mut fade, dirty()),
        EndingFrame::Gameplay(_)
    ));
    assert_eq!(
        (ending.state() as u8, fade, phase, ending.ending_state()),
        (35, 0, 3, 0xAB)
    );
    assert!(matches!(
        ending.frame_head(&mut phase, 0, 0, &mut fade, dirty()),
        EndingFrame::Fade(_)
    ));
    assert_eq!(
        (ending.state() as u8, fade, phase, ending.ending_state()),
        (18, 255, 3, 0xAB)
    );
    assert!(matches!(
        ending.frame_head(&mut phase, 0, 0, &mut fade, dirty()),
        EndingFrame::SequenceInitialized
    ));
    assert_eq!(
        (ending.state() as u8, fade, phase, ending.ending_state()),
        (19, 255, 0, 0)
    );
    assert!(matches!(
        ending.frame_head(&mut phase, 0, 0, &mut fade, dirty()),
        EndingFrame::SequencePending
    ));
}
