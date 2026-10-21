//! Shared boss hit stun, ROM `$9CAC`, with defeat handoff `$9435`.

use crate::actor::{Actor, ContactKind, FrameTick};

#[derive(Debug, Clone, Copy)]
pub(super) struct HitStunContext {
    pub removal_pending: bool,
    pub definition_sprite: u8,
    pub saved_sprite: u8,
    pub saved_state: u8,
    pub saved_contact_kind: ContactKind,
}

pub(super) fn update_hit_stun(actor: &mut Actor, frame: FrameTick, context: HitStunContext) {
    actor.flash.animate_palette(frame);
    if actor.timer != 0 {
        actor.timer -= 1;
        if actor.timer != 0 {
            return;
        }
    }
    if context.removal_pending {
        actor.state = 2;
        actor.timer = 32;
        actor.sprite = context.definition_sprite;
    } else {
        actor.flash.clear();
        actor.sprite = context.saved_sprite;
        actor.state = context.saved_state;
        actor.contact_kind = context.saved_contact_kind;
    }
}

// Snapshot the legacy world's reaction context at the conversion boundary;
// every production caller now builds its own context, so this serves tests.
#[cfg(test)]
mod adapter {
    use super::*;
    use crate::objects::{ObjectSim, Staging};

    impl ObjectSim {
        pub(in crate::objects) fn apply_boss_hit_stun(&self, st: &mut Staging) {
            let context = HitStunContext {
                removal_pending: self.remove_mode != 0,
                definition_sprite: self.boss_sprite,
                saved_sprite: self.spawn_sprite,
                saved_state: self.saved_frame_state,
                saved_contact_kind: ContactKind::from_raw(self.saved_type),
            };
            let mut actor = Actor::from_raw_slot(&st.b);
            update_hit_stun(&mut actor, FrameTick(u64::from(self.fff0)), context);
            st.b = actor.to_raw_slot();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::*;

    #[test]
    fn boss_hit_stun_all_cadences_and_timer_boundaries_preserve_slot_bytes() {
        for frame in 0..=255 {
            for timer in [0, 1, 2, 255] {
                for remove_mode in [0, 1, 255] {
                    let mut sim = ObjectSim::new(Vec::new(), 0);
                    sim.fff0 = frame;
                    sim.remove_mode = remove_mode;
                    sim.boss_sprite = 0xA6;
                    sim.spawn_sprite = 0xD7;
                    sim.saved_frame_state = 0xE8;
                    sim.saved_type = 0xF9;
                    sim.sfx0 = 0x34;
                    sim.sfx1 = 0x56;
                    sim.music_request = Some(0x78);
                    let world_before = format!("{sim:?}");
                    let mut st = Staging {
                        b: std::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(0x93)),
                        px: 0x1234,
                        py: 0xFEDC,
                    };
                    st.b[B_T28] = timer;
                    let mut expected = st.b;
                    if frame % 4 == 0 {
                        expected[B_FLASH] ^= 0x10;
                    }
                    expected[B_T28] = timer.saturating_sub(1);
                    if timer <= 1 {
                        if remove_mode != 0 {
                            expected[B_FSTATE] = 2;
                            expected[B_T28] = 32;
                            expected[B_SPRITE] = 0xA6;
                        } else {
                            expected[B_FLASH] = 0;
                            expected[B_SPRITE] = 0xD7;
                            expected[B_FSTATE] = 0xE8;
                            expected[B_TYPE] = 0xF9;
                        }
                    }
                    sim.boss_hit_stun(&mut st);
                    assert_eq!(
                        st.b, expected,
                        "frame {frame}, timer {timer}, remove {remove_mode}"
                    );
                    assert_eq!((st.px, st.py), (0x1234, 0xFEDC));
                    assert_eq!(format!("{sim:?}"), world_before);
                }
            }
        }
    }

    #[test]
    fn boss_hit_stun_native_recovery_preserves_unknown_saved_values() {
        for saved in 0..=255 {
            let mut actor = Actor::from_raw_slot(&[0xA5; 32]);
            actor.timer = 1;
            update_hit_stun(
                &mut actor,
                FrameTick(256),
                HitStunContext {
                    removal_pending: false,
                    definition_sprite: 0,
                    saved_sprite: saved,
                    saved_state: saved,
                    saved_contact_kind: ContactKind::from_raw(saved),
                },
            );
            let mut expected = [0xA5; 32];
            expected[B_T28] = 0;
            expected[B_FLASH] = 0;
            expected[B_SPRITE] = saved;
            expected[B_FSTATE] = saved;
            expected[B_TYPE] = saved;
            assert_eq!(actor.to_raw_slot(), expected);
        }
    }
}
