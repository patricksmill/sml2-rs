use super::*;

#[test]
fn literal_slot_converts_to_named_fields_and_back() {
    let slot = [
        0x12, 0x34, 0x56, 0xAB, 0xCD, 0x67, 0x89, 0xEF, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0xAE,
        0x0F, 0x10, 0x11, 0x92, 0x80, 0x7F, 0xFE, 0x02, 0xD7, 0x18, 0xE9, 0x1A, 0x1B, 0x1C, 0xED,
        0xBE, 0x1F,
    ];
    let expected = Actor {
        position: Position {
            x: 0x1234,
            y: 0xABCD,
        },
        marker: 0x56,
        definition: 0x67,
        spawn_record: 0x89EF,
        activity: 8,
        screen: ScreenPosition { x: 10, y: 9 },
        sprite: 11,
        defeat_mode: 12,
        sprite_flip: SpriteFlipFlags::from_bits(13),
        flash: FlashFlags::from_bits(0xAE),
        state: 15,
        motion_phase: 16,
        parked_contact: 17,
        contact_kind: ContactKind::from_raw(0x92),
        hitbox: Hitbox {
            top: -128,
            bottom: 127,
            left: -2,
            right: 2,
        },
        interactions: InteractionFlags::from_bits(0xD7),
        motion_leg: 24,
        travel_direction: Facing::from_raw(0xE9),
        animation_frame: 26,
        hit_reaction: 27,
        timer: 28,
        reserved29: 0xED,
        reserved30: 0xBE,
        visibility_mask: 31,
    };
    assert_eq!(Actor::from_raw_slot(&slot), expected);
    assert_eq!(expected.to_raw_slot(), slot);
}

#[test]
fn every_byte_value_in_each_slot_position_survives_conversion() {
    for position in 0..32 {
        for value in 0..=255 {
            let mut slot = [0xA5; 32];
            slot[position] = value;
            assert_eq!(Actor::from_raw_slot(&slot).to_raw_slot(), slot);
        }
    }
}

#[test]
fn signed_world_and_screen_movement_have_separate_wraps() {
    let mut world = Position { x: 0, y: u16::MAX };
    world.move_x(-1);
    world.move_y(2);
    assert_eq!(world, Position { x: 65535, y: 1 });
    world.move_x(2);
    world.move_y(-3);
    assert_eq!(world, Position { x: 1, y: 65534 });
    world.move_x(i16::MIN);
    assert_eq!(world.x, 32769);

    let mut screen = ScreenPosition { x: 0, y: 255 };
    screen.move_x(-3);
    screen.move_y(3);
    assert_eq!(screen, ScreenPosition { x: 253, y: 2 });
    screen.move_x(258);
    screen.move_y(-259);
    assert_eq!(screen, ScreenPosition { x: 255, y: 255 });
}

#[test]
fn directions_preserve_unknown_values_and_reverse_only_low_bit() {
    for value in 0..=255 {
        let mut direction = Facing::from_raw(value);
        assert_eq!(direction.is_right(), value != 0);
        assert_eq!(direction.displacement(3), if value == 0 { -3 } else { 3 });
        direction.reverse();
        assert_eq!(direction.raw(), value ^ 1);
        direction.reverse();
        assert_eq!(direction.raw(), value);
    }
    let mut actor = Actor::default();
    actor.move_screen_horizontally(3);
    assert_eq!(actor.to_raw_slot()[10], 253);
    actor.travel_direction = Facing::from_raw(0xFE);
    actor.move_screen_horizontally(3);
    assert_eq!(actor.to_raw_slot()[10], 0);
}

#[test]
fn palette_flash_preserves_other_bits_at_exact_cadence_then_clears_all() {
    for frame in 0..=255 {
        let mut actor = Actor::from_raw_slot(&[0xA5; 32]);
        actor.flash.animate_palette(FrameTick(frame));
        assert_eq!(
            actor.to_raw_slot()[14],
            if frame & 3 == 0 { 0xB5 } else { 0xA5 }
        );
        actor.flash.clear();
        assert_eq!(actor.to_raw_slot()[14], 0);
    }
}

#[test]
fn native_frame_cadence_continues_past_byte_wrap() {
    assert!(FrameTick(256).every(4));
    assert!(!FrameTick(257).every(4));
    assert!(FrameTick(264).every(8));
    assert!(FrameTick(u64::MAX).every(3));
}

#[test]
#[should_panic(expected = "frame cadence period must be positive")]
fn cadence_rejects_zero_period() {
    FrameTick(0).every(0);
}

#[test]
fn contact_kind_and_interaction_flags_are_distinct_lossless_types() {
    assert_eq!(ContactKind::NONE.raw(), 0);
    assert_eq!(ContactKind::WARIO_CHILD.raw(), 0x40);
    for value in 0..=255 {
        let kind = ContactKind::from_raw(value);
        assert_eq!(kind.raw(), value);
        assert_eq!(kind.is_item(), value & 0x80 != 0);
        let flags = InteractionFlags::from_bits(value);
        assert_eq!(flags.bits(), value);
        assert_eq!(flags.stomped(), value & 1 != 0);
        assert_eq!(flags.projectile_defeat(), value & 2 != 0);
        assert_eq!(flags.star_defeat(), value & 4 != 0);
        assert_eq!(flags.touched(), value & 0x10 != 0);
        assert_eq!(flags.struck_from_below(), value & 0x20 != 0);
    }
}

#[test]
fn sprite_flip_toggles_preserve_other_attribute_bits() {
    for value in 0..=255 {
        let mut flip = SpriteFlipFlags::from_bits(value);
        assert_eq!(flip.horizontal(), value & 0x20 != 0);
        assert_eq!(flip.vertical(), value & 0x40 != 0);
        flip.toggle_horizontal();
        assert_eq!(flip.bits(), value ^ 0x20);
        flip.toggle_vertical();
        assert_eq!(flip.bits(), value ^ 0x60);
        flip.toggle_horizontal();
        flip.toggle_vertical();
        assert_eq!(flip.bits(), value);
    }
}

#[test]
fn motion_profile_reads_before_increment_and_stops_only_at_equal_cap() {
    let mut actor = Actor::default();
    assert_eq!(actor.motion_step(&[9, 7, 5, 3], 2), Ok(9));
    assert_eq!(actor.to_raw_slot()[16], 1);
    assert_eq!(actor.motion_step(&[9, 7, 5, 3], 2), Ok(7));
    assert_eq!(actor.to_raw_slot()[16], 2);
    assert_eq!(actor.motion_step(&[9, 7, 5, 3], 2), Ok(5));
    assert_eq!(actor.to_raw_slot()[16], 2);
    actor.motion_phase = 3;
    assert_eq!(actor.motion_step(&[9, 7, 5, 3], 2), Ok(3));
    assert_eq!(actor.to_raw_slot()[16], 4);
    actor.motion_phase = 255;
    assert_eq!(actor.motion_step(&[4; 256], 2), Ok(4));
    assert_eq!(actor.to_raw_slot()[16], 0);
}

#[test]
fn malformed_motion_profiles_report_error_without_changing_state() {
    let mut actor = Actor::default();
    assert_eq!(
        actor.motion_step(&[], 0),
        Err(MotionProfileError {
            phase: 0,
            cap: 0,
            profile_len: 0
        })
    );
    assert_eq!(actor.motion_phase, 0);
    assert!(actor.motion_step(&[3], 1).is_err());
    assert_eq!(actor.motion_phase, 0);
    actor.motion_phase = 2;
    assert_eq!(
        actor.motion_step(&[3, 2], 1),
        Err(MotionProfileError {
            phase: 2,
            cap: 1,
            profile_len: 2
        })
    );
    assert_eq!(actor.motion_phase, 2);
}

#[test]
fn four_step_bob_gates_and_preserves_unrelated_slot_bytes() {
    for frame in 0..=257 {
        let mut slot = [0xA5; 32];
        slot[26] = 0;
        let mut actor = Actor::from_raw_slot(&slot);
        assert_eq!(actor.animate_four_step_bob(FrameTick(frame)), Ok(()));
        if frame % 4 == 0 {
            slot[26] = 1;
            slot[11] = 0xA6;
        }
        assert_eq!(actor.to_raw_slot(), slot, "frame {frame}");
    }
}

#[test]
fn four_step_bob_advances_before_indexing_and_wraps_sprite_both_ways() {
    // Literal ROM deltas are indexed after increment: +1, -1, +2, -2.
    for (phase, sprite, next_phase, next_sprite) in [
        (0, 0xFF, 1, 0),
        (1, 0, 2, 0xFF),
        (2, 0xFF, 3, 1),
        (3, 0, 0, 0xFE),
        (255, 0, 0, 0xFE),
    ] {
        let mut slot = [0xA5; 32];
        slot[26] = phase;
        slot[11] = sprite;
        let mut actor = Actor::from_raw_slot(&slot);
        assert_eq!(actor.animate_four_step_bob(FrameTick(256)), Ok(()));
        slot[26] = next_phase;
        slot[11] = next_sprite;
        assert_eq!(actor.to_raw_slot(), slot);
    }
}

#[test]
fn four_step_bob_reports_invalid_advanced_index_without_inventing_delta() {
    for phase in 4..=254 {
        let mut slot = [0xA5; 32];
        slot[26] = phase;
        let mut actor = Actor::from_raw_slot(&slot);
        assert_eq!(actor.animate_four_step_bob(FrameTick(1)), Ok(()));
        assert_eq!(actor.to_raw_slot(), slot);
        assert_eq!(
            actor.animate_four_step_bob(FrameTick(4)),
            Err(AnimationFrameError { frame: phase + 1 })
        );
        slot[26] = phase + 1;
        assert_eq!(actor.to_raw_slot(), slot);
    }
}
