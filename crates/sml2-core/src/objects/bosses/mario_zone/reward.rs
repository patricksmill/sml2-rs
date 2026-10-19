//! Slot/world boundary for shared native boss reward motion.
use super::reward_motion::{self, Part};
use crate::{
    actor::{Actor, FrameTick},
    object::ObjectTables,
    objects::{ObjectSim, Staging},
};

impl ObjectSim {
    pub(in crate::objects) fn handler_boss_anim4(
        &mut self,
        st: &mut Staging,
        tables: &ObjectTables,
    ) {
        self.update_boss_reward(st, tables, Part::Main, 0x9642);
    }
    pub(in crate::objects) fn handler_boss_anim5(
        &mut self,
        st: &mut Staging,
        tables: &ObjectTables,
    ) {
        self.update_boss_reward(st, tables, Part::Child, 0x9698);
    }
    fn update_boss_reward(
        &mut self,
        st: &mut Staging,
        tables: &ObjectTables,
        part: Part,
        handler: u32,
    ) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = reward_motion::update(
            &mut actor,
            part,
            FrameTick(u64::from(self.fff0)),
            &tables.boss_picker_profile,
        );
        st.b = actor.to_raw_slot();
        match result {
            Ok(effects) => {
                if effects.spawn_child {
                    self.spawn_anim = 5;
                    self.spawn_sprite = 12;
                    self.spawn_attr = actor.sprite_flip.bits();
                    self.boss_spawn_child(st);
                }
                if effects.clear {
                    self.boss_clear_flag = 1;
                    self.boss_clear_handoff = true;
                    self.done = 1;
                }
            }
            Err(()) => self.note_unported(handler),
        }
    }
}
