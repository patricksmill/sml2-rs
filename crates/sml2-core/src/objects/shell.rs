//! Compatibility boundary for the native carried and kicked shell.

use super::shell_behavior::{self as behavior, CarriedEffect, Carrier, KickedEffect, ShellWorld};
use super::*;
use crate::actor::{Actor, FrameTick};

/// Level probes and the knock-out scan for a native shell; the slot's
/// position is synced first, since the ROM probes read the slot.
struct SimShellWorld<'a> {
    sim: &'a mut ObjectSim,
    st: &'a mut Staging,
    layout: &'a LevelLayout,
}

impl SimShellWorld<'_> {
    fn sync(&mut self, actor: &Actor) {
        set_x(&mut self.st.b, actor.position.x);
        set_y(&mut self.st.b, actor.position.y);
    }
}

impl ShellWorld for SimShellWorld<'_> {
    fn probe_left_wall_low(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_left_wall_low(self.st, self.layout);
        self.sim.move_flags & 0x02 != 0
    }
    fn probe_right_wall_low(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_right_wall_low(self.st, self.layout);
        self.sim.move_flags & 0x01 != 0
    }
    fn probe_ground(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_ground_own(self.st, self.layout);
        self.sim.move_flags & 0x08 != 0
    }
    fn probe_ceiling(&mut self, actor: &Actor) -> bool {
        self.sync(actor);
        self.sim.m_ceil_own(self.st, self.layout, 8);
        self.sim.move_flags & 0x04 != 0
    }
    fn knock_out_overlaps(&mut self, _actor: &Actor) -> bool {
        // The scan reads the slot's screen position, which no update moves.
        self.sim.ko_overlap(self.st);
        self.sim.object_hit_flag != 0
    }
}

impl ObjectSim {
    pub(super) fn handler_carried(
        &mut self,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let carrier = Carrier {
            x: self.mario.x,
            y: self.mario.y,
            big: self.mario.powerup != 0,
            ducking: self.mario.ducking,
            travelling: self.mario.travelling,
            b_held: self.mario.b_held,
        };
        let mut actor = Actor::from_raw_slot(&st.b);
        let effect = behavior::update_carried(
            &mut actor,
            carrier,
            &mut SimShellWorld {
                sim: self,
                st,
                layout,
            },
        );
        st.b = actor.to_raw_slot();
        match effect {
            CarriedEffect::None => {}
            // $58444: pipe travel -> sActionLatch = 0, `$5B2D7` removal.
            CarriedEffect::RemoveForPipe => {
                self.action_latch = 0;
                self.remove_object(st, tables);
            }
            CarriedEffect::Defeat => self.release_and_defeat(st),
            // B released: `$3AB4` (`$85DC`) dropped the anim; re-derive
            // (`$85DC` -> `$863C` -> `$8240`), then the placement.
            CarriedEffect::Released => {
                self.action_latch = 0;
                self.finish_sprite_swap(st, tables);
                self.kick_placement(st, layout);
            }
        }
    }

    pub(super) fn release_and_defeat(&mut self, st: &mut Staging) {
        self.action_latch = 0;
        self.defeat(st);
    }

    pub(super) fn ko_overlap(&mut self, st: &Staging) {
        let sy = st.b[B_SCRY];
        let sx = st.b[B_SCRX];
        for j in 0..SLOT_COUNT {
            let ty = self.slots[j].b[B_TYPE];
            if ty == 0 || ty == 0x5F || ty & 0xA0 != 0 {
                continue;
            }
            if self.view_margin != (0, 0) {
                let near = |a: u16, b: u16| (a.wrapping_sub(b) as i16).unsigned_abs() < 0x80;
                let o = &self.slots[j];
                if !near(o.x(), st.x()) || !near(o.y(), st.y()) {
                    continue;
                }
            }
            let b = &self.slots[j].b;
            let oy = b[B_SCRY];
            let ox = b[B_SCRX];
            let d = b[B_HB_TOP].wrapping_add(3).wrapping_add(oy);
            let e = b[B_HB_BOTTOM]
                .wrapping_add(10)
                .wrapping_add(oy)
                .wrapping_sub(d);
            if sy.wrapping_sub(d) >= e {
                continue;
            }
            let d = b[B_HB_LEFT].wrapping_sub(5).wrapping_add(ox);
            let e = b[B_HB_RIGHT]
                .wrapping_add(5)
                .wrapping_add(ox)
                .wrapping_sub(d);
            if sx.wrapping_sub(d) >= e {
                continue;
            }
            self.slots[j].b[B_FLAGS] |= 0x02;
            self.object_hit_flag = 1;
        }
    }

    pub(super) fn kick_placement(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let facing_right = self.mario.dir_right;
        behavior::place_kick(
            &mut actor,
            facing_right,
            &mut SimShellWorld {
                sim: self,
                st,
                layout,
            },
        );
        st.b = actor.to_raw_slot();
    }

    /// `$5839B` (anim 3): the kicked shell.
    pub(super) fn handler_kicked(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let mario_ref_x = self.mario_ref_x;
        let effect = behavior::update_kicked(
            &mut actor,
            frame,
            mario_ref_x,
            &mut SimShellWorld {
                sim: self,
                st,
                layout,
            },
        );
        st.b = actor.to_raw_slot();
        match effect {
            KickedEffect::Faced(side) => {
                self.facing = match side {
                    std::cmp::Ordering::Greater => 1,
                    std::cmp::Ordering::Less => 0xFF,
                    std::cmp::Ordering::Equal => 0,
                };
            }
            KickedEffect::Bumped => self.sfx0 = 2,
            KickedEffect::None => {}
        }
    }
}
