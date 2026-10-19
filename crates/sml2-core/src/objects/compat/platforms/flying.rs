//! Slot conversion, side probes, and the anim-69 child for ROM `$5A545`.
use super::{handler_0x5a545, *};
use crate::actor::{Actor, FrameTick};
use std::cmp::Ordering;

struct Inputs<'a> {
    layout: &'a LevelLayout,
}

impl Inputs<'_> {
    /// `$8554` / `$84A6`: blocked when either sample tile is below `$38`.
    fn blocked(&self, actor: &Actor, dx: i16) -> bool {
        let x = if dx < 0 {
            actor.position.x.wrapping_sub(11)
        } else {
            actor.position.x.wrapping_add(11)
        };
        [19u16, 4].into_iter().any(|dy| {
            let y = actor.position.y.wrapping_sub(dy);
            self.layout
                .get(usize::from((x >> 4) & 255), usize::from((y >> 4) & 255))
                .unwrap_or(0)
                < 0x38
        })
    }
}

impl handler_0x5a545::World for Inputs<'_> {
    fn left_blocked(&mut self, actor: &Actor) -> bool {
        self.blocked(actor, -1)
    }
    fn right_blocked(&mut self, actor: &Actor) -> bool {
        self.blocked(actor, 1)
    }
}

impl ObjectSim {
    pub(super) fn handler_0x5a545(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = FrameTick(u64::from(self.fff0));
        let player_x = self.mario_ref_x;
        let result = handler_0x5a545::update(&mut actor, frame, player_x, &mut Inputs { layout });
        st.b = actor.to_raw_slot();
        match result {
            Ok(handler_0x5a545::Effect::Initialized {
                facing,
                anim,
                sprite,
                flip,
            }) => {
                self.facing = match facing {
                    Ordering::Less => 255,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
                self.spawn_anim = anim;
                self.spawn_sprite = sprite;
                self.spawn_attr = flip.bits();
                self.spawn_dynamic_5b36d(st);
            }
            Ok(handler_0x5a545::Effect::Moved { carry_dx, carry_dy }) => {
                if let Some(dx) = carry_dx {
                    self.carry_mario_dx(dx);
                }
                if let Some(dy) = carry_dy {
                    let y = self.mario_y_adjust.unwrap_or(self.mario.y);
                    self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
                }
            }
            Ok(handler_0x5a545::Effect::Idle) => {}
            Err(_) => self.note_unported(0x5A545),
        }
    }
}

#[cfg(test)]
#[path = "../../platforms/tests/flying.rs"]
mod tests;
