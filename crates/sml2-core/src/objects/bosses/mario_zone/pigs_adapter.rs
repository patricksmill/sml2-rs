//! Compatibility boundary for the native Mario Zone boss handlers.
use super::pigs::{self, Context, Effects, Part, UpdateError};
use crate::actor::{Actor, ContactKind, FrameTick};
use crate::level::LevelLayout;
use crate::object::ObjectTables;
use crate::objects::boss_stun::HitStunContext;
use crate::objects::{ObjectSim, Staging};

impl ObjectSim {
    fn pig_context(&self) -> Context {
        Context {
            frame: self.frame_tick(),
            hit_processed: self.done != 0,
            stomp_latched: self.boss_hit_latch != 0,
            stun_reload: self.boss_timer_reload,
            stun: HitStunContext {
                removal_pending: self.remove_mode != 0,
                definition_sprite: self.boss_sprite,
                saved_sprite: self.spawn_sprite,
                saved_state: self.saved_frame_state,
                saved_contact_kind: ContactKind::from_raw(self.saved_type),
            },
            fade_idle: self.fade_idle(),
            definitions_left: self.boss_def_count,
            hits_remaining: self.boss_hits_remaining,
        }
    }

    /// Run one native pig update over the slot, then apply its world effects.
    /// `unsupported` is the ROM table base reported per state; `invalid` is
    /// reported for a motion profile error.
    fn run_pig(
        &mut self,
        st: &mut Staging,
        update: impl FnOnce(&mut Actor, Context) -> Result<Effects, UpdateError>,
        unsupported: u32,
        invalid: u32,
    ) {
        self.boss_damage_pass(st);
        let mut actor = Actor::from_raw_slot(&st.b);
        let state = actor.state;
        let result = update(&mut actor, self.pig_context());
        st.b = actor.to_raw_slot();
        let effects = match result {
            Ok(effects) => effects,
            Err(UpdateError::UnsupportedState) => {
                return self.note_unported(unsupported + u32::from(state) * 2);
            }
            Err(UpdateError::InvalidMotionProfile) => return self.note_unported(invalid),
        };
        if let Some((sprite, state, kind)) = effects.save_hit {
            self.spawn_sprite = sprite;
            self.saved_frame_state = state;
            self.saved_type = kind.raw();
        }
        if let Some(sound) = effects.sound0 {
            self.sfx0 = sound;
        }
        if let Some(sound) = effects.sound1 {
            self.sfx1 = sound;
        }
        if effects.release_remove_mode {
            self.remove_mode = 0;
        }
        if let Some((animation, sprite)) = effects.spawn_child {
            self.spawn_anim = animation;
            self.spawn_sprite = sprite;
            self.boss_spawn_child(st);
        }
        if effects.morph {
            self.boss_morph_to_anim3(st);
        }
    }

    /// `$9C36` (boss anim 10): Pig 1 (straw house).
    pub(in crate::objects) fn handler_boss_def2_pig1(
        &mut self,
        st: &mut Staging,
        _layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let fall = &tables.boss_picker_profile;
        self.run_pig(
            st,
            |actor, context| pigs::update_part(actor, Part::Straw, context, fall),
            0x9CA1,
            0x9C36,
        );
    }

    /// `$9D4F` (boss anim 11): Pig 2 (wood house).
    pub(in crate::objects) fn handler_boss_def2_pig2(
        &mut self,
        st: &mut Staging,
        _layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let fall = &tables.boss_picker_profile;
        self.run_pig(
            st,
            |actor, context| pigs::update_part(actor, Part::Wood, context, fall),
            0x9D78,
            0x9D4F,
        );
    }

    /// Shared router for def-2 parts: `left == false` -> Pig 1, `left == true` -> Pig 2.
    pub(in crate::objects) fn handler_boss_def2_part(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
        left: bool,
    ) {
        if left {
            self.handler_boss_def2_pig2(st, layout, tables);
        } else {
            self.handler_boss_def2_pig1(st, layout, tables);
        }
    }

    /// `$9E07` (boss anim 12): Pig 3 (brick house), the controller.
    pub(in crate::objects) fn handler_boss_def2_controller(
        &mut self,
        st: &mut Staging,
        _layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let (rise, fall) = (&tables.boss_picker_profile_15, &tables.boss_picker_profile);
        self.run_pig(
            st,
            |actor, context| pigs::update_controller(actor, context, rise, fall),
            0x9E5C,
            0x9E07,
        );
    }

    /// `$9606` (boss anim 2): the def-2 spawner.
    pub(in crate::objects) fn handler_boss_def2(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let spawn = pigs::update_spawner(&mut actor, self.remove_mode);
        st.b = actor.to_raw_slot();
        if let Some((animation, sprite)) = spawn {
            self.spawn_anim = animation;
            self.spawn_sprite = sprite;
            self.boss_spawn_child(st);
        }
    }

    /// `$9EE3` (boss anim 13): the controller's child.
    pub(in crate::objects) fn handler_boss_anim13(&mut self, st: &mut Staging) {
        let mut actor = Actor::from_raw_slot(&st.b);
        pigs::update_child(&mut actor, self.remove_mode);
        st.b = actor.to_raw_slot();
    }

    /// `$9CDF` over a raw slot, for the engine's bob fixture.
    #[cfg(test)]
    pub(in crate::objects) fn boss_bob(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        pigs::bob(&mut actor, &tables.boss_picker_profile).expect("bob profile");
        st.b = actor.to_raw_slot();
    }
}
