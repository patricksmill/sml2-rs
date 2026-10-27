//! Slot conversion and ceiling probe for ROM `$5A6E7` (mode B anim 74).
use super::{handler_0x5a6e7, *};
use crate::actor::Actor;
use std::cmp::Ordering;

struct Inputs<'a> {
    layout: &'a LevelLayout,
}

impl handler_0x5a6e7::World for Inputs<'_> {
    fn ceiling_present(&mut self, actor: &Actor, moving_left: bool) -> bool {
        let x = if moving_left {
            actor.position.x.wrapping_sub(16)
        } else {
            actor.position.x.wrapping_add(16)
        };
        let y = actor.position.y.wrapping_sub(18);
        self.layout
            .get(usize::from((x >> 4) & 255), usize::from((y >> 4) & 255))
            .unwrap_or(0)
            < 0x38
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5a6e7(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let player_x = self.mario_ref_x;
        let result = handler_0x5a6e7::update(&mut actor, frame, player_x, &mut Inputs { layout });
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a6e7::Effect::Initialized { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5a6e7::Effect::Continue) => {}
            Err(_) => self.note_unported(0x5A6E7),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/ceiling_crawler.rs"]
mod tests;
