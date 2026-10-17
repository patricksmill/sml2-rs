//! Legacy world inputs and probe effects for the native hopping enemy.
use super::{
    handler_0x58848::{self, World},
    *,
};
use crate::actor::{Actor, FrameTick, Position};

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    layout: &'a LevelLayout,
    probe: Position,
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
        let flag = if right { 1 } else { 2 };
        self.sim.move_flags = if right { 0x11 } else { 0x22 };
        self.probe = Position {
            x: position.x.wrapping_add_signed(if right { 7 } else { -7 }),
            y: position.y.wrapping_sub(11),
        };
        if ObjectSim::probe(self.layout, self.probe.x, self.probe.y) < 0x38 {
            return true;
        }
        self.probe.y = position.y.wrapping_sub(5);
        if ObjectSim::probe(self.layout, self.probe.x, self.probe.y) < 0x38 {
            return true;
        }
        self.sim.move_flags &= !flag;
        false
    }
    fn grounded(&mut self, position: Position) -> bool {
        self.sim.move_flags = 0x88;
        // The alignment early return writes only the low probe byte.
        self.probe.y = (self.probe.y & 0xff00) | (position.y & 0xff);
        if position.y as u8 & 0x0c != 0 {
            self.sim.move_flags = 0x80;
            return false;
        }
        self.probe.y = position.y;
        self.probe.x = position.x;
        if ObjectSim::probe(self.layout, self.probe.x, self.probe.y) >= 0x48 {
            self.sim.move_flags = 0x80;
            false
        } else {
            true
        }
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
        let frame = FrameTick(u64::from(self.fff0));
        let mut inputs = Inputs {
            sim: self,
            layout,
            probe: Position { x: st.px, y: st.py },
        };
        let result = handler_0x58848::update(&mut actor, frame, &mut inputs);
        st.px = inputs.probe.x;
        st.py = inputs.probe.y;
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x58848);
        }
    }
}

#[cfg(test)]
#[path = "handler_0x58848_adapter_tests.rs"]
mod tests;
