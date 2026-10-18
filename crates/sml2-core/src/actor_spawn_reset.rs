//! Native `$5B2BF -> $863C/$8240` reset, after origin resolution.
use super::{Actor, ContactKind, Facing, Hitbox, InteractionFlags, Position, SpriteFlipFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnOrigin {
    pub position: Position,
    pub marker: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteReset {
    Rebuild { sprite: u8 },
    Preserve,
}

impl Actor {
    /// Restore the physical origin. Counters belong to the caller's world.
    /// Returns the observable spawn attribute left by `$863C`.
    pub fn reset_to_spawn(&mut self, origin: SpawnOrigin, visuals: SpriteReset) -> SpriteFlipFlags {
        self.position = origin.position;
        self.marker = origin.marker;
        let mut spawn_attr = self.sprite_flip;
        if let SpriteReset::Rebuild { sprite } = visuals {
            self.sprite = sprite;
            self.flash.clear();
            self.state = 0;
            self.motion_phase = 0;
            self.parked_contact = 0;
            self.contact_kind = ContactKind::NONE;
            self.hitbox = Hitbox::default();
            self.interactions = InteractionFlags::default();
            self.motion_leg = 0;
            self.travel_direction = Facing::LEFT;
            self.animation_frame = 0;
            self.hit_reaction = 0;
            self.timer = 0;
            self.reserved29 = 0;
            self.reserved30 = 0;
            spawn_attr = SpriteFlipFlags::default();
        }
        self.activity = 1;
        spawn_attr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_reset_native_full_dirty_slot_rebuild_and_preserve() {
        let dirty = std::array::from_fn(|i| 0x80 + i as u8);
        for rebuild in [false, true] {
            let mut actor = Actor::from_raw_slot(&dirty);
            let origin = SpawnOrigin { position: Position { x: 0x1234, y: 0x5678 }, marker: 2 };
            let visuals = if rebuild { SpriteReset::Rebuild { sprite: 0x42 } } else { SpriteReset::Preserve };
            let attr = actor.reset_to_spawn(origin, visuals);
            let mut expected = dirty;
            expected[..5].copy_from_slice(&[0x12, 0x34, 2, 0x56, 0x78]);
            expected[8] = 1;
            if rebuild {
                expected[11] = 0x42;
                expected[14..31].fill(0);
            }
            assert_eq!(actor.to_raw_slot(), expected);
            assert_eq!(attr.bits(), if rebuild { 0 } else { dirty[13] });
        }
    }
}
