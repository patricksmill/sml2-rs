//! Boss anim 16, ROM `$A0B9`, bank 2 CPU `$60B9`.
use super::{
    boss_morph,
    boss_stun::{self, HitStunContext},
};
use crate::actor::{Actor, ContactKind, FrameTick, RiseProfile, ScreenPosition, SpriteFlipFlags};

pub(super) struct Inputs<'a> {
    pub frame: FrameTick,
    pub div: u8,
    pub fade_idle: bool,
    pub hit: Option<bool>,
    pub primary: bool,
    pub stun: HitStunContext,
    pub timer_reload: u8,
    pub volleys: u8,
    pub children_busy: [bool; 2],
    pub hover_profile: &'a [u8],
    pub sweep_profile: &'a [u8],
}

pub(super) struct ChildCommand {
    pub screen: ScreenPosition,
    pub sprite: u8,
    pub flip: SpriteFlipFlags,
}

#[derive(Default)]
pub(super) struct Effects {
    pub saved_hit: Option<(u8, u8, ContactKind)>,
    pub sound1: Option<u8>,
    pub silence_music: bool,
    pub volleys: Option<u8>,
    pub children: [Option<ChildCommand>; 2],
}

pub(super) fn update(actor: &mut Actor, input: Inputs<'_>) -> Result<Effects, ()> {
    let mut effects = Effects::default();
    if let Some(stomp) = input.hit {
        effects.saved_hit = Some((actor.sprite, actor.state, actor.contact_kind));
        actor.contact_kind = ContactKind::NONE;
        actor.state = if stomp { 9 } else { 1 };
        if stomp {
            actor.sprite = 53;
            actor.timer = 1;
        }
        return Ok(effects);
    }
    match actor.state {
        0 => {
            if input.fade_idle {
                actor.state = 3;
                actor.contact_kind = ContactKind::from_raw(69);
                actor.timer = 1;
                effects.volleys = Some(4);
            }
        }
        1 => boss_stun::update_hit_stun(actor, input.frame, input.stun),
        2 => {
            actor.animate_four_frame_cycle(input.frame, 4);
            effects.silence_music = boss_morph::update(actor, input.frame) && input.primary;
        }
        3 | 6 => {
            if input.volleys == 0 {
                actor.motion_leg = 0;
                actor.state += 1;
                actor.motion_phase = 92;
                return Ok(effects);
            }
            hover(actor, input.hover_profile)?;
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                for (index, busy) in input.children_busy.into_iter().enumerate() {
                    if busy {
                        continue;
                    }
                    let mut screen = actor.screen;
                    if index == 0 {
                        screen.move_x(if actor.state == 3 { -8 } else { 8 });
                    } else {
                        screen.move_y(16);
                    }
                    effects.children[index] = Some(ChildCommand {
                        screen,
                        sprite: if index == 0 { 59 } else { 60 },
                        flip: actor.sprite_flip,
                    });
                }
                effects.volleys = Some(input.volleys - 1);
                actor.timer = 96;
            }
        }
        4 | 5 | 7 | 8 => {
            let descending = matches!(actor.state, 4 | 7);
            let right = actor.state >= 7;
            actor.screen.move_y(if descending { 1 } else { -1 });
            // `$B16B` only reads: these states advance the phase themselves.
            let distance = *input
                .sweep_profile
                .get(usize::from(actor.motion_phase))
                .ok_or(())?;
            actor.screen.move_x(if right {
                i16::from(distance)
            } else {
                -i16::from(distance)
            });
            if descending {
                actor.motion_phase = actor.motion_phase.wrapping_sub(1);
                if actor.motion_phase == 0 {
                    actor.motion_phase = 1;
                    actor.state += 1;
                    actor.sprite_flip.toggle_horizontal();
                }
            } else {
                actor.motion_phase = actor.motion_phase.wrapping_add(1);
                if actor.motion_phase == 93 {
                    actor.motion_phase = 0;
                    actor.state = if right { 3 } else { 6 };
                    actor.screen = ScreenPosition {
                        x: if right { 144 } else { 32 },
                        y: 48,
                    };
                    actor.timer = 80;
                    effects.volleys = Some(input.div & 3);
                }
            }
        }
        9 => {
            actor.timer = actor.timer.wrapping_sub(1);
            if actor.timer == 0 {
                effects.sound1 = Some(5);
                actor.state = 1;
                actor.timer = input.timer_reload;
            }
        }
        _ => return Err(()),
    }
    Ok(effects)
}

fn hover(actor: &mut Actor, profile: &[u8]) -> Result<(), ()> {
    let distance = if actor.motion_leg != 0 {
        -i16::from(actor.motion_step(profile, 12).map_err(|_| ())?)
    } else {
        i16::from(actor.rise_step(RiseProfile::Short).map_err(|_| ())?)
    };
    actor.screen.move_y(distance);
    if actor.motion_phase == 12 {
        actor.motion_phase = 0;
        actor.motion_leg ^= 1;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
