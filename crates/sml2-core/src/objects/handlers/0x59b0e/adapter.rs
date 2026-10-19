use super::{handler_0x59b0e, *};
use crate::actor::{Actor, FrameTick};

impl ObjectSim {
    pub(super) fn handler_0x59b0e(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let effect = handler_0x59b0e::update(&mut actor, frame, self.mario_ref_x);
        st.b = actor.to_raw_slot();
        if matches!(effect, handler_0x59b0e::Effect::Crawl) {
            self.crawler_body(st, layout);
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
