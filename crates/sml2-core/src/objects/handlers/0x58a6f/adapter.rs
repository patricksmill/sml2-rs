//! Raw slot conversion and legacy world outputs for ROM $58A6F.
use super::{
    handler_0x58a6f::{self, World},
    terrain_probes::TerrainProbes,
    *,
};
use crate::actor::{Actor, FrameTick, Position};

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    probes: TerrainProbes<'a>,
}
impl World for Inputs<'_> {
    fn player_x(&self) -> u16 {
        self.sim.mario_ref_x
    }
    fn face_player(&mut self, position: Position) {
        self.sim.facing = match self.player_x().cmp(&position.x) {
            std::cmp::Ordering::Less => 255,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
    }
    fn divider(&self) -> u8 {
        self.sim.div
    }
    fn wall_blocked(&mut self, position: Position, right: bool) -> bool {
        self.probes.wall_blocked(position, right, 7)
    }
}
impl ObjectSim {
    pub(super) fn handler_0x58a6f(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let mut inputs = Inputs {
            sim: self,
            probes: TerrainProbes::new(layout, Position { x: st.px, y: st.py }),
        };
        let result = handler_0x58a6f::update(&mut actor, frame, &mut inputs);
        let probe = inputs.probes.position();
        st.px = probe.x;
        st.py = probe.y;
        if let Some(flags) = inputs.probes.legacy_move_flags() {
            inputs.sim.move_flags = flags;
        }
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x58A6F);
        }
    }
}

#[cfg(test)]
#[path = "tests/behavior.rs"]
mod tests;
