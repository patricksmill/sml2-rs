//! Legacy world inputs and probe effects for the native hopping enemy.
use super::{
    hopping::{self, World},
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
    fn face_player(&mut self, position: Position) -> bool {
        use std::cmp::Ordering;
        match self.player_x().cmp(&position.x) {
            Ordering::Less => {
                self.sim.facing = 255;
                true
            }
            Ordering::Greater => {
                self.sim.facing = 1;
                false
            }
            Ordering::Equal => {
                self.sim.facing = 0;
                // The source returns the compared low coordinate in A on equality.
                self.player_x() as u8 & 0x80 != 0
            }
        }
    }
    fn wall_blocked(&mut self, position: Position, right: bool) -> bool {
        self.probes.wall_blocked(position, right, 7)
    }
    fn grounded(&mut self, position: Position) -> bool {
        self.probes.grounded(position)
    }
    fn rise(&self) -> &[u8] {
        &BOUNCE_SPEED[3..]
    }
    fn fall(&self) -> &[u8] {
        &FALL_SPEED
    }
}

impl ObjectSim {
    pub(super) fn handler_0x58848(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let mut inputs = Inputs {
            sim: self,
            probes: TerrainProbes::new(layout, Position { x: st.px, y: st.py }),
        };
        let result = hopping::update(&mut actor, frame, &mut inputs);
        if let Some(flags) = inputs.probes.legacy_move_flags() {
            inputs.sim.move_flags = flags;
        }
        st.px = inputs.probes.position().x;
        st.py = inputs.probes.position().y;
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x58848);
        }
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/hopping_adapter.rs"]
mod tests;
