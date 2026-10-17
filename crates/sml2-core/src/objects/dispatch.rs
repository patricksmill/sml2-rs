//! Object dispatch routines.

use super::*;

impl ObjectSim {
    pub(super) fn dispatch_handler(
        &mut self,
        handler: u32,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        match handler {
            0x58201 => self.handler_walker(st, layout),
            0x582A9 => self.handler_squashed(st, tables),
            0x582D1 => self.handler_fallwalker(st, layout),
            0x5834F => self.handler_stun(st, layout, tables),
            0x584E0 => self.handler_bobber(st, tables),
            0x5862E => self.handler_crawler(st, layout),
            0x586E1 => self.handler_marker85(st),
            0x586EE => self.handler_marker66(st),
            0x586F5 => self.handler_pickup16(st, tables),
            0x58712 => self.handler_hopper(st, layout, tables),
            0x58755 => self.handler_itempose20(st),
            0x58758 => self.itempose_body(st),
            0x587BD => self.handler_pose86(st),
            0x587D7 => self.handler_pose87(st, tables),
            0x5839B => self.handler_kicked(st, layout),
            0x58444 => self.handler_carried(st, layout, tables),
            0x58DD1 => self.handler_carrier46(st, layout, tables),
            0x594EB => self.handler_fallwalker52(st, layout),
            0x59BC5 => self.handler_groundwalker88(st, layout),
            0x5B1C8 => self.handler_grower92(st),
            0x5B241 => self.handler_floater94(st),
            0x5B28C => self.handler_floater95(st),
            0x586EB => self.handler_0x586eb(st),
            0x5B0D5 => self.handler_0x5b0d5(st),
            // $58100 — a bare `ret` (ROM bytes: the single byte `$C9` right after
            // the 192-byte mode-A table at $58040). Nineteen mode-A indices and
            // eleven mode-B ones dispatch here, and it is a *real* target, not a
            // missing one: the $58000 prelude still runs (interaction, bounce,
            // star), only the per-anim body does nothing. Dispatching it
            // explicitly keeps it out of `note_unported`, where it would look
            // like a gap in the port rather than a fixture of the ROM.
            0x58100 => {}
            0x581B1 => self.handler_death(st, layout, tables),
            // The boss table's own arms (bank-2 addresses — see
            // `ObjectTables::boss_anim_handlers`). anim 0 is a bare `ret` at
            // $9322, the same inert-anim idiom as the level table's $58100.
            0x9322 => {}
            0x9323 => self.handler_boss_anim3(st, tables),
            0x93A7 => self.handler_boss_def0_main(st, layout, tables),
            0x9C36 => self.handler_boss_def2_part(st, layout, tables, false),
            0x9D4F => self.handler_boss_def2_part(st, layout, tables, true),
            0x9E07 => self.handler_boss_def2_controller(st, layout, tables),
            0x9606 => self.handler_boss_def2(st),
            0x9642 => self.handler_boss_anim4(st, tables),
            0x9698 => self.handler_boss_anim5(st, tables),
            0x9EE3 => self.handler_boss_anim13(st),
            0xAB30 => self.boss_anim22(st),
            0xAB6E => self.boss_anim25(st),
            0xA8AB => self.handler_wario20(st, tables),
            0xAB8B => self.handler_wario23(st, tables),
            0xAAA5 => self.handler_wario21(st, tables),
            0xAD93 => self.handler_wario26(st, tables),
            0xAFE0 => self.handler_wario27(st),
            0xB0C2 => self.handler_wario28(st, tables),
            h => self.note_unported(h),
        }
    }
}
