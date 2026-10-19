//! Shared boss damage, ROM `$9285` (bank 2 CPU `$5285`).

use crate::actor::InteractionFlags;

#[derive(Debug, Clone, Copy)]
pub(super) struct DamageContext {
    pub primary_slot: bool,
    pub remaining_hits: u8,
    pub definition: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DamageSound {
    Stomp,
    Projectile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Defeat {
    Remove,
    ClearWithFade,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct DamageEffects {
    pub processed_hit: bool,
    pub latch_stomp: bool,
    pub remaining_hits: Option<u8>,
    pub sound: Option<DamageSound>,
    pub defeat: Option<Defeat>,
}

pub(super) fn damage(interactions: InteractionFlags, context: DamageContext) -> DamageEffects {
    let stomp = interactions.stomped();
    if !stomp && !interactions.projectile_defeat() {
        return DamageEffects::default();
    }
    let mut effects = DamageEffects {
        processed_hit: true,
        latch_stomp: stomp,
        ..DamageEffects::default()
    };
    if !context.primary_slot {
        return effects;
    }
    let hits = context
        .remaining_hits
        .wrapping_sub(if stomp { 2 } else { 1 });
    effects.remaining_hits = Some(hits);
    effects.sound = Some(if stomp {
        DamageSound::Stomp
    } else {
        DamageSound::Projectile
    });
    // Projectile DEC preserves the carry cleared by the primary-slot AND at
    // $9298. Its zero-count wrap does not defeat; stomp SUB 2 also checks borrow.
    if hits == 0 || (stomp && context.remaining_hits < 2) {
        effects.defeat = Some(if context.definition < 6 {
            Defeat::ClearWithFade
        } else {
            Defeat::Remove
        });
    }
    effects
}

// Legacy world/staging conversion stays at this boundary. The helper never
// changes an actor, and effects describe only writes made by the source.
mod adapter {
    use super::*;
    use crate::actor::Actor;
    use crate::objects::{ObjectSim, Staging};

    impl ObjectSim {
        pub(in crate::objects) fn apply_boss_damage(&mut self, st: &Staging) {
            let actor = Actor::from_raw_slot(&st.b);
            let effects = damage(
                actor.interactions,
                DamageContext {
                    primary_slot: self.boss_slot_ptr_lo == 0,
                    remaining_hits: self.boss_hits_remaining,
                    definition: self.boss_def_index,
                },
            );
            if effects.processed_hit {
                self.done = 1;
            }
            if effects.latch_stomp {
                self.boss_hit_latch = 1;
            }
            if let Some(hits) = effects.remaining_hits {
                self.boss_hits_remaining = hits;
            }
            match effects.sound {
                Some(DamageSound::Stomp) => self.sfx0 = 0x13,
                Some(DamageSound::Projectile) => self.sfx1 = 9,
                None => {}
            }
            if let Some(defeat) = effects.defeat {
                self.remove_mode = 1;
                if defeat == Defeat::ClearWithFade {
                    self.fade_timer_set = Some(0x0280);
                    self.boss_clear_flag = 1;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/damage.rs"]
mod tests;
