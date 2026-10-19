//! Live stream marker ownership: $8CA8 allocation and $5B2D7 removal.
use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

fn record(marker: u8) -> SpawnRecord {
    SpawnRecord {
        x: 0x128,
        y: 0x90,
        anim_index: 3,
        mode_flag: marker,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [1, 0x28, marker, 0, 0x90, 3],
    }
}

fn scan(sim: &mut ObjectSim, tables: &ObjectTables) {
    sim.stream_index = 0;
    sim.scan_dir = 1;
    sim.spawn_x = sim.records[0].x;
    sim.scan(tables);
}

#[test]
fn record_marker_source_instructions_copy_then_store_two_and_remove_zero_or_spent() {
    let root = std::env::var_os("SML2_DECOMP_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let decomp = manifest.join("../../decomp");
            if decomp.join("baserom.gb").is_file() {
                decomp
            } else {
                manifest.join("../../../sml2decomp")
            }
        });
    let rom = std::fs::read(root.join("baserom.gb")).expect("source ROM for marker contract");
    // $8CA8 copies all six bytes before the later $8CD4 store of marker 2.
    assert_eq!(&rom[0x8CA8..0x8CAE], &[0x1A, 0x22, 0x13, 0x05, 0x20, 0xFA]);
    assert_eq!(
        &rom[0x8CCA..0x8CD6],
        &[0xFA, 0x1E, 0xAF, 0x67, 0xFA, 0x1F, 0xAF, 0xC6, 2, 0x6F, 0x36, 2]
    );
    // Mode != 0 and dynamic pointers skip marker writes; actor marker $10
    // survives, every other actor marker becomes zero at the record pointer.
    assert_eq!(
        &rom[0x5B2D7..0x5B2F0],
        &[
            0xFA, 0x26, 0xAF, 0xA7, 0x20, 0x13, 0x21, 0xD6, 0xFF, 0x2A, 0xFE, 0xAD, 0x30, 0x0B,
            0x57, 0x2A, 0x5F, 0xF0, 0xD2, 0xFE, 0x10, 0x28, 0x01, 0xAF, 0x12,
        ]
    );
}

#[test]
fn record_marker_real_scan_copies_preallocation_marker_then_keeps_live_two_through_activation() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    for marker in [0, 1, 0x10] {
        let original = record(marker);
        let mut sim = ObjectSim::new(vec![original], 0x128);
        scan(&mut sim, &tables);
        let b = sim.slots[0].b;
        assert_eq!(&b[..9], &[1, 0x28, marker, 0, 0x90, 3, 0xAB, 8, 1]);
        assert_eq!(sim.records[0].mode_flag, 2);
        assert_eq!(sim.records[0].raw, original.raw);
        assert_eq!(sim.spawned, [true]);
        assert_eq!((sim.standby, sim.active), (1, 0));
        // Force the normal engine's even-frame promotion branch. The actor
        // retains the preallocation marker while its source record remains 2.
        sim.phase = true;
        sim.fff0 = 0;
        sim.step(
            0x128,
            0x90,
            &layout,
            &tables,
            &MarioContact::default(),
            false,
        );
        assert_eq!(sim.slots[0].b[B_STATE], 2);
        assert_eq!(sim.slots[0].b[B_MARKER], marker);
        assert_eq!((sim.standby, sim.active), (0, 1));
        assert_eq!(sim.records[0].mode_flag, 2);
        assert_eq!(sim.slots.iter().filter(|s| s.b[B_STATE] != 0).count(), 1);
    }
}

#[test]
fn record_marker_remove_uses_actor_marker_and_mode_without_changing_count_policy() {
    let tables = tables();
    for dirty in [0, 1, 2, 0x10, 0xFF] {
        for marker in [0, 1, 2, 0x10, 0x11, 0xFF] {
            for mode in [0, 1, 2] {
                for state in [1, 2] {
                    for spawned in [false, true] {
                        let mut sim = ObjectSim::new(vec![record(1)], 0);
                        sim.records[0].mode_flag = dirty;
                        sim.spawned[0] = spawned;
                        sim.active = 3;
                        sim.standby = 4;
                        sim.remove_mode = mode;
                        let mut st = Staging::default();
                        st.b[B_MARKER] = marker;
                        st.b[B_RECHI] = 0xAB;
                        st.b[B_RECLO] = 8;
                        st.b[B_STATE] = state;
                        sim.remove_object(&mut st, &tables);
                        let expected = if mode == 0 {
                            if marker == 0x10 {
                                0x10
                            } else {
                                0
                            }
                        } else {
                            dirty
                        };
                        assert_eq!(sim.records[0].mode_flag, expected);
                        assert_eq!(sim.spawned[0], mode != 0 && spawned);
                        assert_eq!(sim.records[0].raw, record(1).raw);
                        assert_eq!(st.b[B_STATE], 0);
                        assert_eq!(st.b[B_MARKER], 0);
                        assert_eq!(
                            (sim.active, sim.standby),
                            if state == 1 { (3, 3) } else { (2, 4) }
                        );
                        assert_eq!(sim.money_bag, u8::from(mode == 1 && state == 2));
                        assert_eq!((sim.done, sim.remove_mode), (2, 0));
                    }
                }
            }
        }
    }
}

#[test]
fn record_marker_dynamic_removal_preserves_stream_state() {
    let tables = tables();
    for high in [0xAD, 0xAE, 0xFF] {
        let mut sim = ObjectSim::new(vec![record(1)], 0);
        sim.spawned[0] = true;
        let mut st = Staging::default();
        st.b[B_RECHI] = high;
        st.b[B_RECLO] = 8;
        st.b[B_MARKER] = 0x10;
        st.b[B_STATE] = 1;
        sim.standby = 1;
        sim.remove_object(&mut st, &tables);
        assert_eq!(sim.records[0], record(1));
        assert_eq!(sim.spawned, [true]);
        assert_eq!(sim.standby, 0);
        assert_eq!(st.b[B_STATE], 0);
    }
}

#[test]
fn record_marker_removed_easy_only_record_respawns_in_easy_and_spent_marker_survives() {
    let tables = tables();
    for spent in [false, true] {
        let mut sim = ObjectSim::new(vec![record(1)], 0);
        sim.easy_mode = true;
        scan(&mut sim, &tables);
        assert_eq!(sim.slots[0].b[B_STATE], 0); // original 1 is skipped in Easy
        sim.easy_mode = false;
        scan(&mut sim, &tables);
        assert_eq!(sim.slots[0].b[B_MARKER], 1);
        assert_eq!(sim.records[0].mode_flag, 2);
        if spent {
            sim.slots[0].b[B_MARKER] = 0x10;
        }
        sim.remove_slot(0, &tables);
        let expected = if spent { 0x10 } else { 0 };
        assert_eq!(sim.records[0].mode_flag, expected);
        assert!(!sim.spawned[0]);
        sim.easy_mode = true;
        scan(&mut sim, &tables);
        assert_eq!(sim.slots[0].b[B_MARKER], expected);
        assert_eq!(sim.slots[0].b[B_STATE], 1);
        assert_eq!(sim.records[0].mode_flag, 2);
        assert!(sim.spawned[0]);
    }
}

#[test]
fn record_marker_level_reset_restores_original_markers_after_wario_replacement() {
    let tables = tables();
    let originals = vec![record(0), record(1), record(0x10)];
    for phase in [1, 2] {
        let mut sim = ObjectSim::new(originals.clone(), 0);
        scan(&mut sim, &tables);
        assert_eq!(
            sim.records.iter().map(|r| r.mode_flag).collect::<Vec<_>>(),
            [2; 3]
        );
        let mut st = Staging::default();
        st.b[B_SCRX] = 0xBE;
        if phase == 1 {
            sim.wario_phase1_exit(&mut st, &tables);
        } else {
            sim.wario_phase2_exit(&mut st, &tables);
        }
        assert_eq!(sim.records.len(), 1);
        assert_eq!(sim.records[0].mode_flag, 0);
        assert_eq!(sim.spawned, [false]);
        scan(&mut sim, &tables);
        assert_eq!(sim.records[0].mode_flag, 2);
        sim.reset_level_start();
        assert_eq!(sim.records, originals);
        assert_eq!(sim.spawned, [false; 3]);
        assert!(sim.slots.iter().all(|s| s.b == [0; 32]));
    }
}
