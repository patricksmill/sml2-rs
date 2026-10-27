//! Slot conversion and crawler patrol for ROM `$5A2D0` (mode B anim 64).
use super::{handler_0x5a2d0, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5a2d0(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = handler_0x5a2d0::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a2d0::Effect::Initialized) | Ok(handler_0x5a2d0::Effect::Continue) => {}
            Ok(handler_0x5a2d0::Effect::Faced { facing }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(handler_0x5a2d0::Effect::Crawl) => self.crawler_body(st, layout),
            Err(_) => self.note_unported(0x5A2D0),
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/rising_crawler.rs"]
mod tests;
