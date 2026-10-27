//! Groundwalker slot conversion and legacy terrain effects.
use super::{groundwalker::*, *};
use crate::actor::{Actor, FrameTick, Position};

fn from_actor(actor: &Actor) -> GroundWalker {
    GroundWalker {
        position: actor.position,
        sprite: actor.sprite,
        sprite_flip: actor.sprite_flip,
        flash: actor.flash,
        contact_kind: actor.contact_kind,
        animation_frame: actor.animation_frame,
        state: match actor.state {
            0 => GroundWalkerState::Init,
            1 => GroundWalkerState::WalkLeft,
            2 => GroundWalkerState::WalkRight,
            value => GroundWalkerState::Unknown(value),
        },
        remaining_pixels: actor.travel_span(),
        pause_ticks: actor.timer,
    }
}

fn write_actor(walker: &GroundWalker, actor: &mut Actor) {
    actor.position = walker.position;
    actor.sprite = walker.sprite;
    actor.sprite_flip = walker.sprite_flip;
    actor.flash = walker.flash;
    actor.contact_kind = walker.contact_kind;
    actor.animation_frame = walker.animation_frame;
    actor.state = match walker.state {
        GroundWalkerState::Init => 0,
        GroundWalkerState::WalkLeft => 1,
        GroundWalkerState::WalkRight => 2,
        GroundWalkerState::Unknown(value) => value,
    };
    actor.set_travel_span(walker.remaining_pixels);
    actor.timer = walker.pause_ticks;
}

struct GroundWalkerSimWorld<'a> {
    sim: &'a mut ObjectSim,
    staging: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl GroundWalkerWorld for GroundWalkerSimWorld<'_> {
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
    pub(super) fn handler_groundwalker88(&mut self, staging: &mut Staging, layout: &LevelLayout) {
        let mut legacy = Actor::from_raw_slot(&staging.b);
        let mut walker = from_actor(&legacy);
        let context = GroundWalkerContext {
            frame: self.frame_tick(),
            mario_ref_x: self.mario_ref_x,
        };
        let effect = groundwalker::update(
            &mut walker,
            context,
            &mut GroundWalkerSimWorld {
                sim: self,
                staging,
                layout,
            },
        );
        if let GroundWalkerEffect::Initialized { facing } = effect {
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
#[path = "../../enemies/tests/groundwalker_adapter.rs"]
mod tests;
