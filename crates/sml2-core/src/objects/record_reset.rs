//! Legacy staging adapter for the native spawn reset contract.
use super::{record_origin::RecordResetError, ObjectSim, ObjectTables, Staging};
use crate::actor::{Actor, SpriteReset};

impl ObjectSim {
    pub(super) fn reset_record_origin(
        &mut self,
        staging: &mut Staging,
        tables: &ObjectTables,
    ) -> Result<(), RecordResetError> {
        let mut actor = Actor::from_raw_slot(&staging.b);
        // Resolve all fallible inputs before changing staging or global outputs.
        let origin = self.resolve_spawn_origin(actor.spawn_record)?;
        let visuals = if actor.spawn_record >> 8 >= 0xAD {
            SpriteReset::Preserve
        } else {
            let sprite_map = if self.sprite_map_index == 0 {
                &tables.anim_to_sprite_a
            } else {
                &tables.anim_to_sprite_b
            };
            let sprite = sprite_map
                .get(usize::from(actor.definition))
                .copied()
                .ok_or(RecordResetError::MissingSprite {
                    level: self.level_id,
                    definition: actor.definition,
                })?;
            SpriteReset::Rebuild { sprite }
        };
        self.spawn_attr = actor.reset_to_spawn(origin, visuals).bits();
        self.done = 1;
        staging.b = actor.to_raw_slot();
        Ok(())
    }
}
