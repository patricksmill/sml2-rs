//! Legacy world boundary for native money bag reward production.
use super::{handler_0x5b36d::DynamicSpawnRequest, reward_spawn, ObjectSim};
use crate::actor::{Actor, SpriteFlipFlags};

impl ObjectSim {
    pub(super) fn add_money_bag_reward(&mut self) {
        if reward_spawn::add_money_bag(&mut self.money_bag) {
            self.spawn_sprite = 28;
            self.spawn_anim = 26;
            self.spawn_reward_from_camera();
            self.sfx0 = 8;
        }
    }

    pub(super) fn spawn_reward_from_camera(&mut self) -> Option<usize> {
        let mut actors = std::array::from_fn(|i| Actor::from_raw_slot(&self.slots[i].b));
        let mut request = DynamicSpawnRequest {
            anim: self.spawn_anim,
            sprite: self.spawn_sprite,
            flip: SpriteFlipFlags::from_bits(self.spawn_attr),
        };
        let result = reward_spawn::spawn_from_camera(
            self.camera_position,
            (self.slot_ptr_hi, self.boss_slot_ptr_lo),
            &mut actors,
            &mut self.standby,
            &mut request,
        );
        if let Some(index) = result {
            self.slots[index].b = actors[index].to_raw_slot();
        }
        self.spawn_anim = request.anim;
        self.spawn_sprite = request.sprite;
        self.spawn_attr = request.flip.bits();
        result
    }
}

#[cfg(test)]
#[path = "../../items/tests/moneybag_reward.rs"]
mod tests;
