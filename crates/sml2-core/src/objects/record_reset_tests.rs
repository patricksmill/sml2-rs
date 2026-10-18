//! Literal source contracts for spawn reset and its compatibility projection.
use super::*;
use super::record_origin::RecordResetError;
use crate::actor::{Position, SpawnOrigin};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json"))).unwrap()
}
fn record() -> SpawnRecord {
    SpawnRecord { x: 0x1234, y: 0x5678, anim_index: 53, mode_flag: 2, sprite_id: 0, hitbox: [0;4], raw: [0xEE;6] }
}
fn staging(pointer: u16) -> Staging {
    let mut b = std::array::from_fn(|i| 0x80 + i as u8);
    b[5] = 53;
    b[6..8].copy_from_slice(&pointer.to_be_bytes());
    Staging { b, ..Default::default() }
}
fn origin(bytes: [u8; 5]) -> SpawnOrigin {
    SpawnOrigin { position: Position { x: u16::from_be_bytes([bytes[0],bytes[1]]), y: u16::from_be_bytes([bytes[3],bytes[4]]) }, marker: bytes[2] }
}

#[test]
fn record_reset_full_slot_live_map_and_counters() {
    let mut tables = tables();
    tables.anim_to_sprite_a[53] = 0x42;
    tables.anim_to_sprite_b[53] = 0x63;
    for (level, map) in [(10,0),(0,1),(0,2),(0,255)] {
        for (active, standby) in [(0,255),(255,0),(7,9)] {
            let mut sim = ObjectSim::new(vec![record()],0);
            sim.level_id = level;
            sim.sprite_map_index = map;
            sim.active = active;
            sim.standby = standby;
            sim.spawn_attr = 0xE7;
            let mut st = staging(0xAB08);
            let mut expected = st.b;
            expected[..5].copy_from_slice(&[0x12,0x34,2,0x56,0x78]);
            expected[8] = 1;
            expected[11] = if map == 0 {0x42} else {0x63};
            expected[14..31].fill(0);
            sim.respawn_object_5b2bf(&mut st, &tables);
            assert_eq!(st.b, expected);
            assert_eq!((sim.done,sim.spawn_attr), (1,0));
            assert_eq!((sim.active,sim.standby), (active,standby));
            assert_eq!(sim.records, [record()]);
        }
    }
}

#[test]
fn record_reset_projection_prefix_sentinel_tail_live_fields_and_unaligned_windows() {
    let mut sim = ObjectSim::new(vec![record()],0);
    sim.records[0].anim_index = 0x99;
    sim.records[0].mode_flag = 0x10;
    assert_eq!(sim.resolve_spawn_origin(0xAB02), Ok(origin([255;5])));
    assert_eq!(sim.resolve_spawn_origin(0xAB08), Ok(origin([0x12,0x34,0x10,0x56,0x78])));
    assert_eq!(sim.resolve_spawn_origin(0xAB09), Ok(origin([0x34,0x10,0x56,0x78,0x99])));
    assert_eq!(sim.resolve_spawn_origin(0xAB0D), Ok(origin([0x99,255,255,0,0])));
    assert_eq!(sim.resolve_spawn_origin(0xACFD), Ok(origin([0;5])));
    // Last valid aligned record: $ACF8, pointer $ACFA; no low-byte carry.
    sim.records = vec![record();84];
    assert_eq!(sim.resolve_spawn_origin(0xACFA), Ok(origin([0x12,0x34,2,0x56,0x78])));
    assert_eq!(sim.resolve_spawn_origin(0xAD00), Ok(origin([255,255,0,0,0])));
    // Arbitrary unaligned pointer $AC00 copies the actual adjacent records.
    assert_eq!(sim.resolve_spawn_origin(0xAC00), Ok(origin([2,0x56,0x78,53,0x12])));
}

#[test]
fn record_reset_dynamic_copies_committed_slots_including_self_and_page_boundary() {
    let tables = tables();
    for (pointer, slot) in [(0xAD00,0),(0xAD20,1),(0xAE00,8)] {
        let mut sim = ObjectSim::new(vec![record();84],0);
        sim.slots[slot].b[..3].copy_from_slice(&[0x31,0x42,0x53]);
        if slot > 0 { sim.slots[slot-1].b[30..].copy_from_slice(&[0x61,0x72]); }
        let mut st = staging(pointer);
        sim.slot_ptr_hi = (pointer >> 8) as u8;
        sim.boss_slot_ptr_lo = pointer as u8;
        let mut expected = st.b;
        expected[..5].copy_from_slice(&[if slot==0 {255}else{0x61},if slot==0 {255}else{0x72},0x31,0x42,0x53]);
        expected[8] = 1;
        sim.spawn_attr = 0;
        sim.active = 0;
        sim.standby = 255;
        sim.respawn_object_5b2bf(&mut st,&tables);
        assert_eq!(st.b,expected);
        assert_eq!((sim.done,sim.spawn_attr),(1,0x8D));
        assert_eq!((sim.active,sim.standby),(0,255));
        assert_eq!(&sim.slots[slot].b[..3], &[0x31,0x42,0x53]);
    }
}

#[test]
fn record_reset_ac_ad_threshold_changes_visual_reset_only() {
    let tables = tables();
    for pointer in [0xACFF,0xAD00] {
        let mut sim = ObjectSim::new(vec![record()],0);
        let mut st = staging(pointer);
        let dirty=st.b;
        sim.reset_record_origin(&mut st,&tables).unwrap();
        assert_eq!(&st.b[..5], &[0;5]);
        if pointer == 0xACFF {
            assert_eq!(&st.b[14..31], &[0;17]);
            assert_eq!(sim.spawn_attr,0);
        } else {
            assert_eq!(&st.b[9..], &dirty[9..]);
            assert_eq!(sim.spawn_attr,dirty[13]);
        }
    }
}

#[test]
fn record_reset_errors_preserve_inputs_and_outputs() {
    let mut tables = tables();
    for (pointer, count) in [(0xAB00,1),(0,1),(1,1),(0xAF00,1),(0xAC00,85),(0xAB08,1)] {
        let mut sim = ObjectSim::new(vec![record();count],0);
        sim.done=7; sim.spawn_attr=9; sim.active=255; sim.standby=0;
        let mut st=staging(pointer);
        if pointer==0xAB08 { tables.anim_to_sprite_a.clear(); }
        let before=st.b;
        let slots=sim.slots.each_ref().map(|s|s.b);
        let records=sim.records.clone();
        let spawned=sim.spawned.clone();
        assert!(sim.reset_record_origin(&mut st,&tables).is_err());
        assert_eq!(st.b,before);
        assert_eq!(sim.slots.each_ref().map(|s|s.b),slots);
        assert_eq!(sim.records,records);
        assert_eq!(sim.spawned,spawned);
        assert_eq!((sim.done,sim.spawn_attr,sim.active,sim.standby),(7,9,255,0));
    }
    let sim=ObjectSim::new(vec![record();85],0);
    assert_eq!(sim.resolve_spawn_origin(0xAC00),Err(RecordResetError::OversizedStream {records:85}));
}

#[test]
fn record_reset_live_allocation_and_removal_marker_lifecycle() {
    let tables=tables();
    for spent in [false,true] {
        let mut rec=record();rec.x=0x128;rec.y=0x90;rec.mode_flag=1;
        let mut sim=ObjectSim::new(vec![rec],0);
        sim.scan_dir=1;sim.spawn_x=0x128;sim.scan(&tables);
        assert_eq!(sim.records[0].mode_flag,2);
        let mut st=staging(0xAB08);
        sim.respawn_object_5b2bf(&mut st,&tables);
        assert_eq!(st.b[2],2);
        sim.slots[0].b[2]=if spent {16}else{1};
        sim.remove_slot(0,&tables);
        sim.respawn_object_5b2bf(&mut st,&tables);
        assert_eq!(st.b[2],if spent {16}else{0});
        assert_eq!(sim.records[0].raw,[0xEE;6]);
    }
}

#[test]
fn record_reset_real_charging_step_then_separate_promotion_counter_owner() {
    let tables=tables();
    let layout=LevelLayout::from_bytes(&[0x60;0x3000]).unwrap();
    let mut rec=record();rec.x=0x128;rec.y=0x90;rec.anim_index=83;
    let mut sim=ObjectSim::new(vec![rec],0x128);
    sim.configure(10);
    let mut st=staging(0xAB08);
    st.b[0..5].copy_from_slice(&[1,0x38,2,0,0x90]);
    st.b[5]=83;st.b[8]=2;st.b[15]=4;st.b[10]=0xB8;
    st.b[18]=0;st.b[23]=0;st.b[27]=0;
    sim.slots[0].b=st.b;
    sim.active=1;sim.standby=0;
    sim.phase=true;sim.fff0=0; // next engine phase false, even, slot0 handler runs
    let mario=MarioContact {travelling:true,..Default::default()};
    sim.step(0x128,0x90,&layout,&tables,&mario,false);
    let bytes=sim.slots[0].b;
    assert_eq!(&bytes[..9], &[1,0x28,2,0,0x90,83,0xAB,8,1]);
    assert_eq!(bytes[11],tables.anim_to_sprite_b[83]);
    assert_eq!(&bytes[14..19], &[0;5]);
    assert_eq!(&bytes[23..31], &[0;8]);
    let hb=tables.hitbox_for(10,bytes[11]).unwrap();
    assert_eq!(&bytes[19..23],&hb.map(|b|b as u8));
    assert!(sim.slot_done[0]);
    assert_eq!(sim.spawn_attr,0);
    assert_eq!((sim.active,sim.standby),(1,0));
    // The source resets activity without accounting; later promotion owns its
    // own wrapping count update even though these values look unbalanced.
    sim.phase=true;sim.fff0=0;
    sim.step(0x128,0x90,&layout,&tables,&mario,false);
    assert_eq!(sim.slots[0].b[8],2);
    assert_eq!((sim.active,sim.standby),(2,255));
}

#[test]
fn record_reset_source_instructions_validate_constants_and_call_chain() {
    let root=std::env::var_os("SML2_DECOMP_ROOT").map(std::path::PathBuf::from).unwrap_or_else(||std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../sml2decomp"));
    let rom=std::fs::read(root.join("baserom.gb")).unwrap();
    assert_eq!(&rom[0x5B2BF..0x5B2D7],&[0xF0,0xD6,0x67,0xF0,0xD7,0x6F,0x2B,0x2B,0x11,0xD0,0xFF,0x06,5,0xCD,0xAA,0x3C,0xCD,0xA0,0x3A,0x3E,1,0xE0,0xD8,0xC9]);
    assert_eq!(&rom[0x863C..0x864A],&[0xF0,0xDD,0xEA,0x38,0xAF,0xCD,0x40,0x42,0x3E,1,0xEA,0x3E,0xAF,0xC9]);
    assert_eq!(&rom[0x8240..0x8245],&[0xF0,0xD6,0xFE,0xAD,0xD0]);
    assert_eq!(&rom[0x8261..0x826D],&[0x06,17,0xAF,0x22,0x05,0x20,0xFC,0xAF,0xEA,0x38,0xAF,0xC9]);
    assert_eq!(&rom[0x3CAA..0x3CB1],&[0x2A,0x12,0x1C,0x05,0x20,0xFA,0xC9]);
}
