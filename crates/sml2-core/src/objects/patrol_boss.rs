//! Boss anim 14, ROM `$9EF4` (bank 2 CPU `$5EF4`).
use super::{
    boss_morph,
    boss_stun::{self, HitStunContext},
};
use crate::actor::{Actor, ContactKind, FrameTick, ScreenPosition};

pub(super) struct Inputs {
    pub frame: FrameTick,
    pub div: u8,
    pub fade_idle: bool,
    pub hit: Option<bool>, // Some(true) is a stomp; Some(false) a projectile.
    pub primary: bool,
    pub stun: HitStunContext,
    pub children_busy: [bool; 2],
    pub scene_slot_parity: u8,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SavedHit {
    pub sprite: u8,
    pub state: u8,
    pub contact: ContactKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChildSlot {
    First,
    Second,
}

impl ChildSlot {
    pub(super) fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ChildCommand {
    pub slot: ChildSlot,
    pub screen: ScreenPosition,
    pub sprite: u8,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Effects {
    pub saved_hit: Option<SavedHit>,
    pub sound1: Option<u8>,
    pub silence_music: bool,
    pub child: Option<ChildCommand>,
    pub scene_slot_parity: Option<u8>,
}

pub(super) fn update(actor: &mut Actor, input: Inputs) -> Result<Effects, u8> {
    let mut effects = Effects::default();
    if let Some(stomp) = input.hit {
        effects.saved_hit = Some(SavedHit {
            sprite: actor.sprite,
            state: actor.state,
            contact: actor.contact_kind,
        });
        if stomp {
            actor.sprite = 48;
            actor.timer = 104;
        }
        actor.contact_kind = ContactKind::NONE;
        actor.state = 1;
        return Ok(effects);
    }
    match actor.state {
        0 => {
            if input.fade_idle {
                actor.state = 3;
                actor.contact_kind = ContactKind::from_raw(69);
                actor.motion_leg = 32;
                actor.set_travel_span(32);
            }
        }
        1 => boss_stun::update_hit_stun(actor, input.frame, input.stun),
        2 => effects.silence_music = boss_morph::update(actor, input.frame) && input.primary,
        3 => {
            actor.screen.move_y(-2);
            actor.motion_leg = actor.motion_leg.wrapping_sub(2);
            if actor.motion_leg == 0 {
                actor.defeat_mode = 0;
                actor.state = 4;
                actor.motion_leg = actor.travel_span();
                actor.sprite = 46;
                effects.sound1 = Some(7);
            }
        }
        4 | 6 => {
            if !input.frame.every(2) {
                return Ok(effects);
            }
            actor.screen.move_y(-1);
            actor.motion_leg = actor.motion_leg.wrapping_sub(1);
            if actor.motion_leg != 0 {
                return Ok(effects);
            }
            let slot = if input.scene_slot_parity == 0 {
                ChildSlot::First
            } else {
                ChildSlot::Second
            };
            if input.div & 3 != 0 && !input.children_busy[slot.index()] {
                effects.child = Some(ChildCommand {
                    slot,
                    screen: actor.screen,
                    sprite: 49,
                });
                effects.scene_slot_parity = Some(input.scene_slot_parity ^ 1);
                actor.sprite = 47;
                actor.timer = 48;
            } else {
                actor.sprite = 44;
            }
            finish_leg(actor, &mut effects);
        }
        5 | 7 => {
            if actor.timer != 0 {
                actor.timer -= 1;
                if actor.timer != 0 {
                    return Ok(effects);
                }
                actor.sprite = 44;
            }
            actor.animate_two_frame_toggle(input.frame, 4);
            if !input.frame.every(4) {
                return Ok(effects);
            }
            let right = actor.state == 5;
            if (right && actor.screen.x >= 160) || (!right && actor.screen.x < 16) {
                actor.state ^= 2;
                actor.sprite_flip.toggle_horizontal();
                return Ok(effects);
            }
            actor.screen.move_x(if right { 1 } else { -1 });
            actor.screen.move_y(2);
            actor.motion_leg = actor.motion_leg.wrapping_sub(2);
            if actor.motion_leg == 0 {
                actor.sprite = 46;
                finish_leg(actor, &mut effects);
            }
        }
        state => return Err(state),
    }
    Ok(effects)
}

fn finish_leg(actor: &mut Actor, effects: &mut Effects) {
    actor.motion_leg = actor.travel_span();
    actor.state ^= 1;
    if actor.state & 1 == 0 {
        effects.sound1 = Some(7);
    }
}

#[cfg(test)]
#[path = "patrol_boss_tests.rs"]
mod tests;
