//! Boss anim 19, ROM `$A372` (bank 2), with a fixed arena circuit.
use super::{
    boss_morph,
    boss_stun::{self, HitStunContext},
    circuit_boss_motion,
};
use crate::actor::{Actor, ContactKind, FlashFlags, FrameTick};

pub(super) struct Inputs {
    pub frame: FrameTick,
    pub div: u8,
    pub fade_idle: bool,
    pub hit: Option<bool>,
    pub primary: bool,
    pub stun: HitStunContext,
}
#[derive(Default)]
pub(super) struct Effects {
    pub saved_hit: Option<(u8, u8, ContactKind)>,
    pub sound1: Option<u8>,
    pub sound2: Option<u8>,
    pub silence_music: bool,
}

pub(super) fn update(actor: &mut Actor, input: Inputs) -> Result<Effects, ()> {
    let mut effects = Effects::default();
    if let Some(stomp) = input.hit {
        effects.saved_hit = Some((actor.sprite, actor.state, actor.contact_kind));
        if stomp {
            actor.sprite = 74;
            actor.timer = 48;
        }
        actor.state = 1;
        actor.contact_kind = ContactKind::NONE;
        return Ok(effects);
    }
    match actor.state {
        0 => {
            if input.fade_idle {
                actor.state = 3;
                actor.contact_kind = ContactKind::from_raw(69);
                actor.set_travel_span(36);
                actor.flash = FlashFlags::from_bits(16);
            }
        }
        1 => boss_stun::update_hit_stun(actor, input.frame, input.stun),
        2 => {
            actor.animate_two_frame_toggle(input.frame, 4);
            effects.silence_music = boss_morph::update(actor, input.frame) && input.primary;
            // `$A426` overwrites the flash even after terminal morph clears it.
            actor.flash = FlashFlags::from_bits(16);
        }
        _ => {
            actor.flash = FlashFlags::from_bits(16);
            actor.animate_four_step_bob(input.frame).map_err(|_| ())?;
            if actor.defeat_mode == 0 && input.frame.every(2) {
                effects.sound2 = Some(7);
                if input.div & 63 == 0 {
                    effects.sound1 = Some(3);
                }
            }
            circuit_boss_motion::update(actor)?;
        }
    }
    Ok(effects)
}

#[cfg(test)]
#[path = "circuit_boss_tests.rs"]
mod tests;
