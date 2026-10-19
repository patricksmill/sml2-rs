use super::*;

fn layout(tile: u8) -> LevelLayout {
    LevelLayout::from_bytes(&vec![tile; 0x3000]).unwrap()
}

#[test]
fn terrain_probes_wall_threshold_order_reach_and_flags() {
    for reach in [7, 11] {
        for right in [false, true] {
            for (upper, lower, blocked, final_y) in [
                (55, 56, true, 13),
                (56, 55, true, 19),
                (56, 56, false, 19),
                (55, 55, true, 13),
            ] {
                let mut map = layout(96);
                let x = if right { 32 + reach } else { 32 - reach };
                map.cells[usize::from(x / 16)] = upper;
                map.cells[256 + usize::from(x / 16)] = lower;
                let mut probes = TerrainProbes::new(&map, Position { x: 999, y: 888 });
                assert_eq!(
                    probes.wall_blocked(Position { x: 32, y: 24 }, right, reach),
                    blocked
                );
                assert_eq!(probes.position(), Position { x, y: final_y });
                assert_eq!(
                    probes.legacy_move_flags(),
                    Some(match (right, blocked) {
                        (true, true) => 17,
                        (true, false) => 16,
                        (false, true) => 34,
                        (false, false) => 32,
                    })
                );
            }
        }
    }
}

#[test]
fn terrain_probes_wall_reach_selects_distinct_tiles() {
    for right in [false, true] {
        // Reach 7 remains in the adjacent clear cell; reach 11 crosses into solid.
        let actor = Position {
            x: if right { 22 } else { 25 },
            y: 24,
        };
        let mut map = layout(56);
        let solid_x = if right { 33 } else { 14 };
        map.cells[solid_x / 16] = 55;
        let mut probes = TerrainProbes::new(&map, Position::default());
        assert!(!probes.wall_blocked(actor, right, 7));
        assert!(probes.wall_blocked(actor, right, 11));
        assert_eq!(probes.position().x, solid_x as u16);
    }
}

#[test]
fn terrain_probes_ground_threshold_and_all_alignment_bits() {
    for low in 0..16 {
        for tile in [71, 72] {
            let map = layout(tile);
            let initial = Position {
                x: 0x7777,
                y: 0x2233,
            };
            let actor = Position { x: 48, y: 32 + low };
            let mut probes = TerrainProbes::new(&map, initial);
            let grounded = low < 4 && tile == 71;
            assert_eq!(probes.grounded(actor), grounded, "low={low} tile={tile}");
            assert_eq!(
                probes.legacy_move_flags(),
                Some(if grounded { 136 } else { 128 })
            );
            assert_eq!(
                probes.position(),
                if low < 4 {
                    actor
                } else {
                    Position {
                        x: initial.x,
                        y: 0x2220 + low,
                    }
                }
            );
        }
    }
}

#[test]
fn terrain_probes_misaligned_ground_retains_wall_high_y_and_x() {
    let map = layout(96);
    let mut probes = TerrainProbes::new(
        &map,
        Position {
            x: 0x7777,
            y: 0x8888,
        },
    );
    assert!(!probes.wall_blocked(Position { x: 32, y: 0x104 }, true, 7));
    assert_eq!(probes.position(), Position { x: 39, y: 0xff });
    assert!(!probes.grounded(Position { x: 33, y: 0x104 }));
    assert_eq!(probes.position(), Position { x: 39, y: 4 });
    assert_eq!(probes.legacy_move_flags(), Some(128));
}

#[test]
fn terrain_probes_coordinate_wrap_mask_and_missing_tile() {
    let map = layout(56);
    let mut probes = TerrainProbes::new(&map, Position::default());
    // X wraps at u16, while tile coordinates independently mask to eight bits.
    assert!(!probes.wall_blocked(Position { x: 0xfffc, y: 24 }, true, 11));
    assert_eq!(probes.position(), Position { x: 7, y: 19 });
    assert!(!probes.wall_blocked(Position { x: 0, y: 24 }, false, 7));
    assert_eq!(probes.position(), Position { x: 0xfff9, y: 19 });
    assert!(!probes.wall_blocked(
        Position {
            x: 0x1030,
            y: 0x1018
        },
        true,
        7
    ));
    assert_eq!(
        probes.position(),
        Position {
            x: 0x1037,
            y: 0x1013
        }
    );
    // Wrapped Y maps beyond the 48-row layout: absent tiles read as zero.
    assert!(probes.wall_blocked(Position { x: 0, y: 0 }, false, 11));
    assert_eq!(
        probes.position(),
        Position {
            x: 0xfff5,
            y: 0xfff5
        }
    );
    assert_eq!(probes.legacy_move_flags(), Some(34));
}

#[test]
fn terrain_probes_no_probe_preserves_projection_and_has_no_flags() {
    let map = layout(0);
    let initial = Position {
        x: 0x1234,
        y: 0xabcd,
    };
    let probes = TerrainProbes::new(&map, initial);
    assert_eq!(probes.position(), initial);
    assert_eq!(probes.legacy_move_flags(), None);
}
