//! Native third-form (final) Wario, ROM `$0AD93`; slot/world mapping lives in the adapter.

use crate::actor::{
    Actor, AnimationFrameError, ContactKind, EndingBobError, Facing, FrameTick, RiseProfile,
    SpriteFlipFlags,
};
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
    pub player_powerup: u8,
    pub saved_power_state: u8,
    pub child_sprite: u8,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct Effects {
    pub save_hit: Option<(u8, u8, ContactKind)>,
    pub spawn_child: Option<BossSpawnRequest>,
    pub fade_low: Option<u8>,
    pub intro_active: Option<bool>,
    pub consume_child_latch: bool,
    pub facing: Option<Facing>,
    pub sound0: Option<u8>,
    pub sound1: Option<u8>,
    pub sound2: Option<u8>,
    pub music_request: Option<u8>,
    pub boss_clear_flag: bool,
    pub fight_phase: Option<u8>,
    pub saved_power_state: Option<u8>,
    pub done: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum UpdateError {
    UnsupportedState,
    InvalidMotionProfile,
    InvalidWalkAnimation(AnimationFrameError),
    InvalidEndingAnimation(EndingBobError),
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
            // ROM $ADBE: spawn child 25, timer = 32, state = 12
            actor.state = 12;
            actor.timer = 32;
            effects.spawn_child = Some(BossSpawnRequest {
                animation: 25,
                sprite: 14,
                flip: context.spawn_flip,
            });
        }
        1 => update_hit_stun(actor, context.frame, context.stun),
        2 => {
            // ROM $AE17: Defeat fall
            effects.boss_clear_flag = true;
            actor.animate_sprite_blink(context.frame, 8, 49);
            let step = actor
                .motion_step(forward, 21)
                .map_err(|_| UpdateError::InvalidMotionProfile)?;
            actor.screen.move_y(i16::from(step));
            if actor.screen.y >= 138 {
                actor.state = 13;
                actor.timer = 128;
                actor.motion_phase = 2;
                effects.music_request = Some(24);
            }
        }
        3 => {
            // ROM $AEDB: prepare trajectory child throw
            actor.animation_frame = 0;
            actor.sprite = 92;
            let facing = actor.face_toward_screen(context.player_screen_x);
            effects.facing = Some(facing);
            actor.state = if facing.is_right() { 5 } else { 4 };
            effects.spawn_child = Some(BossSpawnRequest {
                animation: 27,
                sprite: 14,
                flip: actor.sprite_flip,
            });
            effects.sound2 = Some(8);
            actor.timer = 80;
        }
        4 | 5 => {
            // ROM $AF07 / $AF32: walk left (4) or right (5)
            if actor.timer != 0 {
                actor.timer -= 1;
                if actor.timer == 0 {
                    actor.sprite = 88;
                }
                return Ok(effects);
            }

            actor
                .animate_four_step_bob(context.frame)
                .map_err(UpdateError::InvalidWalkAnimation)?;

            if actor.state == 4 {
                actor.screen.move_x(-2);
                if actor.screen.x < 24 {
                    actor.state ^= 1;
                    actor.sprite_flip.toggle_horizontal();
                    return Ok(effects);
                }
            } else {
                actor.screen.move_x(2);
                if actor.screen.x >= 152 {
                    actor.state ^= 1;
                    actor.sprite_flip.toggle_horizontal();
                    return Ok(effects);
                }
            }

            // Proximity check to player
            let dist_x = actor.screen.horizontal_distance_to(context.player_screen_x);
            if dist_x < 48 {
                actor.state += 2; // 4 -> 6, 5 -> 7
                actor.sprite = 91;
                effects.saved_power_state = Some(context.player_powerup.wrapping_sub(2));
            } else if context.level_timer_low & 3 == 0 && context.child_sprite < 14 {
                actor.state = 3;
            }
        }
        6 | 7 => {
            // ROM $AF79 / $AF86: jump left (6) or right (7)
            if actor.state == 6 {
                actor.screen.move_x(-1);
                if actor.screen.x < 24 {
                    actor.state ^= 1;
                    actor.sprite_flip.toggle_horizontal();
                    return Ok(effects);
                }
            } else {
                actor.screen.move_x(1);
                if actor.screen.x >= 152 {
                    actor.state ^= 1;
                    actor.sprite_flip.toggle_horizontal();
                    return Ok(effects);
                }
            }

            // AF91 selects from the power byte saved by AF66, even if Mario changes.
            let high_jump = context.saved_power_state == 0;
            let cap = if high_jump { 27 } else { 15 };

            if actor.motion_leg == 0 {
                let profile = if high_jump {
                    RiseProfile::High
                } else {
                    RiseProfile::Low
                };
                let step = actor
                    .rise_step(profile)
                    .map_err(|_| UpdateError::InvalidMotionProfile)?;
                actor.screen.move_y(-i16::from(step));
            } else {
                let step = actor
                    .motion_step(forward, cap)
                    .map_err(|_| UpdateError::InvalidMotionProfile)?;
                actor.screen.move_y(i16::from(step));
            }

            if actor.motion_phase == cap {
                actor.motion_phase = 0;
                actor.motion_leg ^= 1;
                if actor.motion_leg == 0 {
                    // Jump completed: return to walking
                    actor.state -= 2; // 6 -> 4, 7 -> 5
                    actor.sprite = 88;
                    actor.animation_frame = 0;
                    if context.child_sprite < 14 {
                        actor.state = 3;
                    }
                }
            }
        }
        8 => {
            // ROM $AE58: defeat crying / spawn child 28
            effects.fade_low = Some(5);
            actor.animate_two_frame_toggle(context.frame, 16);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.timer = 48;
                effects.facing = Some(actor.face_toward_screen(context.player_screen_x));
                actor.state = 9;
                actor.sprite = 99;
                effects.spawn_child = Some(BossSpawnRequest {
                    animation: 28,
                    sprite: 108,
                    flip: actor.sprite_flip,
                });
            } else if actor.timer < 32 {
                actor.sprite = 99;
            }
        }
        9 => {
            // ROM $AE91: defeat pause
            effects.fade_low = Some(5);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.state = 10;
                actor.sprite = 102;
                actor.sprite_flip = SpriteFlipFlags::from_bits(0);
                actor.animation_frame = 0;
                actor.timer = 48;
            }
        }
        10 => {
            // ROM $AEAE: defeat run offscreen
            effects.fade_low = Some(5);
            actor
                .animate_ending_bob(context.frame)
                .map_err(UpdateError::InvalidEndingAnimation)?;
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(effects);
            }
            if context.frame.0 & 1 != 0 {
                return Ok(effects);
            }
            actor.screen.move_x(3);
            if actor.screen.x >= 192 {
                effects.done = true;
                effects.fight_phase = Some(3);
                // The phase event consumer also writes scene state zero.
                *actor = Actor::default();
            }
        }
        11 => {
            // ROM $ADD9: intro fade timer countdown
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
            }
        }
        12 => {
            // ROM $AC3A: wait for child 25 to land
            if context.child_landed {
                actor.animate_sprite_blink(context.frame, 8, 11);
                actor.timer = actor.timer.wrapping_sub(1);
                if actor.timer == 0 {
                    actor.state = 11;
                    actor.sprite = 116;
                    effects.consume_child_latch = true;
                }
            }
        }
        13 => {
            // ROM $AE3C: defeat ground pause
            actor.animate_sprite_blink(context.frame, 8, 49);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.motion_phase = actor.motion_phase.wrapping_sub(1);
                if actor.motion_phase == 0 {
                    actor.state = 8;
                    actor.timer = 255;
                    actor.sprite = 100;
                }
            }
        }
        _ => return Err(UpdateError::UnsupportedState),
    }

    Ok(effects)
}
