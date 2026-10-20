use crate::map::MapNodeRecords;
use crate::map_sim::MapSim;

fn load(screen: u8) -> MapSim {
    let mut map = MapSim::default();
    map.load_screen(screen, 0, &MapNodeRecords::default());
    map
}

// Expected byte sequences transcribed from bank $0F's loader lists, with the
// loader's $3DB00 path markers and Mario-specific writes applied.
#[test]
fn zone_sprite_loaders_match_rom_initial_bytes() {
    let expected: &[&[u8]] = &[
        &[
            84, 53, 15, 120, 52, 18, 24, 16, 22, 86, 101, 255, 106, 79, 255, 58, 79, 255, 87, 67,
            255,
        ],
        &[
            68, 17, 0, 128, 88, 4, 28, 50, 6, 87, 109, 255, 52, 122, 255, 92, 122, 255, 115, 78,
            255, 48, 78, 255, 78, 46, 255, 128, 132, 255,
        ],
        &[
            23, 73, 255, 0, 0, 255, 0, 0, 255, 0, 0, 255, 23, 129, 255, 39, 113, 255, 91, 113, 255,
            99, 73, 255,
        ],
        &[
            36, 40, 73, 32, 71, 67, 32, 32, 67, 64, 129, 67, 79, 93, 255, 63, 45, 255, 224, 224,
            70, 248, 144, 255, 248, 144, 255, 86, 86, 255, 151, 45, 255,
        ],
        &[
            88, 72, 255, 24, 120, 42, 112, 112, 43, 144, 96, 44, 79, 45, 255, 119, 101, 255, 95,
            125, 255, 128, 64, 46, 48, 80, 46, 160, 64, 50, 148, 16, 255,
        ],
        &[
            70, 36, 28, 90, 36, 29, 76, 16, 35, 74, 44, 36, 94, 44, 36, 74, 119, 37, 114, 43, 30,
            66, 123, 255, 84, 101, 255, 84, 82, 255, 84, 22, 255,
        ],
        &[25, 32, 52, 80, 33, 53, 52, 28, 255, 112, 98, 57],
        &[
            8, 16, 66, 128, 20, 66, 128, 136, 95, 141, 144, 96, 149, 144, 96, 157, 144, 96,
        ],
    ];
    for (i, expected) in expected.iter().enumerate() {
        let map = load(i as u8 + 1);
        let mut bytes: Vec<_> = expected.to_vec();
        bytes.resize(48, 0xFF);
        let actual: Vec<_> = map
            .zone_slots
            .iter()
            .flat_map(|&(x, y, id)| [x, y, id])
            .collect();
        assert_eq!(actual, bytes, "screen {}", i + 1);
    }
}

#[test]
fn zone_sprite_loaders_use_saved_coin_and_path_high_bits() {
    for (screen, first_slot, first_progress, count) in [
        (1, 3, 6, 4),
        (2, 4, 1, 5),
        (3, 4, 11, 4),
        (4, 4, 16, 2),
        (5, 4, 21, 3),
        (6, 7, 26, 4),
    ] {
        for flag in [1, 0x80] {
            let mut map = MapSim::default();
            map.level_progress.fill(flag);
            map.load_screen(screen, 0, &MapNodeRecords::default());
            for slot in first_slot..first_slot + count {
                assert_eq!(
                    map.zone_slots[slot].2,
                    if flag == 0x80 { 14 } else { 255 },
                    "screen {screen}, progress start {first_progress}, slot {slot}"
                );
            }
            match screen {
                1 => assert_eq!(map.zone_slots[2].2, if flag == 0x80 { 255 } else { 22 }),
                2 => assert_eq!(map.zone_slots[0].2, if flag == 0x80 { 255 } else { 0 }),
                3 => assert_eq!(map.zone_slots[0].2, if flag == 0x80 { 14 } else { 255 }),
                4 => assert_eq!(map.zone_slots[10].2, if flag == 0x80 { 14 } else { 255 }),
                6 => assert_eq!(
                    map.zone_slots[2],
                    if flag == 0x80 {
                        (92, 19, 35)
                    } else {
                        (76, 16, 35)
                    }
                ),
                _ => {}
            }
        }
    }
}

#[test]
fn zone_sprite_turtle_new_secret_and_saved_secret_use_distinct_ids() {
    for (saved, exit, dead, expected) in [
        (0, 0, 0, 255),
        (0, 1, 0, 125),
        (1, 1, 0, 129),
        (1, 0, 0, 129),
        (0, 1, 255, 255),
    ] {
        let mut map = MapSim::default();
        map.level_progress[22] = saved;
        map.mario_dead = dead;
        map.load_screen(5, exit, &MapNodeRecords::default());
        assert_eq!(map.zone_slots[10].2, expected);
    }
}

#[test]
fn zone_sprite_paths_use_individual_saved_flags_and_clear_previous_screen() {
    let mut map = MapSim::default();
    map.level_progress[2] = 0x80;
    map.load_screen(2, 0, &MapNodeRecords::default());
    assert_eq!(map.zone_slots[4].2, 255);
    assert_eq!(map.zone_slots[5].2, 14);
    assert_eq!(map.zone_slots[6].2, 255);
    map.load_screen(7, 0, &MapNodeRecords::default());
    assert_eq!(map.zone_slots[5], (255, 255, 255));
    assert_eq!(map.zone_slots[0], (25, 32, 52));
}
