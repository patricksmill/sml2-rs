//! Paced walker slot conversion and legacy terrain effects.
use super::{paced_walker::*, *};
use crate::actor::{Actor, FrameTick, Position};

fn from_actor(actor: &Actor) -> PacedWalker {
    PacedWalker {
        position: actor.position,
        sprite: actor.sprite,
        sprite_flip: actor.sprite_flip,
        flash: actor.flash,
        contact_kind: actor.contact_kind,
        state: match actor.state {
            0 => PacedWalkerState::Init,
            1 => PacedWalkerState::WalkLeft,
            2 => PacedWalkerState::WalkRight,
            value => PacedWalkerState::Unknown(value),
        },
        remaining_pixels: actor.travel_span(),
        pause_ticks: actor.timer,
    }
}

fn write_actor(walker: &PacedWalker, actor: &mut Actor) {
    actor.position = walker.position;
    actor.sprite = walker.sprite;
    actor.sprite_flip = walker.sprite_flip;
    actor.flash = walker.flash;
    actor.contact_kind = walker.contact_kind;
    actor.state = match walker.state {
        PacedWalkerState::Init => 0,
        PacedWalkerState::WalkLeft => 1,
        PacedWalkerState::WalkRight => 2,
        PacedWalkerState::Unknown(value) => value,
    };
    actor.set_travel_span(walker.remaining_pixels);
    actor.timer = walker.pause_ticks;
}

struct PacedWalkerSimWorld<'a> {
    sim: &'a mut ObjectSim,
    staging: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl PacedWalkerWorld for PacedWalkerSimWorld<'_> {
    fn probe_left_wall(&mut self, position: Position) -> bool {
        set_x(&mut self.staging.b, position.x);
        set_y(&mut self.staging.b, position.y);
        self.sim.m_left_wall(self.staging, self.layout);
        self.sim.move_flags & 0x02 != 0
    }

    fn probe_right_wall(&mut self, position: Position) -> bool {
        set_x(&mut self.staging.b, position.x);
        set_y(&mut self.staging.b, position.y);
        self.sim.m_right_wall(self.staging, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_paced_walker(&mut self, staging: &mut Staging, layout: &LevelLayout) {
        let mut legacy = Actor::from_raw_slot(&staging.b);
        let mut walker = from_actor(&legacy);
        let context = PacedWalkerContext {
            frame: FrameTick(u64::from(self.fff0)),
            mario_ref_x: self.mario_ref_x,
        };
        let effect = paced_walker::update(
            &mut walker,
            context,
            &mut PacedWalkerSimWorld {
                sim: self,
                staging,
                layout,
            },
        );
        if let PacedWalkerEffect::Initialized { facing } = effect {
            self.facing = match facing {
                std::cmp::Ordering::Less => 0xFF,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
        }
        write_actor(&walker, &mut legacy);
        staging.b = legacy.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/paced_walker_adapter.rs"]
mod tests;
