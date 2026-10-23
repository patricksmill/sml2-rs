//! The room to the left of the file select. World 0 is the doorway.

use super::catalog::{self, Pedestal};
use crate::settings::Settings;

pub(super) const GAP: i32 = 48;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Hall {
    pub mario: i32,
    pub hop: u8,
    pub goal: Option<usize>,
}

impl Hall {
    pub(super) fn enter() -> Self {
        Self {
            mario: 0,
            hop: 0,
            goal: None,
        }
    }
}

pub(super) fn pedestal_x(index: usize) -> i32 {
    -GAP * (index as i32 + 1)
}

pub(super) fn step(
    hall: &mut Hall,
    left: bool,
    right: bool,
    jump: bool,
    settings: &mut Settings,
    debug: &mut bool,
) -> bool {
    if hall.hop > 0 {
        hall.hop -= 1;
        return false;
    }
    if let Some(index) = hall.goal {
        let target = pedestal_x(index);
        if hall.mario < target {
            hall.mario += 1;
        } else if hall.mario > target {
            hall.mario -= 1;
        } else {
            catalog::cycle(catalog::PEDESTALS[index], settings, debug);
            hall.hop = 12;
            hall.goal = None;
        }
        return false;
    }
    if left {
        hall.mario -= 1;
    } else if right {
        hall.mario += 1;
    }
    if hall.mario > 0 {
        return true;
    }
    if jump {
        if let Some(index) = (0..catalog::PEDESTALS.len()).find(|index| {
            (hall.mario - pedestal_x(*index)).abs() <= 8
        }) {
            catalog::cycle(catalog::PEDESTALS[index], settings, debug);
            hall.hop = 12;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walking_left_reaches_volume_and_a_jump_raises_it() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        let volume_at = pedestal_x(
            catalog::PEDESTALS
                .iter()
                .position(|p| *p == Pedestal::Volume)
                .unwrap(),
        );
        for _ in 0..200 {
            if hall.mario == volume_at {
                break;
            }
            assert!(!step(&mut hall, true, false, false, &mut settings, &mut debug));
        }
        assert_eq!(hall.mario, volume_at);
        assert!(!step(&mut hall, false, false, true, &mut settings, &mut debug));
        assert!((settings.volume - 0.8).abs() < f32::EPSILON);
        assert!(hall.hop > 0);
        let during_hop = settings.volume;
        step(&mut hall, false, false, true, &mut settings, &mut debug);
        assert!((settings.volume - during_hop).abs() < f32::EPSILON, "the hop ignores A");
    }

    #[test]
    fn walking_right_through_the_door_leaves() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        assert!(step(&mut hall, false, true, false, &mut settings, &mut debug));
    }
}
