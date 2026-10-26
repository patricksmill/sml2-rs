use super::*;
use crate::actor::InteractionFlags;

#[test]
fn test_init_and_waiting() {
    let mut actor = Actor::default();
    actor.position.x = 100;
    actor.position.y = 200;

    let eff = update(&mut actor).unwrap();
    assert_eq!(eff, RisingPlatformEffect::Initialized);
    assert_eq!(actor.contact_kind.raw(), 32);
    assert_eq!(actor.state, RisingPlatformState::Waiting.to_raw());

    // Stays waiting while Mario is not on it
    let eff2 = update(&mut actor).unwrap();
    assert_eq!(eff2, RisingPlatformEffect::Waiting);
    assert_eq!(actor.position.y, 200);

    // Mario steps on -> activates Rising state
    actor.interactions = InteractionFlags::from_bits(0x08);
    let eff3 = update(&mut actor).unwrap();
    assert_eq!(eff3, RisingPlatformEffect::Activated);
    assert_eq!(actor.state, RisingPlatformState::Rising.to_raw());
    assert_eq!(actor.motion_leg, 64);
    assert_eq!(actor.travel_span(), 64);
}

#[test]
fn test_rising_to_apex() {
    let mut actor = Actor::default();
    actor.position.y = 200;
    actor.state = RisingPlatformState::Rising.to_raw();
    actor.motion_leg = 64;
    actor.set_travel_span(64);
    actor.interactions = InteractionFlags::from_bits(0x08); // stood on

    // Rises for 64 steps
    for step in 0..64 {
        let eff = update(&mut actor).unwrap();
        assert_eq!(eff, RisingPlatformEffect::Rising { carry_dy: Some(-1) });
        assert_eq!(actor.position.y, 200 - (step + 1) as u16);
        assert_eq!(actor.motion_leg, 63 - step as u8);
    }

    assert_eq!(actor.position.y, 136);
    assert_eq!(actor.motion_leg, 0);

    // At apex, stays there
    let eff_apex = update(&mut actor).unwrap();
    assert_eq!(eff_apex, RisingPlatformEffect::ReachedApex);
    assert_eq!(actor.position.y, 136);
    assert_eq!(actor.motion_leg, 0);
}

#[test]
fn test_stepping_off_delay_and_descent() {
    let mut actor = Actor::default();
    actor.position.y = 190; // rose 10 px
    actor.state = RisingPlatformState::Rising.to_raw();
    actor.motion_leg = 54; // 64 - 10
    actor.set_travel_span(64);
    actor.interactions = InteractionFlags::from_bits(0x08);

    // Mario steps off
    actor.interactions = InteractionFlags::from_bits(0x00);
    let eff_off = update(&mut actor).unwrap();
    assert_eq!(eff_off, RisingPlatformEffect::SteppedOff);
    assert_eq!(actor.state, RisingPlatformState::Falling.to_raw());
    assert_eq!(actor.timer, 16);
    assert_eq!(actor.position.y, 190, "does not move on step-off frame");

    // 15 frames of delay (timer 16 -> 15 .. 1)
    for expected_timer in (1..16).rev() {
        let eff = update(&mut actor).unwrap();
        assert_eq!(eff, RisingPlatformEffect::Delaying);
        assert_eq!(actor.timer, expected_timer);
        assert_eq!(actor.position.y, 190);
    }
    assert_eq!(actor.timer, 1);

    // Frame 16: timer reaches 0 and first descent step happens
    let eff_first_descent = update(&mut actor).unwrap();
    assert_eq!(eff_first_descent, RisingPlatformEffect::Descending);
    assert_eq!(actor.timer, 0);
    assert_eq!(actor.position.y, 191);
    assert_eq!(actor.motion_leg, 55);

    // Descends remaining 9 steps back to 200 (budget 55 -> 64)
    for _ in 0..9 {
        let eff = update(&mut actor).unwrap();
        assert_eq!(eff, RisingPlatformEffect::Descending);
    }
    assert_eq!(actor.position.y, 200);
    assert_eq!(actor.motion_leg, 64);

    // Resting at bottom
    let eff_bottom = update(&mut actor).unwrap();
    assert_eq!(eff_bottom, RisingPlatformEffect::RestingAtBottom);
    assert_eq!(actor.position.y, 200);
}

#[test]
fn test_re_step_on_during_falling() {
    let mut actor = Actor::default();
    actor.position.y = 195;
    actor.state = RisingPlatformState::Falling.to_raw();
    actor.motion_leg = 59;
    actor.set_travel_span(64);
    actor.timer = 10;
    actor.interactions = InteractionFlags::from_bits(0x00);

    // Mario steps back on
    actor.interactions = InteractionFlags::from_bits(0x08);
    let eff = update(&mut actor).unwrap();
    assert_eq!(eff, RisingPlatformEffect::SteppedOn);
    assert_eq!(actor.state, RisingPlatformState::Rising.to_raw());
    assert_eq!(actor.timer, 0);
}
