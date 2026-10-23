//! Physical representation of the AF1E/AF1F stream cursor.

use super::{ObjectSim, STREAM_BASE};

impl ObjectSim {
    /// Physical stream address; the public index is inactive after scratch clear.
    pub fn stream_ptr(&self) -> u16 {
        if self.stream_cursor_cleared {
            0
        } else {
            STREAM_BASE.wrapping_add(self.stream_index as u16)
        }
    }

    // Used by the scratch-clear helper once its remaining AF fields are ported.
    #[allow(dead_code)]
    pub(super) fn clear_stream_cursor(&mut self) {
        self.stream_cursor_cleared = true;
        self.stream_index = 0;
    }

    pub(super) fn seed_stream_cursor(&mut self) {
        self.stream_cursor_cleared = false;
        self.stream_index = 0;
        self.margin_filled = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::{
        ObjectTables, SpawnRecord, Staging, B_DEF, B_MARKER, B_RECHI, B_RECLO, B_SCRX, B_STATE,
        B_XHI, B_XLO, B_YHI, B_YLO,
    };

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

    #[test]
    fn stream_cursor_clear_preserves_records_markers_and_slots() {
        let mut sim = ObjectSim::new(vec![record(), record()], 0);
        assert_eq!(sim.stream_ptr(), 0xAB06);
        sim.stream_index = 6;
        assert_eq!(sim.stream_ptr(), 0xAB0C);
        sim.spawned = vec![true, false];
        sim.slots[0].b = [0xA5; 32];
        sim.clear_stream_cursor();
        assert_eq!(sim.stream_ptr().to_be_bytes(), [0, 0]);
        assert_eq!(sim.stream_index, 0);
        assert_eq!(sim.records, vec![record(), record()]);
        assert_eq!(sim.spawned, vec![true, false]);
        assert_eq!(sim.slots[0].b, [0xA5; 32]);
        // Public index writes cannot reactivate a cleared physical cursor.
        sim.stream_index = 6;
        assert_eq!(sim.stream_ptr(), 0);
        assert!(!sim.advance_stream());
        assert_eq!(sim.stream_index, 6);
    }

    #[test]
    fn stream_cursor_cleared_scan_reports_unsupported_and_cannot_spawn() {
        let mut sim = ObjectSim::new(vec![record(), record()], 0);
        sim.scan_dir = 1;
        sim.spawn_x = 0x0128;
        sim.clear_stream_cursor();
        sim.scan(&tables());
        sim.scan(&tables());
        assert!(sim.slots.iter().all(|slot| slot.b == [0; 32]));
        assert_eq!(sim.spawned, vec![false, false]);
        assert_eq!(sim.records, vec![record(), record()]);
        assert_eq!(sim.unported, vec![0x8C31]);
        assert_eq!(sim.stream_ptr(), 0);
        assert!(!sim.advance_stream());
        assert_eq!(sim.stream_index, 0);
    }

    #[test]
    fn stream_cursor_wario_reseed_reactivates_real_scan() {
        let mut sim = ObjectSim::new(vec![record()], 0);
        sim.clear_stream_cursor();
        let mut st = Staging::default();
        st.b[B_SCRX] = 0xBE;
        let tables = tables();
        sim.wario_phase1_exit(&mut st, &tables);
        assert_eq!(sim.stream_ptr().to_be_bytes(), [0xAB, 6]);
        sim.scan_dir = 1;
        sim.spawn_x = 0x07C8;
        sim.scan(&tables);
        let b = sim.slots[0].b;
        assert_eq!(
            [
                b[B_XHI],
                b[B_XLO],
                b[B_MARKER],
                b[B_YHI],
                b[B_YLO],
                b[B_DEF]
            ],
            [7, 0xC8, 0, 0, 0xB0, 0x1D]
        );
        assert_eq!([b[B_RECHI], b[B_RECLO], b[B_STATE]], [0xAB, 8, 1]);
        assert_eq!(sim.spawned, vec![true]);
        assert!(sim.unported.is_empty());
    }

    #[test]
    fn stream_cursor_reload_restores_original_records_and_active_pointer() {
        let mut sim = ObjectSim::new(vec![record(), record()], 0);
        sim.records.clear();
        sim.spawned = vec![true];
        sim.clear_stream_cursor();
        sim.reset_level_start();
        assert_eq!(sim.stream_ptr(), 0xAB06);
        assert_eq!(sim.stream_index, 0);
        assert_eq!(sim.records, vec![record(), record()]);
        assert_eq!(sim.spawned, vec![false, false]);
        sim.scan_dir = 1;
        sim.spawn_x = 0x0128;
        sim.scan(&tables());
        assert_eq!(sim.slots[0].b[B_STATE], 1);
        assert_eq!(sim.spawned, vec![true, true]);
        assert!(sim.unported.is_empty());
    }

    #[test]
    fn backward_scan_spawns_every_record_in_the_edge_column() {
        // `$8CEB`: after a spawn (and on a marker skip) the backward scan
        // steps the pointer by `$FFFA`, so both records at the edge column
        // spawn in one pass and the cursor stops on the record to the left.
        let at = |x: u16, y: u16| SpawnRecord {
            x,
            y,
            raw: [(x >> 8) as u8, x as u8, 0, (y >> 8) as u8, y as u8, 3],
            ..record()
        };
        let records = vec![
            at(0x0120, 0x90),
            at(0x0128, 0x60),
            at(0x0128, 0x90),
            at(0x0130, 0x90),
        ];
        let mut sim = ObjectSim::new(records, 0);
        sim.stream_index = 18;
        sim.scan_dir = 2;
        sim.spawn_x = 0x0128;
        sim.scan(&tables());
        assert_eq!(sim.spawned, vec![false, true, true, false]);
        let ys: Vec<u8> = sim.slots[..2].iter().map(|s| s.b[B_YLO]).collect();
        assert_eq!(ys, [0x90, 0x60]);
        assert_eq!(sim.stream_index, 0);
    }

    #[test]
    fn stream_cursor_physical_address_wraps_at_sixteen_bits() {
        let mut sim = ObjectSim::new(vec![], 0);
        for (offset, pointer) in [
            (0x54F9, 0xFFFF),
            (0x54FA, 0),
            (0x54FB, 1),
            (0x10006, 0xAB0C),
        ] {
            sim.stream_index = offset;
            assert_eq!(sim.stream_ptr(), pointer);
        }
    }

    #[test]
    fn stream_cursor_spawn_record_pointer_keeps_source_low_byte_wrap() {
        let mut sim = ObjectSim::new(vec![record(); 85], 0);
        sim.stream_index = 504;
        assert_eq!(sim.stream_ptr(), 0xACFE);
        sim.scan_dir = 1;
        sim.spawn_x = 0x0128;
        sim.scan(&tables());
        let b = sim.slots[0].b;
        assert_eq!([b[B_RECHI], b[B_RECLO], b[B_STATE]], [0xAC, 0, 1]);
        assert!(sim.spawned[84]);
    }
}
