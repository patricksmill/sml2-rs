//! Shared death-animation handoff, ROM `$9443..$9462` (bank 2).
use crate::actor::{Actor, FrameTick};

/// Return true on the frame the source changes this actor to anim 3.
pub(super) fn update(actor: &mut Actor, frame: FrameTick) -> bool {
    actor.flash.animate_palette(frame);
    actor.timer = actor.timer.wrapping_sub(1);
    if actor.timer != 0 {
        return false;
    }
    actor.state = 0;
    actor.definition = 3;
    actor.animation_frame = 0;
    actor.flash.clear();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::*;

    #[test]
    fn boss_morph_source_frame_gates_timer_wrap_and_whole_slot() {
        for frame in 0..=255 {
            for timer in [0u8, 1, 2, 255] {
                let mut expected = [0xA5; 32];
                expected[28] = timer;
                let mut actor = Actor::from_raw_slot(&expected);
                assert_eq!(update(&mut actor, FrameTick(frame)), timer == 1);
                if frame % 4 == 0 {
                    expected[14] ^= 0x10;
                }
                expected[28] = timer.wrapping_sub(1);
                if timer == 1 {
                    expected[15] = 0;
                    expected[5] = 3;
                    expected[26] = 0;
                    expected[14] = 0;
                }
                assert_eq!(actor.to_raw_slot(), expected);
            }
        }
    }

    #[test]
    fn boss_morph_facade_mutes_only_primary_on_handoff() {
        for slot in [0, 0x20, 0x40] {
            for timer in [0u8, 1, 2] {
                let mut sim = ObjectSim::new(vec![], 0);
                sim.boss_slot_ptr_lo = slot;
                sim.music_request = Some(0x77);
                let mut st = Staging::default();
                st.b[B_T28] = timer;
                sim.boss_morph_to_anim3(&mut st);
                assert_eq!(
                    sim.music_request,
                    Some(if slot == 0 && timer == 1 { 0xFF } else { 0x77 })
                );
                assert_eq!(st.b[B_T28], timer.wrapping_sub(1));
                assert_eq!(st.b[B_DEF], if timer == 1 { 3 } else { 0 });
            }
        }
    }
}
