//! Slot conversion and crawler patrol for ROM `$5A2D0` (mode B anim 64).
use super::{handler_0x5a2d0, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x5a2d0(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
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
            Ok(handler_0x5a2d0::Effect::Crawl) => self.crawler_patrol(st, layout),
            Err(_) => self.note_unported(0x5A2D0),
        }
    }

    /// `$5864E` / `$5866D`, without the anim-8 entry's sprite toggle.
    fn crawler_patrol(&mut self, st: &mut Staging, layout: &LevelLayout) {
        match st.b[B_FSTATE] {
            1 | 3 => {
                if self.fff0 & 1 == 0 {
                    self.mv_x_sub(st, 1);
                    self.m_left_wall(st, layout);
                    if self.move_flags & 0x02 != 0 {
                        st.b[B_FSTATE] = 2;
                        st.b[B_FLIP] ^= 0x20;
                    }
                    return;
                }
                self.crawler_vertical(st, layout);
            }
            2 | 4 => {
                if self.fff0 & 1 == 0 {
                    self.mv_x_add(st, 1);
                    self.m_right_wall(st, layout);
                    if self.move_flags & 0x01 != 0 {
                        st.b[B_FSTATE] = 1;
                        st.b[B_FLIP] ^= 0x20;
                    }
                    return;
                }
                self.crawler_vertical(st, layout);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/rising_crawler.rs"]
mod tests;
