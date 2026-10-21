//! Native floaters, anims 94 (`$5B241`, vertical) and 95 (`$5B28C`,
//! horizontal): legs alternate between the `$74A1` curve and the fall curve,
//! and the floater turns back after each `$74A1` leg. Slot mapping lives in
//! the `items` adapter.
use super::{FALL_SPEED, STEP_74A1};
use crate::actor::{Actor, ContactKind};

/// Contact kind while floating.
const FLOATING_CONTACT: u8 = 0x48;
/// Both curves end at this phase.
const CURVE_END: u8 = 0x12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Axis {
    Vertical,
    Horizontal,
}

/// States 1/2 float up/left and down/right.
pub(super) fn update(actor: &mut Actor, axis: Axis) {
    let toward = match actor.state {
        0 => {
            actor.contact_kind = ContactKind::from_raw(FLOATING_CONTACT);
            actor.state = 1;
            return;
        }
        1 => -1,
        2 => 1,
        _ => return,
    };
    let step = toward * i16::from(curve_step(actor));
    match axis {
        Axis::Vertical => actor.position.move_y(step),
        Axis::Horizontal => actor.position.move_x(step),
    }
    end_leg(actor);
}

/// The `$74A1` curve on `motion_leg` 0, else the fall curve. A phase past
/// the curve reads 0.
fn curve_step(actor: &mut Actor) -> u8 {
    let curve: &[u8] = if actor.motion_leg == 0 {
        &STEP_74A1
    } else {
        &FALL_SPEED
    };
    let step = curve
        .get(usize::from(actor.motion_phase))
        .copied()
        .unwrap_or(0);
    if actor.motion_phase != CURVE_END {
        actor.motion_phase = actor.motion_phase.wrapping_add(1);
    }
    step
}

/// A finished curve swaps to the other one; leaving the `$74A1` curve also
/// turns back (`$5D33`).
fn end_leg(actor: &mut Actor) {
    if actor.motion_phase != CURVE_END {
        return;
    }
    actor.motion_phase = 0;
    actor.motion_leg ^= 1;
    if actor.motion_leg != 0 {
        actor.state ^= 3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floater_eases_out_and_back_then_reverses() {
        let mut actor = Actor::default();
        actor.position.x = 0x100;
        update(&mut actor, Axis::Horizontal);
        assert_eq!((actor.state, actor.contact_kind.raw()), (1, 0x48));
        for _ in 0..usize::from(CURVE_END) {
            update(&mut actor, Axis::Horizontal);
        }
        let out: u16 = STEP_74A1[..18].iter().map(|&v| u16::from(v)).sum();
        assert_eq!(
            (actor.position.x, actor.motion_leg, actor.state),
            (0x100 - out, 1, 2)
        );
        for _ in 0..usize::from(CURVE_END) {
            update(&mut actor, Axis::Horizontal);
        }
        assert_eq!((actor.motion_leg, actor.state), (0, 2));
    }
}
