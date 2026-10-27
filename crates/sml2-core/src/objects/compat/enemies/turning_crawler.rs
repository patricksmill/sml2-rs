use super::{handler_0x59b0e, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x59b0e(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let effect = handler_0x59b0e::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        if matches!(effect, handler_0x59b0e::Effect::Crawl) {
            self.crawler_body(st, layout);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/turning_crawler.rs"]
mod tests;
