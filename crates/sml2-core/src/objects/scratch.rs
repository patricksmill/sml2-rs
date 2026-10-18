//! `$EC40` (reached through home `$3A28`): the object scratch page clear.
//!
//! The ROM zeroes all 256 bytes of `$AF00-$AFFF` with a zero-B counter
//! (`ld hl,sObjectSpawnXHi` / `ld b,0` / `xor a` / `ldi [hl],a` / `dec b` /
//! `jr nz`) — including the stream pointer `$AF1E/$AF1F`. This zeroes every
//! **represented** cell of that page and deliberately leaves everything the
//! port keeps outside it alone: the stream buffer `$AB00-$ACFF` (the records
//! and their spawn markers), the slot page `$AD00-$AEFF` (cleared by `$EC31`,
//! a separate routine), the player/timer/fade/audio bytes in `$A2xx` /
//! `$A468` / `$A53F`, the sampled hardware `$FF04`, and HRAM `$FFF0`.
//!
//! Both ROM callers run it: the boss entry (`$8D9C` -> `$3A28`, before the
//! slots are cleared and the fight is staged) and the level-(re)load tail
//! (`$EBBB`, which then re-seeds the stream through `$EC17`).

use super::ObjectSim;

impl ObjectSim {
    /// `$EC40`: zero every represented cell of the object scratch page.
    ///
    /// The physical stream cursor goes to `$0000`, which is *not* a stream:
    /// only [`ObjectSim::seed_stream_cursor`] (the `$EC17`/`$EBBB` re-seed and
    /// the Wario phase-one exit's explicit `$AB06` write) makes it readable
    /// again.
    pub(super) fn clear_object_scratch(&mut self) {
        self.spawn_x = 0; // $AF00/$AF01
        self.sprite_map_index = 0; // $AF06
        self.update_phase = 0; // $AF08
        self.phase = false;
        self.clear_stage_flags = false; // $AF09
        self.clear_stream_cursor(); // $AF1E/$AF1F
        self.scan_dir = 0; // $AF22
        self.prev_cam_x = 0; // $AF24/$AF25
        self.remove_mode = 0; // $AF26
        self.slot_ptr_hi = 0; // $AF28
        self.boss_slot_ptr_lo = 0; // $AF29
        self.window_hit = false; // $AF2A
        self.sprite_flip_flags = 0; // $AF2B
        self.facing = 0; // $AF2C
        self.object_hit_flag = 0; // $AF2E
        self.move_flags = 0; // $AF2F
        self.mario_ref_x = 0; // $AF30-$AF33
        self.mario_ref_y = 0;
        self.spawn_anim = 0; // $AF36-$AF38
        self.spawn_sprite = 0;
        self.spawn_attr = 0;
        self.action_latch = 0; // $AF3A
        self.done = 0; // $AF3E
        self.standby = 0; // $AF40
        self.active = 0; // $AF41
        self.boss_def_index = 0; // $AFC0
        self.boss_def_count = 0; // $AFC1
        self.boss_hit_latch = 0; // $AFC3
        self.saved_frame_state = 0; // $AFC4
        self.boss_hits_remaining = 0; // $AFC5
        self.boss_alt_phase = 0; // $AFC6
        self.door_phase = 0; // $AFC7
        self.door_sequence_active = 0; // $AFC8
        self.door_parity = 0; // $AFC9
        self.scene_slot_parity = 0; // $AFCB
        self.boss_sprite = 0; // $AFCA
        self.saved_type = 0; // $AFCC
        self.boss_timer_reload = 0; // $AFCD
        self.boss_transition = 0; // $AFCE
        self.ending_door_active = 0; // $AFCF
        self.boss_delay = 0; // $AFD0
        self.saved_power_state = 0; // $AFD1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{ObjectTables, SpawnRecord, B_DEF, SLOT_COUNT};

    #[test]
    fn tree_boss_alternate_phase_clears_on_arena_entry_and_level_reset() {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        assert_eq!(sim.boss_alt_phase, 0);
        sim.boss_alt_phase = 0xA5;
        sim.spawn_boss(0);
        assert_eq!(sim.boss_alt_phase, 0, "$8D9C clears $AFC6");
        sim.boss_alt_phase = 1;
        sim.reset_level_start();
        assert_eq!(sim.boss_alt_phase, 0, "$049C clears $AFC6");
    }

    fn tables() -> ObjectTables {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        )))
        .unwrap()
    }

    fn record() -> SpawnRecord {
        SpawnRecord {
            x: 0x0128,
            y: 0x0090,
            anim_index: 3,
            mode_flag: 0,
            sprite_id: 0,
            hitbox: [0; 4],
            raw: [1, 0x28, 0, 0, 0x90, 3],
        }
    }

    /// Dirty every cell the page clear owns, plus the outside state it must
    /// keep, so one call has to get both sides right.
    fn dirty(sim: &mut ObjectSim) {
        sim.spawn_x = 0x1234;
        sim.sprite_map_index = 0x55;
        sim.update_phase = 9;
        sim.phase = true;
        sim.clear_stage_flags = true;
        sim.stream_index = 12;
        sim.scan_dir = 1;
        sim.prev_cam_x = 0x4321;
        sim.remove_mode = 2;
        sim.slot_ptr_hi = 0xAE;
        sim.boss_slot_ptr_lo = 0x40;
        sim.window_hit = true;
        sim.sprite_flip_flags = 0x60;
        sim.facing = 2;
        sim.object_hit_flag = 1;
        sim.move_flags = 0x88;
        sim.mario_ref_x = 0x0100;
        sim.mario_ref_y = 0x0200;
        sim.spawn_anim = 0x0D;
        sim.spawn_sprite = 9;
        sim.spawn_attr = 0x20;
        sim.action_latch = 1;
        sim.done = 3;
        sim.standby = 1;
        sim.active = 2;
        sim.boss_def_index = 6;
        sim.boss_def_count = 1;
        sim.boss_hit_latch = 1;
        sim.saved_frame_state = 8;
        sim.boss_hits_remaining = 3;
        sim.door_phase = 3;
        sim.door_sequence_active = 1;
        sim.door_parity = 1;
        sim.scene_slot_parity = 0xA5;
        sim.boss_sprite = 0x4F;
        sim.saved_type = 0x41;
        sim.boss_timer_reload = 80;
        sim.boss_transition = 1;
        sim.ending_door_active = 1;
        sim.boss_delay = 1;
        sim.saved_power_state = 0xFE;
    }

    #[test]
    fn boss_entry_scratch_clear_zeroes_every_represented_cell() {
        let mut sim = ObjectSim::new(vec![record(), record()], 0x0300);
        dirty(&mut sim);
        sim.clear_object_scratch();
        assert_eq!(
            (
                sim.spawn_x,
                sim.sprite_map_index,
                sim.update_phase,
                sim.phase,
                sim.clear_stage_flags,
                sim.scan_dir,
                sim.prev_cam_x,
                sim.remove_mode,
            ),
            (0, 0, 0, false, false, 0, 0, 0)
        );
        assert_eq!(
            (
                sim.slot_ptr_hi,
                sim.boss_slot_ptr_lo,
                sim.window_hit,
                sim.sprite_flip_flags,
                sim.facing,
                sim.object_hit_flag,
                sim.move_flags,
                sim.mario_ref_x,
                sim.mario_ref_y,
            ),
            (0, 0, false, 0, 0, 0, 0, 0, 0)
        );
        assert_eq!(
            (
                sim.spawn_anim,
                sim.spawn_sprite,
                sim.spawn_attr,
                sim.action_latch,
                sim.done,
                sim.standby,
                sim.active,
            ),
            (0, 0, 0, 0, 0, 0, 0)
        );
        assert_eq!(
            (
                sim.boss_def_index,
                sim.boss_def_count,
                sim.boss_hit_latch,
                sim.saved_frame_state,
                sim.boss_hits_remaining,
                sim.boss_sprite,
                sim.saved_type,
                sim.boss_timer_reload,
                sim.boss_transition,
                sim.ending_door_active,
                sim.boss_delay,
            ),
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
        );
        assert_eq!(sim.saved_power_state, 0);
        assert_eq!(sim.scene_slot_parity, 0);
        assert_eq!((sim.door_phase, sim.door_sequence_active, sim.door_parity), (0, 0, 0));
        // $AF1E/$AF1F are zero, so the physical pointer is $0000 and the
        // public index is inactive — not the $AB06 of a live cursor.
        assert_eq!(sim.stream_index, 0);
        assert_eq!(sim.stream_ptr().to_be_bytes(), [0, 0]);
        // The stream buffer and its markers are outside the page.
        assert_eq!(sim.records, vec![record(), record()]);
        assert_eq!(sim.spawned, vec![false, false]);
    }

    #[test]
    fn door_boss_reentry_and_level_reload_clear_sequence_scratch() {
        let mut sim = ObjectSim::new(vec![record()], 80);
        sim.configure(9);
        for reload in [false, true] {
            sim.door_phase = 3;
            sim.door_sequence_active = 1;
            sim.door_parity = 1;
            sim.fff0 = 0xA5;
            sim.div = 0x5A;
            if reload { sim.reset_level_start(); } else { sim.spawn_boss(1); }
            assert_eq!((sim.door_phase, sim.door_sequence_active, sim.door_parity), (0, 0, 0));
            assert_eq!((sim.fff0, sim.div), (0xA5, 0x5A));
            if !reload {
                assert_eq!(sim.slots[0].b[B_DEF], 6);
                assert_eq!(sim.boss_def_count, 3);
            }
        }
    }

    #[test]
    fn boss_entry_scratch_clear_preserves_state_outside_the_page() {
        let mut sim = ObjectSim::new(vec![record()], 0x0300);
        sim.spawned = vec![true];
        sim.fff0 = 7;
        sim.easy_mode = true;
        sim.level_id = 0x0A;
        sim.fade_timer = 0xABCD;
        sim.fade_timer_set = Some(0xAB05);
        sim.timer_low = 0x21;
        sim.div = 0x80;
        sim.pending_item = 0x42;
        sim.goal_bell = 1;
        sim.stomp_chain = 3;
        sim.money_bag = 2;
        sim.secret_route = true;
        sim.bounce = true;
        sim.boss_defeated = true;
        sim.boss_5290 = 1;
        sim.boss_clear_flag = 1;
        sim.mario_dead = 1;
        sim.music_request = Some(3);
        sim.sfx0 = 0x0F;
        sim.sfx1 = 4;
        sim.sfx2 = 2;
        sim.camera_pan_set = Some(0x34);
        sim.scene_phase_set = Some(1);
        sim.note_unported(0x8C31);
        sim.slots[0].b = [0xA5; 32];

        sim.clear_object_scratch();

        assert_eq!(
            (
                sim.fff0,
                sim.easy_mode,
                sim.level_id,
                sim.fade_timer,
                sim.fade_timer_set,
                sim.timer_low,
                sim.div,
            ),
            (7, true, 0x0A, 0xABCD, Some(0xAB05), 0x21, 0x80)
        );
        assert_eq!(
            (
                sim.pending_item,
                sim.goal_bell,
                sim.stomp_chain,
                sim.money_bag,
                sim.secret_route,
                sim.bounce,
                sim.boss_defeated,
                sim.boss_5290,
                sim.boss_clear_flag,
                sim.mario_dead,
            ),
            (0x42, 1, 3, 2, true, true, true, 1, 1, 1)
        );
        assert_eq!(
            (sim.music_request, sim.sfx0, sim.sfx1, sim.sfx2),
            (Some(3), 0x0F, 4, 2)
        );
        assert_eq!(
            (sim.camera_pan_set, sim.scene_phase_set),
            (Some(0x34), Some(1))
        );
        assert_eq!(sim.unported, vec![0x8C31]);
        assert_eq!(sim.spawned, vec![true]);
        assert_eq!(sim.records, vec![record()]);
        assert_eq!(sim.slots[0].b, [0xA5; 32]);
    }

    /// The real reentry: a dirty, played level-0 state enters the fight
    /// through `spawn_boss` (the `$8D9C` order: scratch clear, slot clear,
    /// staging) and the staged fight runs a frame from the cleared cursor.
    #[test]
    fn boss_entry_spawn_boss_clears_the_page_then_stages_the_fight() {
        let tables = tables();
        let layout = crate::level::LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
        let mut sim = ObjectSim::new(vec![record(), record()], 0x0300);
        dirty(&mut sim);
        sim.spawned = vec![true, true];
        sim.pending_item = 0x42;
        sim.fade_timer = 0x1234;
        sim.slot_done = [true; SLOT_COUNT];
        sim.slots[0].b = [0xA5; 32];

        sim.spawn_boss(6);

        // $EC40 then $8DFD/$8E6C: the scratch is zero and the fight is staged.
        assert_eq!(sim.boss_def_index, 6);
        assert_eq!(sim.slot_done, [false; SLOT_COUNT]);
        assert_eq!((sim.slots[0].b[B_DEF], sim.slots[1].b[B_DEF]), (20, 21));
        assert_eq!(sim.stream_ptr().to_be_bytes(), [0, 0]);
        assert_eq!(sim.prev_cam_x, 0);
        assert_eq!(sim.done, 0);
        assert_eq!(sim.saved_power_state, 0);
        // Outside the page nothing moved.
        assert_eq!((sim.fff0, sim.fade_timer, sim.pending_item), (0, 0x1234, 0x42));
        assert_eq!(sim.spawned, vec![true, true]);
        assert_eq!(sim.records, vec![record(), record()]);

        // The fight's driver runs from the cleared cursor without reports.
        sim.boss_engine(0x0050, 0x0060, &layout, &tables);
        assert!(sim.unported.is_empty(), "no unported path in the fight");
        assert_eq!(sim.slots[1].b[B_DEF], 21);
        assert!(!sim.boss_oam.is_empty(), "the staged fight renders");
    }

    /// The level-(re)load tail runs the same `$EC40` first, then `$EC17`
    /// re-seeds the stream — so a reload ends on the *live* `$AB06` cursor
    /// where a boss entry ends on the cleared one.
    #[test]
    fn the_reload_path_runs_the_same_page_clear_then_reseeds_the_stream() {
        let mut sim = ObjectSim::new(vec![record(), record()], 0x0300);
        dirty(&mut sim);
        sim.slot_done = [true; SLOT_COUNT];
        sim.slots[0].b = [0xA5; 32];

        sim.reset_level_start();

        assert_eq!(
            (sim.boss_def_index, sim.boss_def_count, sim.boss_hit_latch),
            (0, 0, 0)
        );
        assert_eq!(
            (sim.boss_transition, sim.ending_door_active, sim.boss_delay),
            (0, 0, 0)
        );
        assert_eq!(sim.saved_power_state, 0);
        assert_eq!(sim.stream_ptr(), 0xAB06, "$EC17 re-seeds after the clear");
        assert_eq!(sim.slot_done, [false; SLOT_COUNT]);
        assert_eq!(sim.slots[0].b, [0; 32]);
        assert_eq!(sim.records, vec![record(), record()]);
        assert_eq!(sim.spawned, vec![false, false]);
    }
}
