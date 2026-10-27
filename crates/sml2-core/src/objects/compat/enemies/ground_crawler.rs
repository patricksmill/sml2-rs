//! Slot conversion and ground probe for ROM `$5A739` (mode B anim 76).
use super::{handler_0x5a739, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

struct Inputs<'a> {
    layout: &'a LevelLayout,
}

impl handler_0x5a739::World for Inputs<'_> {
    fn ground_present(&mut self, actor: &Actor, moving_left: bool) -> bool {
        if actor.position.y & 0x0C != 0 {
            return false;
        }
        let x = if moving_left {
            actor.position.x.wrapping_sub(16)
        } else {
            actor.position.x.wrapping_add(16)
        };
        self.layout
            .get(
                usize::from((x >> 4) & 255),
                usize::from((actor.position.y >> 4) & 255),
            )
            .unwrap_or(0)
            < 0x48
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5a739(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let player_x = self.mario_ref_x;
        let result = handler_0x5a739::update(&mut actor, frame, player_x, &mut Inputs { layout });
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a739::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5a739::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5A739),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/ground_crawler.rs"]
mod tests;
