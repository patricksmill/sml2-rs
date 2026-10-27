//! Slot conversion for ROM `$59431` (anim 65) and `$5945A` (anim 66).
use super::terrain_adapter::TerrainWorld;
use super::{
    handler_0x59431::{self, Kind},
    *,
};
use crate::actor::Actor;
use std::cmp::Ordering;

impl handler_0x59431::World for TerrainWorld<'_> {
    fn mario_ref_x(&self) -> u16 {
        self.sim.mario_ref_x
    }
    fn set_facing(&mut self, relation: Ordering) {
        self.sim.facing = match relation {
            Ordering::Less => 255,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };
    }
}

impl ObjectSim {
    /// Shared slot adapter for ROM `$59431` (anim 65) and `$5945A` (anim 66).
    /// `entry` keeps an out-of-profile report attributed to the dispatching
    /// entry; `kind` selects that entry's landing contact and fall flash.
    pub(super) fn handler_landing_walker(
        &mut self,
        entry: u32,
        kind: Kind,
        st: &mut Staging,
        layout: &LevelLayout,
    ) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let result = {
            let mut world = TerrainWorld::new(self, st, layout);
            handler_0x59431::update(&mut actor, frame, &mut world, kind)
        };
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(entry);
        }
    }

    pub(super) fn handler_0x59431(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.handler_landing_walker(0x59431, handler_0x59431::ANIM_65, st, layout);
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/landing_walker.rs"]
mod tests;
