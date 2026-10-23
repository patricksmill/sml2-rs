//! Native Mario Zone boss (Three Piggys, definition 2), bank 2: Pig 1 (anim 10,
//! `$9C36`), Pig 2 (anim 11, `$9D4F`), the controller Pig 3 (anim 12, `$9E07`),
//! the spawner (anim 2, `$9606`) and the controller's child (anim 13, `$9EE3`).
//! Slot/world mapping lives in `pigs_adapter`.
use crate::actor::{Actor, ContactKind, FrameTick, RiseProfile, SpriteFlipFlags};
use crate::objects::boss_stun::{update_hit_stun, HitStunContext};

/// Arena walls for every walk.
const LEFT_WALL_X: u8 = 0x10;
const RIGHT_WALL_X: u8 = 0xA0;
/// The intro drop and every bob land on this floor.
const FLOOR_Y: u8 = 0x90;
/// Intro frames the second pig and the controller wait after their cue.
const INTRO_WAIT: u8 = 0x30;

#[derive(Clone, Copy)]
pub(super) struct Context {
    pub frame: FrameTick,
    pub hit_processed: bool,
    pub stomp_latched: bool,
    pub stun_reload: u8,
    pub stun: HitStunContext,
    pub fade_idle: bool,
    /// Boss definitions still alive (`sBossDefCount`).
    pub definitions_left: u8,
    /// The controller's speed comes from the shared hit counter, not its own slot.
    pub hits_remaining: u8,
}

#[derive(Default, Debug)]
pub(super) struct Effects {
    pub save_hit: Option<(u8, u8, ContactKind)>,
    pub sound0: Option<u8>,
    pub sound1: Option<u8>,
    /// State 2 hands the slot to the shared death morph.
    pub morph: bool,
    /// The intro cue releases the remove mode for the whole fight.
    pub release_remove_mode: bool,
    /// Child animation and sprite; the flip stays the engine's current spawn flip.
    pub spawn_child: Option<(u8, u8)>,
}

#[derive(Debug)]
pub(super) enum UpdateError {
    UnsupportedState,
    InvalidMotionProfile,
}

/// The two parts differ in how their intro starts and how they move.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    /// Pig 1: runs along the floor.
    Straw,
    /// Pig 2: hops, waiting until it is the last part left.
    Wood,
}

/// `$B402`: the intro blink flips the sprite every sixteenth frame.
fn flip_blink(actor: &mut Actor, frame: FrameTick) {
    if frame.every(16) {
        actor.sprite_flip.toggle_horizontal();
    }
}

/// `$B3D7`: every fourth frame step an eight-sprite run cycle.
fn run_cycle(actor: &mut Actor, frame: FrameTick) {
    if !frame.every(4) {
        return;
    }
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    if actor.animation_frame == 8 {
        actor.animation_frame = 0;
        actor.sprite = actor.sprite.wrapping_sub(7);
    } else {
        actor.sprite = actor.sprite.wrapping_add(1);
    }
}

/// `$9C3F`: a part's hit. A stomp costs 2 HP, a projectile 1; zero or a
/// borrow starts the defeat fall (`$5435`).
fn part_hit(actor: &mut Actor, context: &Context, effects: &mut Effects) {
    effects.save_hit = Some((actor.sprite, actor.state, actor.contact_kind));
    let damage = if context.stomp_latched {
        effects.sound0 = Some(0x13);
        actor.sprite = 0x1F;
        actor.timer = context.stun_reload;
        2
    } else {
        effects.sound1 = Some(9);
        1
    };
    actor.contact_kind = ContactKind::NONE;
    let (hp, borrow) = actor.activity.overflowing_sub(damage);
    actor.activity = hp;
    if hp == 0 || borrow {
        actor.state = 2;
        actor.timer = 0x20;
        actor.sprite = context.stun.definition_sprite;
    } else {
        actor.state = 1;
    }
}

/// `$9C8B`: the intro's end arms the fight.
fn arm(actor: &mut Actor, effects: &mut Effects) {
    actor.sprite = 0x20;
    actor.timer = 0;
    actor.state = 3;
    actor.contact_kind = ContactKind::from_raw(0x45);
    effects.sound1 = Some(4);
}

/// `$9CDF`: rise on the cap-15 curve, then fall on the cap-21 curve to the floor;
/// the intro (state 3 or below) then starts walking left.
pub(super) fn bob(actor: &mut Actor, fall: &[u8]) -> Result<(), UpdateError> {
    if actor.motion_leg == 0 {
        let step = actor
            .rise_step(RiseProfile::Low)
            .map_err(|_| UpdateError::InvalidMotionProfile)?;
        actor.screen.move_y(-i16::from(step));
        if actor.motion_phase == RiseProfile::Low.cap() {
            actor.motion_phase = 0;
            actor.motion_leg = 1;
        }
        return Ok(());
    }
    let step = actor
        .motion_step(fall, 21)
        .map_err(|_| UpdateError::InvalidMotionProfile)?;
    actor.screen.move_y(i16::from(step));
    if actor.screen.y < FLOOR_Y {
        return Ok(());
    }
    actor.screen.y = FLOOR_Y;
    actor.motion_phase = 0;
    actor.motion_leg = 0;
    if actor.state <= 3 {
        actor.state = 4;
        actor.sprite_flip = SpriteFlipFlags::from_bits(0x20);
    }
    Ok(())
}

/// `$9D3D`: a walk reached a wall; turn around and re-announce the animation.
fn turn_at_wall(actor: &mut Actor, effects: &mut Effects) {
    actor.state ^= 1;
    actor.sprite_flip.toggle_horizontal();
    effects.sound1 = Some(actor.definition);
}

/// Walk `speed` pixels toward a wall; returns whether it was reached.
/// Pig 1 walks right in state 4, the controller in state 5.
fn walk(actor: &mut Actor, rightward: bool, speed: u8) -> bool {
    if rightward {
        actor.screen.move_x(i16::from(speed));
        actor.screen.x >= RIGHT_WALL_X
    } else {
        actor.screen.move_x(-i16::from(speed));
        actor.screen.x < LEFT_WALL_X
    }
}

/// The states 1 and 2 every pig shares.
fn stun_or_defeat(actor: &mut Actor, context: &Context, effects: &mut Effects) -> bool {
    match actor.state {
        1 => update_hit_stun(actor, context.frame, context.stun),
        2 => {
            actor.animate_two_frame_toggle(context.frame, 2);
            effects.morph = true;
        }
        _ => return false,
    }
    true
}

/// Anims 10 and 11.
pub(super) fn update_part(
    actor: &mut Actor,
    part: Part,
    context: Context,
    fall: &[u8],
) -> Result<Effects, UpdateError> {
    let mut effects = Effects::default();
    if context.hit_processed {
        part_hit(actor, &context, &mut effects);
        return Ok(effects);
    }
    if actor.state == 0 {
        flip_blink(actor, context.frame);
        // Only Pig 1 waits for the fade (`$B14E`); Pig 2's intro has no such call.
        if part == Part::Straw && !context.fade_idle {
            return Ok(effects);
        }
        if part == Part::Wood {
            // `$9D63`: wait until this is the last part, then a short delay.
            if context.definitions_left != 1 {
                return Ok(effects);
            }
            effects.release_remove_mode = true;
            actor.timer = actor.timer.wrapping_add(1);
            if actor.timer != INTRO_WAIT {
                return Ok(effects);
            }
        }
        arm(actor, &mut effects);
        return Ok(effects);
    }
    if stun_or_defeat(actor, &context, &mut effects) {
        return Ok(effects);
    }
    match (part, actor.state) {
        (Part::Straw, 3) => {
            actor.screen.move_x(1);
            bob(actor, fall)?;
        }
        (Part::Straw, 4 | 5) => {
            let speed = 4u8.wrapping_sub(actor.activity >> 1);
            run_cycle(actor, context.frame);
            if walk(actor, actor.state == 4, speed) {
                turn_at_wall(actor, &mut effects);
            }
        }
        (Part::Wood, 3..=5) => {
            effects.sound1 = super::pig2_motion::update(actor, fall)
                .map_err(|_| UpdateError::InvalidMotionProfile)?;
        }
        _ => return Err(UpdateError::UnsupportedState),
    }
    Ok(effects)
}

/// Anim 12, Pig 3: waits for both parts, spawns its brick house (anim 13), then
/// leaps across the arena, faster as the shared hit counter falls.
pub(super) fn update_controller(
    actor: &mut Actor,
    context: Context,
    rise: &[u8],
    fall: &[u8],
) -> Result<Effects, UpdateError> {
    let mut effects = Effects::default();
    if context.hit_processed {
        effects.save_hit = Some((actor.sprite, actor.state, actor.contact_kind));
        if context.stomp_latched {
            actor.sprite = 0x1F;
            actor.timer = context.stun_reload;
        }
        actor.contact_kind = ContactKind::NONE;
        actor.state = 1;
        return Ok(effects);
    }
    if actor.state == 0 {
        flip_blink(actor, context.frame);
        if context.definitions_left != 0 {
            return Ok(effects);
        }
        effects.release_remove_mode = true;
        actor.timer = actor.timer.wrapping_add(1);
        if actor.timer != INTRO_WAIT {
            return Ok(effects);
        }
        effects.spawn_child = Some((0x0D, 0x0E));
        arm(actor, &mut effects);
        return Ok(effects);
    }
    if stun_or_defeat(actor, &context, &mut effects) {
        return Ok(effects);
    }
    match actor.state {
        3 => {
            actor.timer = 8;
            bob(actor, fall)?;
        }
        4 | 5 => {
            // `$9E6E`: an eight-frame crouch on landing.
            if actor.timer != 0 {
                actor.sprite = 0x28;
                actor.timer = actor.timer.wrapping_sub(1);
                if actor.timer != 0 {
                    return Ok(effects);
                }
                actor.sprite = 0x29;
            }
            if !context.frame.every(2) {
                return Ok(effects);
            }
            let speed = 4u8.wrapping_sub(context.hits_remaining >> 1);
            if walk(actor, actor.state == 5, speed) {
                turn_at_wall(actor, &mut effects);
                return Ok(effects);
            }
            // `$9E9A`: a higher arc than Pig 2's, two pixels more per step.
            let profile = if actor.motion_leg == 0 { rise } else { fall };
            let step = actor
                .motion_step(profile, 15)
                .map_err(|_| UpdateError::InvalidMotionProfile)?
                .wrapping_add(2);
            actor.screen.move_y(if actor.motion_leg == 0 {
                -i16::from(step)
            } else {
                i16::from(step)
            });
            if actor.motion_phase == 15 {
                actor.motion_phase = 0;
                if actor.motion_leg == 0 {
                    actor.motion_leg = 1;
                } else {
                    actor.motion_leg = 0;
                    actor.timer = 8;
                    effects.sound1 = Some(actor.definition);
                }
            }
        }
        _ => return Err(UpdateError::UnsupportedState),
    }
    Ok(effects)
}

/// Anim 2, `$9606`: while the fight is in remove mode 2, spawn one reward child
/// (anim 4 / sprite 9) and park. Its later state bodies belong to another table.
pub(super) fn update_spawner(actor: &mut Actor, remove_mode: u8) -> Option<(u8, u8)> {
    if actor.state != 0 || remove_mode != 2 {
        return None;
    }
    actor.contact_kind = ContactKind::NONE;
    actor.state = 1;
    actor.sprite = 7;
    Some((4, 9))
}

/// Anim 13, `$9EE3`: in remove mode 2 the brick house becomes the reward
/// (anim 4 / sprite 9) at the arena centre (`$98F8`).
pub(super) fn update_child(actor: &mut Actor, remove_mode: u8) {
    if remove_mode != 2 {
        return;
    }
    actor.screen.y = 0x90;
    actor.screen.x = 0x58;
    actor.definition = 0x04;
    actor.sprite = 0x09;
    actor.state = 0;
    actor.motion_phase = 0;
    actor.contact_kind = ContactKind::NONE;
}
