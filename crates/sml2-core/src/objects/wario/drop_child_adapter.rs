//! Compatibility boundary between legacy object staging and native drop child actor.

use super::drop_child::{update_drop_child, DropChildContext};
use crate::actor::{Actor, FrameTick};
use crate::objects::{ObjectSim, ObjectTables, Staging};

impl ObjectSim {
    pub(in crate::objects) fn handler_wario28(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let context = DropChildContext {
            frame: FrameTick(u64::from(self.fff0)),
            player_screen_x: self.player_screen_x,
            picker_profile: &tables.boss_picker_profile,
        };
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = update_drop_child(&mut actor, context);
        st.b = actor.to_raw_slot();
        match result {
            Ok(effects) => {
                if let Some(sound) = effects.sound0 {
                    self.sfx0 = sound;
                }
            }
            Err(_) => {
                self.note_unported(0xB0C2);
            }
        }
    }
}
