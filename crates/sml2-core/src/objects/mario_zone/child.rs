//! Mario Zone boss child (anim 13), ROM `$9EE3`.

use crate::objects::{
    ObjectSim, Staging, B_DEF, B_FSTATE, B_PHASE, B_SCRX, B_SCRY, B_SPRITE, B_TYPE,
};

impl ObjectSim {
    /// `$9EE3` (boss anim 13): the child spawned by def-2's controller.
    pub(in crate::objects) fn handler_boss_anim13(&mut self, st: &mut Staging) {
        if self.remove_mode != 2 {
            return; // $9EE4
        }
        st.b[B_SCRY] = 0x90; // $9EE9
        st.b[B_SCRX] = 0x58; // $9EED
                             // $9EF1 -> $98F8: become anim 4 / sprite 9, state, phase and type cleared.
        st.b[B_DEF] = 0x04;
        st.b[B_SPRITE] = 0x09;
        st.b[B_FSTATE] = 0;
        st.b[B_PHASE] = 0;
        st.b[B_TYPE] = 0;
    }
}
