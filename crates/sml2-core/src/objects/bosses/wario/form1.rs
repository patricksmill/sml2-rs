//! Native first-form Wario, ROM `$A8AB` (bank 2 CPU `$68AB`); slot/world mapping
//! lives in `form1_adapter`.
use crate::actor::{Actor, ContactKind, Facing, FrameTick, RiseProfile, SpriteFlipFlags};
use crate::objects::boss_stun::{update_hit_stun, HitStunContext};

/// The fade low byte that starts the door intro and its blink.
const INTRO_FADE_LOW: u8 = 0x61;
/// Screen Y at which a descent lands on the arena floor.
const FLOOR_Y: u8 = 0x8A;
/// Side walk turnaround bounds.
const LEFT_WALL_X: u8 = 0x10;
const RIGHT_WALL_X: u8 = 0x98;

#[derive(Clone, Copy)]
pub(super) struct Context {
    pub frame: FrameTick,
    pub hit_processed: bool,
    pub stomp_latched: bool,
    pub stun_reload: u8,
    pub stun: HitStunContext,
    pub fade_timer: u16,
    pub intro_active: bool,
    pub child_landed: bool,
    pub level_timer_low: u8,
}

#[derive(Default, Debug)]
pub(super) struct Effects {
    pub save_hit: Option<(u8, u8, ContactKind)>,
    pub intro_active: Option<bool>,
    pub consume_child_latch: bool,
    /// Landing shake: camera pan distance, with the boss transition latch.
    pub camera_impact: Option<u8>,
    pub sound0: Option<u8>,
    pub sound1: Option<u8>,
    /// State 8 hands the slot to the shared phase-1 exit.
    pub phase1_exit: bool,
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
            actor.sprite = 0x52;
            actor.timer = context.stun_reload;
        }
        actor.contact_kind = ContactKind::NONE;
        actor.state = 1;
        return Ok(effects);
    }
    let descend = |actor: &mut Actor, cap: u8| -> Result<(), UpdateError> {
        let step = actor
            .motion_step(forward, cap)
            .map_err(|_| UpdateError::InvalidMotionProfile)?;
        actor.screen.move_y(i16::from(step));
        Ok(())
    };
    let rise = |actor: &mut Actor, profile: RiseProfile| -> Result<(), UpdateError> {
        let step = actor
            .rise_step(profile)
            .map_err(|_| UpdateError::InvalidMotionProfile)?;
        actor.screen.move_y(-i16::from(step));
        Ok(())
    };
    match actor.state {
        // Door intro: blink until the fade finishes, then start the first jump.
        0 => {
            let mut intro = context.intro_active;
            if context.fade_timer as u8 == INTRO_FADE_LOW {
                effects.sound1 = Some(0x0F);
                effects.intro_active = Some(true);
                intro = true;
            }
            if intro {
                actor.animate_sprite_blink(context.frame, 8, 1);
            }
            if context.fade_timer != 0 {
                return Ok(effects);
            }
            effects.intro_active = Some(false);
            actor.state = 3;
            actor.contact_kind = ContactKind::from_raw(0x41);
            actor.sprite = 0x4F;
        }
        1 => update_hit_stun(actor, context.frame, context.stun),
        // Defeat fall.
        2 => {
            actor.flash.animate_palette(context.frame);
            descend(actor, 21)?;
            if actor.screen.y >= FLOOR_Y {
                actor.state = 6;
                actor.timer = 0x80;
            }
        }
        // Short hop: rise, then fall to the floor and shake the arena.
        3 => {
            if actor.motion_leg == 0 {
                rise(actor, RiseProfile::Short)?;
                if actor.motion_phase == RiseProfile::Short.cap() {
                    actor.motion_phase = 0;
                    actor.motion_leg = 1;
                }
            } else {
                descend(actor, 24)?;
                if actor.screen.y >= FLOOR_Y {
                    actor.motion_phase = 0;
                    actor.motion_leg = 0;
                    impact(actor, &mut effects, 0x34, 0x30);
                }
            }
        }
        // Walk between the walls until the level timer's low byte is a multiple of 8.
        4 => {
            if context.level_timer_low & 7 == 0 {
                actor.state = 5;
                actor.sprite = 0x4F;
                return Ok(effects);
            }
            actor
                .animate_four_step_bob(context.frame)
                .map_err(|_| UpdateError::InvalidMotionProfile)?;
            let walking_right = actor.travel_direction.is_right();
            if walking_right {
                actor.screen.move_x(2);
                if actor.screen.x < RIGHT_WALL_X {
                    return Ok(effects);
                }
            } else {
                actor.screen.move_x(-2);
                if actor.screen.x >= LEFT_WALL_X {
                    return Ok(effects);
                }
            }
            actor.sprite_flip.toggle_horizontal();
            actor.travel_direction = Facing::from_raw(actor.travel_direction.raw() ^ 1);
            actor.timer = 8;
        }
        // High jump: medium rise, full fall, then shake.
        5 => {
            if actor.motion_leg == 0 {
                rise(actor, RiseProfile::Medium)?;
            } else {
                descend(actor, 21)?;
            }
            if actor.motion_phase == 21 {
                actor.motion_phase = 0;
                actor.motion_leg ^= 1;
                if actor.motion_leg == 0 {
                    impact(actor, &mut effects, 0x38, 0x38);
                }
            }
        }
        // Defeated: flash, then face right for the exit.
        6 => {
            actor.flash.animate_palette(context.frame);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.animation_frame = 0;
                actor.flash.clear();
                actor.state = 8;
                actor.sprite = 0x4C;
                actor.sprite_flip = SpriteFlipFlags::from_bits(0x20);
            }
        }
        // `$A952`: shared with Wario23; Wario20 never enters it (see feature inventory).
        7 => {
            if !context.child_landed {
                return Ok(effects);
            }
            actor.animate_sprite_blink(context.frame, 8, 0x1F);
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                actor.state = 11;
                actor.sprite = 0x72;
                effects.consume_child_latch = true;
            }
        }
        8 => effects.phase1_exit = true,
        // Post-landing wait, then a short bounce before walking again.
        9 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                return Ok(effects);
            }
            if actor.motion_leg == 0 {
                rise(actor, RiseProfile::Short)?;
            } else {
                descend(actor, 12)?;
            }
            if actor.motion_phase == 12 {
                actor.motion_phase = 0;
                actor.animation_frame = 0;
                actor.sprite = 0x4C;
                actor.motion_leg ^= 1;
                if actor.motion_leg == 0 {
                    actor.state = 4;
                }
            }
        }
        _ => return Err(UpdateError::UnsupportedState),
    }
    Ok(effects)
}

/// Landing: wait, shake the camera and play the thud.
fn impact(actor: &mut Actor, effects: &mut Effects, pan: u8, wait: u8) {
    actor.state = 9;
    actor.timer = wait;
    effects.camera_impact = Some(pan);
    effects.sound0 = Some(0x18);
}
