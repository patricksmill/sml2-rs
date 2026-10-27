//! Compatibility boundary for the native anim23 handler.
use super::form2::{self, Context};
use crate::actor::{Actor, ContactKind, FrameTick, SpriteFlipFlags};
use crate::objects::boss_stun::HitStunContext;
use crate::objects::{ObjectSim, ObjectTables, Staging};
impl ObjectSim {
    pub(in crate::objects) fn handler_wario23(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.boss_damage_pass(st);
        let context = Context {
            spawn_flip: SpriteFlipFlags::from_bits(self.spawn_attr),
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
            fade_timer: self.fade_timer,
            intro_active: self.ending_door_active != 0,
            child_landed: self.boss_delay != 0,
            level_timer_low: self.timer_low,
            player_screen_x: self.player_screen_x,
        };
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = form2::update(&mut actor, context, &tables.boss_picker_profile);
        st.b = actor.to_raw_slot();
        let Ok(effects) = result else {
            self.note_unported(0xAB8B);
            return;
        };
        if let Some((sprite, state, kind)) = effects.save_hit {
            self.spawn_sprite = sprite;
            self.saved_frame_state = state;
            self.saved_type = kind.raw();
        }
        if let Some(low) = effects.fade_low {
            self.fade_timer = (self.fade_timer & 0xff00) | u16::from(low);
            self.fade_timer_set = Some(self.fade_timer);
        }
        if let Some(active) = effects.intro_active {
            self.ending_door_active = u8::from(active);
        }
        if effects.consume_child_latch {
            self.boss_delay = 0;
        }
        if let Some(facing) = effects.facing {
            self.facing = if facing.is_right() { 1 } else { 255 };
        }
        if effects.camera_impact {
            self.camera_pan_set = Some(56);
            self.boss_transition = 1;
        }
        if let Some(sound) = effects.sound0 {
            self.sfx0 = sound;
        }
        if let Some(sound) = effects.sound1 {
            self.sfx1 = sound;
        }
        if let Some(sound) = effects.sound2 {
            self.sfx2 = sound;
        }
        if let Some(request) = effects.spawn_child {
            self.spawn_anim = request.animation;
            self.spawn_sprite = request.sprite;
            self.spawn_attr = request.flip.bits();
            self.apply_boss_spawn(st);
        }
        if effects.shared_exit {
            self.wario_phase2_exit(st, tables);
        }
    }
}
