//! Wario phase exit, ROM `$A96D..$A9BC` (bank 2 CPU `$696D`).

use super::{ObjectSim, ObjectTables, SpawnRecord, Staging, B_SCRX};

impl ObjectSim {
    pub(super) fn wario_phase1_exit(&mut self, st: &mut Staging, tables: &ObjectTables) {
        // $A96F writes only the low byte; expose the write to subsequent slots
        // immediately as well as to the owner's deferred fade write consumer.
        self.fade_timer = (self.fade_timer & 0xFF00) | 5;
        self.fade_timer_set = Some(self.fade_timer);
        self.boss_flash_b40e(st);
        self.boss_sprite_cycle_b339(st);
        self.scr_x_add(st, 2);
        if st.b[B_SCRX] < 0xC0 {
            return;
        }
        self.boss_finish_blank(st);
        self.scene_phase_set = Some(1);

        // $A9B9 forces sprite mode B, independent of this level's initial mode.
        // The level-based table API uses $0A as a representative mode-B level.
        // This metadata is informational: scan derives sprite/hitbox itself.
        // Absent metadata never changes the six literal ROM stream bytes.
        let sprite_id = tables.sprite_for(0x0A, 0x1D).unwrap_or(0);
        let hitbox = tables.hitbox_for(0x0A, sprite_id).unwrap_or([0; 4]);
        // records abstracts the six $FF left sentinels and the right $FF marker;
        // index zero is the ROM's stream pointer $AB06, not the left sentinel.
        self.records = vec![SpawnRecord {
            x: 0x07C8,
            y: 0x00B0,
            anim_index: 0x1D,
            mode_flag: 0,
            sprite_id,
            hitbox,
            raw: [0x07, 0xC8, 0x00, 0x00, 0xB0, 0x1D],
        }];
        self.seed_stream_cursor();
        self.spawned = vec![false];
        self.sprite_map_index = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{B_ANIMFRAME, B_FLASH, B_SPRITE};

    fn tables() -> ObjectTables {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        )))
        .unwrap()
    }

    #[test]
    fn helper_0x0a96d_before_boundary_preserves_slot_and_stream() {
        let tables = tables();
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fade_timer = 0xABCD;
        sim.stream_index = 6;
        sim.spawned = vec![true, true];
        sim.fff0 = 1;
        let mut st = Staging::default();
        st.b = [0xA5; 32];
        st.b[B_ANIMFRAME] = 0;
        st.b[B_SCRX] = 0xBD;
        let mut expected = st.b;
        expected[B_SCRX] = 0xBF;
        sim.wario_phase1_exit(&mut st, &tables);
        assert_eq!(st.b, expected);
        assert_eq!((sim.fade_timer, sim.fade_timer_set), (0xAB05, Some(0xAB05)));
        assert_eq!(sim.scene_phase_set, None);
        assert_eq!(
            (sim.done, sim.sprite_map_index, sim.stream_index),
            (0, 0, 6)
        );
        assert!(sim.records.is_empty());
        assert_eq!(sim.spawned, vec![true, true]);
    }

    #[test]
    fn helper_0x0a96d_flash_cycle_gate_and_wrapped_x_do_not_exit() {
        let tables = tables();
        let mut sim = ObjectSim::new(vec![], 0);
        for (frame, flash, cycle, sprite) in [(0xFC, 0xB5, 1, 0xFF), (3, 0xA5, 0, 0xFE)] {
            sim.fff0 = frame;
            sim.fade_timer = 0x1200;
            let mut st = Staging::default();
            st.b = [0xA5; 32];
            st.b[B_ANIMFRAME] = 0;
            st.b[B_SCRX] = 0xFE;
            st.b[B_SPRITE] = 0xFE;
            let mut expected = st.b;
            expected[B_SCRX] = 0;
            expected[B_FLASH] = flash;
            expected[B_ANIMFRAME] = cycle;
            expected[B_SPRITE] = sprite;
            sim.wario_phase1_exit(&mut st, &tables);
            assert_eq!(st.b, expected);
            assert_eq!((sim.fade_timer, sim.fade_timer_set), (0x1205, Some(0x1205)));
            assert_eq!(sim.scene_phase_set, None);
            assert_eq!(sim.done, 0);
        }
    }

    #[test]
    fn helper_0x0a96d_boundary_blanks_and_installs_exact_new_stream() {
        let tables = tables();
        let old = SpawnRecord {
            x: 1,
            y: 2,
            anim_index: 3,
            mode_flag: 4,
            sprite_id: 5,
            hitbox: [6; 4],
            raw: [7; 6],
        };
        let mut sim = ObjectSim::new(vec![old, old], 0);
        sim.stream_index = 6;
        sim.spawned = vec![true, true];
        sim.fade_timer = 0x3401;
        sim.level_id = 0;
        let mut st = Staging::default();
        st.b = [0xA5; 32];
        st.b[B_ANIMFRAME] = 3;
        st.b[B_SCRX] = 0xBE;
        sim.wario_phase1_exit(&mut st, &tables);
        assert_eq!(st.b, [0; 32]);
        assert_eq!((sim.fade_timer, sim.fade_timer_set), (0x3405, Some(0x3405)));
        assert_eq!(sim.scene_phase_set, Some(1));
        assert_eq!(
            (
                sim.done,
                sim.sprite_map_index,
                sim.stream_index,
                sim.level_id
            ),
            (1, 1, 0, 0)
        );
        assert_eq!(sim.spawned, vec![false]);
        assert_eq!(sim.records.len(), 1);
        let record = sim.records[0];
        assert_eq!(
            (record.x, record.y, record.anim_index, record.mode_flag),
            (0x07C8, 0x00B0, 0x1D, 0)
        );
        assert_eq!(record.raw, [0x07, 0xC8, 0, 0, 0xB0, 0x1D]);
        let sprite = tables.sprite_for(0x0A, 0x1D).unwrap();
        assert_eq!(record.sprite_id, sprite);
        assert_eq!(record.hitbox, tables.hitbox_for(0x0A, sprite).unwrap());
    }

    #[test]
    fn helper_0x0a96d_level_reload_restores_original_expanded_stream() {
        let tables = tables();
        let first = SpawnRecord {
            x: 0x0128,
            y: 0x0090,
            anim_index: 0x03,
            mode_flag: 1,
            sprite_id: 0x40,
            hitbox: [-1, 2, -3, 4],
            raw: [0x01, 0x28, 1, 0, 0x90, 3],
        };
        let second = SpawnRecord {
            x: 0x0230,
            raw: [0x02, 0x30, 1, 0, 0x90, 3],
            ..first
        };
        let mut sim = ObjectSim::new(vec![first, second], 0);
        sim.spawned = vec![true, true];
        sim.stream_index = 6;
        let mut st = Staging::default();
        st.b[B_SCRX] = 0xBE;
        sim.wario_phase1_exit(&mut st, &tables);
        assert_eq!(sim.records.len(), 1);
        assert_eq!(sim.records[0].raw, [7, 0xC8, 0, 0, 0xB0, 0x1D]);
        assert_eq!(sim.scene_phase_set, Some(1));
        sim.spawned[0] = true;
        sim.reset_level_start();
        assert_eq!(sim.records, vec![first, second]);
        assert_eq!(sim.spawned, vec![false, false]);
        assert_eq!(sim.stream_index, 0);
        assert_eq!(sim.scene_phase_set, None);
    }

    #[test]
    fn helper_0x0a96d_event_releases_camera_and_stream_scans_level_object() {
        use crate::objects::{B_DEF, B_MARKER, B_STATE, B_XHI, B_XLO, B_YHI, B_YLO};
        let tables = tables();
        let mut sim = ObjectSim::new(vec![], 0);
        sim.level_id = 0x18;
        let mut cam = crate::camera::CameraSim::new(0x0780, 0);
        cam.scene_state = 0xF0;
        cam.boss_fight_phase = 0;
        assert!(cam.boss_mode());
        let mut st = Staging::default();
        st.b[B_SCRX] = 0xBE;
        sim.wario_phase1_exit(&mut st, &tables);
        cam.apply_object_result(&mut sim);
        assert_eq!((cam.scene_state, cam.boss_fight_phase), (0, 1));
        assert!(!cam.boss_mode());
        assert_eq!(sim.scene_phase_set, None);
        sim.scan_dir = 1;
        sim.spawn_x = 0x07C8;
        sim.scan(&tables);
        let bytes = &sim.slots[0].b;
        assert_eq!(
            [
                bytes[B_XHI],
                bytes[B_XLO],
                bytes[B_MARKER],
                bytes[B_YHI],
                bytes[B_YLO],
                bytes[B_DEF]
            ],
            [7, 0xC8, 0, 0, 0xB0, 0x1D]
        );
        assert_eq!(bytes[B_STATE], 1);
        assert_eq!(sim.spawned, vec![true]);
        assert!(sim.unported.is_empty());
    }
}
