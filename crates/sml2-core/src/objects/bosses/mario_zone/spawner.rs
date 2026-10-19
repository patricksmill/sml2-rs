//! Mario Zone boss spawner (anim 2), ROM `$9606`.

use crate::objects::{ObjectSim, Staging, B_FSTATE, B_SPRITE, B_TYPE};

impl ObjectSim {
    /// `$9606` (boss anim 2): the def-2 spawner. While its remove-mode is 2 it
    /// spawns one child (anim 4 / sprite 9) and parks itself at sprite 7; every
    /// later call returns at the state test, because its state bodies belong to
    /// the def-4 family's table, not to its own.
    pub(in crate::objects) fn handler_boss_def2(&mut self, st: &mut Staging) {
        if st.b[B_FSTATE] != 0 {
            return; // $9606
        }
        if self.remove_mode != 2 {
            return; // $960A
        }
        st.b[B_TYPE] = 0; // $9610
        st.b[B_FSTATE] = 1; // $9613
        self.spawn_anim = 4; // $9617
        self.spawn_sprite = 9; // $961C
        self.boss_spawn_child(st); // $9621
        st.b[B_SPRITE] = 7; // $9624
    }
}
