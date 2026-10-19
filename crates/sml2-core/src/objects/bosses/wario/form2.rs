//! Native second-form Wario, ROM `$AB8B`; slot/world mapping lives in the adapter.
use crate::actor::{Actor, ContactKind, Facing, FrameTick, RiseProfile, SpriteFlipFlags};
use crate::objects::boss_spawn::BossSpawnRequest;
use crate::objects::boss_stun::{update_hit_stun, HitStunContext};

#[derive(Clone, Copy)]
pub(super) struct Context {
    pub frame: FrameTick,
    pub spawn_flip: SpriteFlipFlags,
    pub hit_processed: bool,
    pub stomp_latched: bool,
    pub stun_reload: u8,
    pub stun: HitStunContext,
    pub fade_timer: u16,
    pub intro_active: bool,
    pub child_landed: bool,
    pub level_timer_low: u8,
    pub player_screen_x: u8,
}
#[derive(Default, Debug)]
pub(super) struct Effects {
    pub save_hit: Option<(u8, u8, ContactKind)>,
    pub spawn_child: Option<BossSpawnRequest>,
    pub shared_exit: bool,
    pub fade_low: Option<u8>,
    pub intro_active: Option<bool>,
    pub consume_child_latch: bool,
    pub facing: Option<Facing>,
    pub camera_impact: bool,
    pub sound0: Option<u8>,
    pub sound1: Option<u8>,
    pub sound2: Option<u8>,
}

#[derive(Debug)]
pub(super) enum UpdateError {
    UnsupportedState,
    InvalidMotionProfile,
}

pub(super) fn update(
    actor: &mut Actor,
    context: Context,
    forward: &[u8],
) -> Result<Effects, UpdateError> {
    let mut effects = Effects::default();
    if context.hit_processed {
        effects.save_hit = Some((actor.sprite, actor.state, actor.contact_kind));
        if context.stomp_latched {
            actor.sprite = 82;
            actor.timer = context.stun_reload;
        }
        actor.contact_kind = ContactKind::NONE;
        actor.state = 1;
        return Ok(effects);
    }
    match actor.state {
        0 => {
            actor.state = 7;
            actor.timer = 32;
            effects.spawn_child = Some(BossSpawnRequest {
                animation: 22,
                sprite: 14,
                flip: context.spawn_flip,
            });
        }
        1 => update_hit_stun(actor, context.frame, context.stun),
        2 => {
            actor.flash.animate_palette(context.frame);
            let step = actor
                .motion_step(forward, 21)
                .map_err(|_| UpdateError::InvalidMotionProfile)?;
            actor.screen.move_y(i16::from(step));
            if actor.screen.y >= 138 {
                actor.state = 6;
                actor.timer = 128;
            }
        }
        3 => {
            let step = actor
                .rise_step(RiseProfile::Full)
                .map_err(|_| UpdateError::InvalidMotionProfile)?;
            actor.screen.move_y(-i16::from(step));
            if actor.motion_phase == RiseProfile::Full.cap() {
                actor.motion_phase = 0;
                let facing = actor.face_toward_screen(context.player_screen_x);
                effects.facing = Some(facing);
                actor.state = if facing.is_right() { 5 } else { 4 };
                effects.sound2 = Some(2);
            }
        }
        4 | 5 => {
            actor.animate_two_frame_toggle(context.frame, 8);
            let left = actor.state == 4;
            actor.screen.move_x(if left { -1 } else { 1 });
            if (left && actor.screen.x < 32) || (!left && actor.screen.x >= 144) {
                actor.state ^= 1;
                actor.sprite_flip.toggle_horizontal();
                effects.sound2 = Some(2);
            } else if context.frame.every(8) {
                if actor.screen.horizontal_distance_to(context.player_screen_x) < 32
                    && context.level_timer_low & 3 == 0
                {
                    actor.state = 10;
                    actor.sprite = 86;
                    actor.timer = 32;
                } else {
                    actor.screen.move_y(1);
                    if actor.screen.y >= 138 {
                        actor.state = 3;
                        actor.sprite = 86;
                    }
                }
            }
        }
        6 => {
            effects.fade_low = Some(5);
            actor.flash.animate_palette(context.frame);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.animation_frame = 0;
                actor.flash.clear();
                actor.state = 8;
                actor.sprite = 83;
                actor.sprite_flip = crate::actor::SpriteFlipFlags::from_bits(32);
            }
        }
        7 => {
            if context.child_landed {
                actor.animate_sprite_blink(context.frame, 8, 31);
                actor.timer = actor.timer.wrapping_sub(1);
                if actor.timer == 0 {
                    actor.state = 11;
                    actor.sprite = 114;
                    effects.consume_child_latch = true;
                }
            }
        }
        8 => effects.shared_exit = true,
        9 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(effects);
            }
            let cap = if actor.motion_leg == 0 {
                let step = actor
                    .rise_step(RiseProfile::Short)
                    .map_err(|_| UpdateError::InvalidMotionProfile)?;
                actor.screen.move_y(-i16::from(step));
                12
            } else {
                let step = actor
                    .motion_step(forward, 12)
                    .map_err(|_| UpdateError::InvalidMotionProfile)?;
                actor.screen.move_y(i16::from(step));
                12
            };
            if actor.motion_phase == cap {
                actor.motion_phase = 0;
                actor.animation_frame = 0;
                actor.sprite = 83;
                actor.motion_leg ^= 1;
                if actor.motion_leg == 0 {
                    actor.state = 3;
                    actor.sprite = 86;
                }
            }
        }
        10 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(effects);
            }
            let step = actor
                .motion_step(forward, 21)
                .map_err(|_| UpdateError::InvalidMotionProfile)?;
            actor.screen.move_y(i16::from(step));
            if actor.screen.y >= 138 {
                actor.motion_phase = 0;
                actor.state = 9;
                actor.timer = 48;
                effects.camera_impact = true;
                effects.sound0 = Some(24);
            }
        }
        11 => {
            let starting = context.fade_timer as u8 == 97;
            if starting {
                effects.sound1 = Some(1);
                effects.intro_active = Some(true);
            }
            if context.intro_active || starting {
                actor.animate_two_frame_toggle(context.frame, 8);
            }
            if context.fade_timer == 0 {
                effects.intro_active = Some(false);
                actor.state = 3;
                actor.contact_kind = ContactKind::from_raw(65);
                actor.sprite = 86;
            }
        }
        _ => return Err(UpdateError::UnsupportedState),
    }
    Ok(effects)
}
