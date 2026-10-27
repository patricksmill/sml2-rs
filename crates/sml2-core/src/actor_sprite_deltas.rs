//! Source animation tables indexed after advancing the persistent phase.
use super::{Actor, AnimationFrameError, FrameTick};

impl Actor {
    pub fn animate_sprite_delta_cycle(
        &mut self,
        frame: FrameTick,
        period: u64,
        deltas: &[i8],
    ) -> Result<(), AnimationFrameError> {
        if !frame.every(period) {
            return Ok(());
        }
        self.animation_frame = self.animation_frame.wrapping_add(1);
        if usize::from(self.animation_frame) == deltas.len() {
            self.animation_frame = 0;
        }
        let delta = deltas
            .get(usize::from(self.animation_frame))
            .ok_or(AnimationFrameError {
                frame: self.animation_frame,
            })?;
        self.sprite = self.sprite.wrapping_add_signed(*delta);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sprite_delta_cycle_literal_boundaries_and_delays() {
        for period in [2, 4] {
            for (phase, next, sprite) in
                [(0, 1, 1), (1, 2, 1), (2, 3, 1), (3, 0, 253), (255, 0, 253)]
            {
                let mut slot = [0xa5; 32];
                slot[26] = phase;
                slot[11] = 0;
                let mut actor = Actor::from_raw_slot(&slot);
                actor
                    .animate_sprite_delta_cycle(FrameTick::new(1), period, &[-3, 1, 1, 1])
                    .unwrap();
                assert_eq!(actor.to_raw_slot(), slot);
                actor
                    .animate_sprite_delta_cycle(FrameTick::new(4), period, &[-3, 1, 1, 1])
                    .unwrap();
                slot[26] = next;
                slot[11] = sprite;
                assert_eq!(actor.to_raw_slot(), slot);
            }
        }
        let mut actor = Actor::default();
        actor.animation_frame = 4;
        assert_eq!(
            actor.animate_sprite_delta_cycle(FrameTick::new(0), 2, &[-3, 1, 1, 1]),
            Err(AnimationFrameError { frame: 5 })
        );
        assert_eq!((actor.animation_frame, actor.sprite), (5, 0));
        actor.animation_frame = 255;
        actor.sprite = 0;
        actor
            .animate_sprite_delta_cycle(FrameTick::new(0), 1, &[-2, 1, 1])
            .unwrap();
        assert_eq!((actor.animation_frame, actor.sprite), (0, 254));
        actor.animation_frame = 255;
        assert_eq!(
            actor.animate_sprite_delta_cycle(FrameTick::new(0), 2, &[]),
            Err(AnimationFrameError { frame: 0 })
        );
    }
}
