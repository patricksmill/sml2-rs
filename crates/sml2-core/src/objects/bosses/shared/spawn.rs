//! Shared boss child spawning, ROM `$B224..$B263`.

use crate::actor::{Actor, SpriteFlipFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct BossSpawnRequest {
    pub animation: u8,
    pub sprite: u8,
    pub flip: SpriteFlipFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BossSpawnError {
    pub count: u8,
}

/// Append the boss's next child, preserving all target fields not written by ROM.
/// Count fifteen is a normal capacity abort; both normal paths consume the request.
/// Counts outside the source domain leave every input untouched and return an error.
pub(super) fn spawn_child(
    parent: &Actor,
    actors: &mut [Actor; 16],
    count: &mut u8,
    request: &mut BossSpawnRequest,
) -> Result<Option<usize>, BossSpawnError> {
    if *count > 15 {
        return Err(BossSpawnError { count: *count });
    }
    let child = if *count == 15 {
        None
    } else {
        let child = usize::from(*count) + 1;
        let target = &mut actors[child];
        target.definition = request.animation;
        target.spawn_record = parent.spawn_record;
        target.activity = parent.activity;
        target.screen = parent.screen;
        target.sprite = request.sprite;
        target.sprite_flip = request.flip;
        *count += 1;
        Some(child)
    };
    *request = BossSpawnRequest::default();
    Ok(child)
}

use super::{ObjectSim, Staging};

impl ObjectSim {
    pub(super) fn apply_boss_spawn(&mut self, st: &Staging) {
        let parent = Actor::from_raw_slot(&st.b);
        let mut actors = std::array::from_fn(|i| Actor::from_raw_slot(&self.slots[i].b));
        let mut request = BossSpawnRequest {
            animation: self.spawn_anim,
            sprite: self.spawn_sprite,
            flip: SpriteFlipFlags::from_bits(self.spawn_attr),
        };
        let result = spawn_child(&parent, &mut actors, &mut self.boss_def_count, &mut request);
        match result {
            Ok(child) => {
                if let Some(child) = child {
                    self.slots[child].b = actors[child].to_raw_slot();
                }
                self.spawn_anim = request.animation;
                self.spawn_sprite = request.sprite;
                self.spawn_attr = request.flip.bits();
            }
            Err(_) => self.note_unported(0xB224),
        }
    }
}

#[cfg(test)]
#[path = "tests/spawn.rs"]
mod tests;
