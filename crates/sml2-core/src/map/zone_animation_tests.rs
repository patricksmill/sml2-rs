use super::*;
use crate::map::MapNodeRecords;

#[test]
fn a_first_space_clear_starts_the_ufo_and_reaches_landing_and_shake() {
    let mut map = MapSim::new();
    map.load_screen(4, 1, &MapNodeRecords::default());
    assert_eq!(map.level_progress[16] & 1, 1);
    assert_eq!(map.zone_slots[7], (248, 144, 255));
    assert_eq!(map.zone_slots[9].2, 117);
    assert_eq!(map.zone_animation.patch_ids[0], 47);
    map.bookkeeping(0);
    assert_eq!(map.zone_slots[7], (251, 141, 111));
    for _ in 1..119 {
        map.bookkeeping(0);
    }
    assert_eq!(map.zone_animation.space.launch_state, 2);
    assert_eq!(map.zone_slots[7], (168, 14, 111));
    for _ in 0..10 {
        map.bookkeeping(0);
    }
    assert_eq!(map.zone_animation.space.launch_state, 3);
    assert_eq!(map.zone_slots[7], (156, 36, 112));
    assert_eq!(&map.zone_animation.patch_ids[..2], &[56, 52]);
    assert_eq!(map.zone_animation.sfx0, 24);
    map.bookkeeping(0); // Frame 130: shake bit 1 is set.
    assert_eq!(map.zone_background_scroll(0), (9, 246));
    for _ in 0..20 {
        map.bookkeeping(0);
    }
    assert_eq!(map.zone_background_scroll(0), (0, 0));
}

#[test]
fn source_route_patch_records_match_the_extracted_rom_tables() {
    let manifest = crate::map::OverworldManifest::from_json_str(include_str!(
        "../../../../extracted/map/route_patches.json"
    ))
    .unwrap();
    assert_eq!(animation_patches::PATCHES.len(), manifest.patches.len());
    for (source, extracted) in animation_patches::PATCHES.iter().zip(manifest.patches) {
        let bytes: Vec<_> = extracted
            .tiles
            .iter()
            .map(|t| (t.dx, t.dy, t.tile))
            .collect();
        assert_eq!(*source, bytes, "patch {}", extracted.id);
    }
}
