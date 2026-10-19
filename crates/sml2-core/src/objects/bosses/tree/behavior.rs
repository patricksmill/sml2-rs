//! Tree Zone boss, anim 1, ROM `$93A7..$9594` (bank 2).
use super::boss_morph;
use crate::actor::{Actor, ContactKind, FrameTick};

pub(super) struct Inputs<'a> {
    pub frame: FrameTick,
    pub fade_idle: bool,
    pub hit: Option<bool>,
    pub primary: bool,
    pub timer_reload: u8,
    pub removal_pending: bool,
    pub definition_sprite: u8,
    pub saved_sprite: u8,
    pub saved_state: u8,
    pub remaining_hits: u8,
    pub alternate_phase: u8,
    pub entrance_profile: &'a [u8],
    pub sweep_profile: &'a [u8],
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Effects {
    pub saved_hit: Option<(u8, u8, ContactKind)>,
    pub alternate_phase: Option<u8>,
    pub sound1: Option<u8>,
    pub sound2: Option<u8>,
    pub silence_music: bool,
}

pub(super) fn update(actor: &mut Actor, input: Inputs<'_>) -> Result<Effects, ()> {
    let mut effects = Effects::default();
    if let Some(stomp) = input.hit {
        effects.saved_hit = Some((actor.sprite, actor.state, actor.contact_kind));
        if stomp {
            actor.sprite = 0;
            actor.timer = input.timer_reload;
        }
        actor.contact_kind = ContactKind::NONE;
        actor.state = 1;
        return Ok(effects);
    }
    match actor.state {
        0 => {
            actor.animate_sprite_blink(input.frame, 4, 6);
            if input.fade_idle {
                actor.contact_kind = ContactKind::from_raw(69);
                actor.sprite = 3;
                actor.state = 3;
            }
        }
        1 => recover_hit(actor, &input),
        2 => effects.silence_music = boss_morph::update(actor, input.frame) && input.primary,
        3 => {
            let distance = actor
                .motion_step(input.entrance_profile, 12)
                .map_err(|_| ())?;
            actor.screen.move_y(i16::from(distance));
            if actor.motion_phase == 12 {
                actor.motion_phase = 1;
                actor.state = 4;
                effects.sound2 = Some(2);
            }
        }
        4 | 6 => {
            if actor.motion_phase >= 12 {
                actor.screen.move_y(-1);
            }
            let distance = *input
                .sweep_profile
                .get(usize::from(actor.motion_phase))
                .ok_or(())?;
            actor.screen.move_x(if actor.state == 4 {
                i16::from(distance)
            } else {
                -i16::from(distance)
            });
            actor.motion_phase = actor.motion_phase.wrapping_add(1);
            if actor.motion_phase == 105 {
                actor.motion_phase = 104;
                actor.state += 1;
                actor.timer = 80;
                if input.remaining_hits < 3 {
                    effects.alternate_phase = Some(1);
                    actor.motion_phase = 92;
                }
            }
        }
        5 | 7 => {
            if actor.timer != 0 {
                actor.animate_two_frame_toggle(input.frame, 4);
                actor.timer -= 1;
                if actor.timer != 0 {
                    return Ok(effects);
                }
                actor.sprite = 5;
                effects.sound1 = Some(8);
            }
            actor.animate_sprite_blink(input.frame, 4, 6);
            if input.alternate_phase != 0 || actor.motion_phase >= 12 {
                actor.screen.move_y(1);
            }
            let distance = *input
                .sweep_profile
                .get(usize::from(actor.motion_phase))
                .ok_or(())?;
            actor.screen.move_x(if actor.state == 5 {
                -i16::from(distance)
            } else {
                i16::from(distance)
            });
            actor.motion_phase = actor.motion_phase.wrapping_sub(1);
            if actor.motion_phase == 0 {
                actor.motion_phase = 1;
                actor.state = if actor.state == 5 { 6 } else { 4 };
                actor.sprite = 2;
                actor.sprite_flip.toggle_horizontal();
                effects.sound2 = Some(2);
            }
        }
        _ => return Err(()),
    }
    Ok(effects)
}

/// `$93FE` differs from the other bosses' hit recovery: after the countdown,
/// hold sprite 4 for sixteen frames and restore contact type 69.
fn recover_hit(actor: &mut Actor, input: &Inputs<'_>) {
    actor.flash.animate_palette(input.frame);
    if actor.timer != 0 {
        actor.animate_two_frame_toggle(input.frame, 2);
        actor.timer -= 1;
        if actor.timer != 0 {
            return;
        }
    }
    if input.removal_pending {
        actor.state = 2;
        actor.timer = 32;
        actor.sprite = input.definition_sprite;
        return;
    }
    actor.sprite = 4;
    actor.animation_frame = actor.animation_frame.wrapping_add(1);
    if actor.animation_frame == 16 {
        actor.animation_frame = 0;
        actor.flash.clear();
        actor.sprite = input.saved_sprite;
        actor.state = input.saved_state;
        actor.contact_kind = ContactKind::from_raw(69);
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
