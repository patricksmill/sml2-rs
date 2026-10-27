//! Compatibility boundary between legacy object staging and native trajectory child actor.

use super::trajectory_child::{
    update_trajectory_child, TrajectoryChildContext, TrajectoryChildEffects,
};
use crate::actor::Actor;
use crate::objects::{ObjectSim, Staging};

impl ObjectSim {
    pub(in crate::objects) fn handler_wario27(&mut self, st: &mut Staging) {
        let context = TrajectoryChildContext {
            frame: self.frame_tick(),
        };
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = update_trajectory_child(&mut actor, context);
        match result {
            Ok(TrajectoryChildEffects::None) => {
                st.b = actor.to_raw_slot();
            }
            Ok(TrajectoryChildEffects::CountedRemove) => {
                self.boss_def_count = self.boss_def_count.wrapping_sub(1);
                self.done = 1;
                st.b = [0u8; 32];
            }
            Err(_) => {
                st.b = actor.to_raw_slot();
                self.note_unported(0xAFE0);
            }
        }
    }
}
