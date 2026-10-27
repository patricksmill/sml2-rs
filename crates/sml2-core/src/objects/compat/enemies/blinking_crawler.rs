//! Slot conversion for ROM `$5A9D3` (mode B anim 82).
use super::{handler_0x5a9d3, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x5a9d3(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let effect = handler_0x5a9d3::update(&mut actor, frame);
        st.b = actor.to_raw_slot();
        if matches!(effect, handler_0x5a9d3::Effect::Crawl) {
            self.crawler_body(st, layout);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/blinking_crawler.rs"]
mod tests;
