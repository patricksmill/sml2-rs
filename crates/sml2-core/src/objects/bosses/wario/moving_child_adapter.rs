//! Compatibility boundary between legacy object staging and native actors.

use super::moving_child::{update_child, ChildContext, RespawnEdge};
use crate::actor::{Actor, FrameTick};
use crate::objects::{ObjectSim, ObjectTables, Staging};

impl ObjectSim {
    pub(in crate::objects) fn handler_wario21(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let context = ChildContext {
            frame: FrameTick(u64::from(self.fff0)),
            transition_pending: self.boss_transition != 0,
            respawn_edge: if self.div & 1 == 0 {
                RespawnEdge::Left
            } else {
                RespawnEdge::Right
            },
        };
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = update_child(&mut actor, context, &tables.boss_picker_profile);
        st.b = actor.to_raw_slot();
        match result {
            Ok(effects) => {
                if effects.consume_transition {
                    self.boss_transition = 0;
                }
            }
            Err(_) => self.note_unported(0xAAA5),
        }
    }
}
