use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

// Literal ROM records, independently checked through bank-1 pointer table $4000.
// E5..E7: $42A6/$42B7/$42C8; E8..EA: $4273/$4284/$4295;
// EB..EF: $4226/$4237/$4248/$4251/$4262. All terminate with $80.
fn records(frame: u8) -> &'static [[u8; 4]] {
    match frame {
        0xE5 => &[
            [0, 0, 0x63, 0],
            [0, 8, 0x64, 0],
            [8, 0, 0x73, 0],
            [8, 8, 0x74, 0],
        ],
        0xE6 => &[
            [0, 0, 0x65, 0],
            [0, 8, 0x66, 0],
            [8, 0, 0x75, 0],
            [8, 8, 0x76, 0],
        ],
        0xE7 => &[
            [0, 0, 0x67, 0],
            [0, 8, 0x68, 0],
            [8, 0, 0x77, 0],
            [8, 8, 0x78, 0],
        ],
        0xE8 => &[
            [0, 0, 0x10, 0],
            [0, 8, 0x53, 0],
            [8, 0, 0x62, 0],
            [8, 8, 0x63, 0],
        ],
        0xE9 => &[
            [255, 0, 0x6B, 0],
            [255, 8, 0x6C, 0],
            [7, 0, 0x7B, 0],
            [7, 8, 0x7C, 0],
        ],
        0xEA => &[
            [253, 0, 0x3E, 0],
            [253, 8, 0x3F, 0],
            [5, 0, 0x3D, 0],
            [5, 8, 0x5F, 0],
        ],
        0xEB => &[
            [0, 0, 0xAB, 0],
            [0, 8, 0xAB, 0x20],
            [8, 0, 0xBB, 0],
            [8, 8, 0xBB, 0x20],
        ],
        0xEC => &[
            [0, 0, 0x8C, 0],
            [0, 8, 0x8C, 0x20],
            [8, 0, 0x9C, 0],
            [8, 8, 0x9C, 0x20],
        ],
        0xED => &[[4, 0, 0x8F, 0], [4, 8, 0x8F, 0]],
        0xEE => &[
            [0, 0, 0x9C, 0x40],
            [0, 8, 0x9C, 0x60],
            [8, 0, 0x8C, 0x40],
            [8, 8, 0x8C, 0x60],
        ],
        0xEF => &[
            [0, 0, 0xBB, 0x40],
            [0, 8, 0xBB, 0x60],
            [8, 0, 0xAB, 0x40],
            [8, 8, 0xAB, 0x60],
        ],
        _ => panic!("not an ending frame"),
    }
}

fn rendered(frame: u8, x: u8, y: u8, attr_mask: u8) -> Vec<[u8; 4]> {
    records(frame)
        .iter()
        .map(|r| {
            [
                y.wrapping_add(r[0]),
                x.wrapping_add(r[1]),
                r[2],
                r[3] | attr_mask,
            ]
        })
        .collect()
}

fn bytes(cells: &[Cell]) -> Vec<[u8; 4]> {
    cells.iter().map(|c| [c.y, c.x, c.tile, c.attr]).collect()
}

fn prefix() -> Vec<Cell> {
    vec![Cell {
        y: 1,
        x: 2,
        tile: 3,
        attr: 4,
    }]
}

#[test]
fn ending_sprites_walk_timer_boundaries_and_pair_fallthrough() {
    let tables = tables();
    for (initial, next, walk_frame, pair_frame) in [
        (0, 1, 0xE5, 0xE8),
        (14, 15, 0xE5, 0xE8),
        (15, 16, 0xE6, 0xE9),
        (30, 31, 0xE6, 0xE9),
        (31, 32, 0xE7, 0xEA),
        (46, 47, 0xE7, 0xEA),
        (47, 0, 0xE5, 0xE8),
        (48, 0, 0xE5, 0xE8),
        (254, 0, 0xE5, 0xE8),
        (255, 0, 0xE5, 0xE8),
    ] {
        for credits_y in [0, 1, 2, 203, 204, 250, 255] {
            let mut timer = initial;
            let mut cells = prefix();
            assert_eq!(
                append_walk(&tables, &mut timer, credits_y, &mut cells),
                Ok(())
            );
            assert_eq!(timer, next);
            let mut expected = vec![[1, 2, 3, 4]];
            expected.extend(rendered(walk_frame, 84, credits_y, 0x90));
            assert_eq!(
                bytes(&cells),
                expected,
                "walk timer {initial} y {credits_y}"
            );

            timer = initial;
            cells = prefix();
            assert_eq!(
                append_pair(&tables, &mut timer, credits_y, &mut cells),
                Ok(())
            );
            assert_eq!(timer, next);
            expected = vec![[1, 2, 3, 4]];
            expected.extend(rendered(pair_frame, 84, credits_y, 0));
            expected.extend(rendered(0xEB, 79, credits_y.wrapping_add(52), 0));
            assert_eq!(
                bytes(&cells),
                expected,
                "pair timer {initial} y {credits_y}"
            );
        }
    }
}

#[test]
fn ending_sprites_secondary_inherits_priority_without_palette_override() {
    let tables = tables();
    for behind in [false, true] {
        for credits_y in 0..=255_u8 {
            let mut cells = prefix();
            append_secondary(&tables, credits_y, behind, &mut cells).unwrap();
            let mut expected = vec![[1, 2, 3, 4]];
            expected.extend(rendered(
                0xEB,
                79,
                credits_y.wrapping_add(52),
                if behind { 0x80 } else { 0 },
            ));
            assert_eq!(bytes(&cells), expected);
        }
    }
}

#[test]
fn ending_sprites_burst_full_frame_cadence_and_sound_requests() {
    let tables = tables();
    for frame_byte in 0..=255_u8 {
        // Four frames per source table entry; repeat after 32 frames.
        let expected_frame = match frame_byte % 32 {
            0..=3 => 0xEB,
            4..=7 | 20..=23 => 0xEC,
            8..=11 | 24..=27 => 0xED,
            12..=15 | 28..=31 => 0xEE,
            16..=19 => 0xEF,
            _ => unreachable!(),
        };
        for frame in [u64::from(frame_byte), 0x1234_5600 + u64::from(frame_byte)] {
            let mut cells = prefix();
            let outcome = append_burst(&tables, FrameTick(frame), &mut cells);
            assert_eq!(outcome.sprites, Ok(()));
            assert_eq!(
                outcome.sound_effect,
                if frame_byte % 2 == 1 {
                    Some(EndingSoundEffect::RequestSound1(24))
                } else {
                    None
                }
            );
            let mut expected = vec![[1, 2, 3, 4]];
            expected.extend(rendered(expected_frame, 79, 96, 0));
            assert_eq!(bytes(&cells), expected, "frame {frame}");
        }
    }
}

#[test]
fn ending_sprites_missing_assets_preserve_source_timer_and_effect_order() {
    let mut tables = tables();
    tables.player_mappings.truncate(0xEB);
    let mut cells = prefix();
    let mut timer = 15;
    assert_eq!(
        append_pair(&tables, &mut timer, 0, &mut cells),
        Err(MissingEndingFrame(0xEB))
    );
    assert_eq!(timer, 16);
    let mut expected = vec![[1, 2, 3, 4]];
    expected.extend(rendered(0xE9, 84, 0, 0));
    assert_eq!(bytes(&cells), expected);
    for frame in [0, 1] {
        cells = prefix();
        let outcome = append_burst(&tables, FrameTick(frame), &mut cells);
        assert_eq!(outcome.sprites, Err(MissingEndingFrame(0xEB)));
        assert_eq!(
            outcome.sound_effect,
            if frame == 1 {
                Some(EndingSoundEffect::RequestSound1(24))
            } else {
                None
            }
        );
        assert_eq!(bytes(&cells), vec![[1, 2, 3, 4]]);
    }
    tables.player_mappings.clear();
    cells = prefix();
    timer = 47;
    assert_eq!(
        append_walk(&tables, &mut timer, 0, &mut cells),
        Err(MissingEndingFrame(0xE5))
    );
    assert_eq!(timer, 0);
    timer = 15;
    assert_eq!(
        append_pair(&tables, &mut timer, 0, &mut cells),
        Err(MissingEndingFrame(0xE9))
    );
    assert_eq!(timer, 16);
    assert_eq!(
        append_secondary(&tables, 0, true, &mut cells),
        Err(MissingEndingFrame(0xEB))
    );
    assert_eq!(bytes(&cells), vec![[1, 2, 3, 4]]);
}

#[test]
fn ending_sprites_native_pose_uses_shared_append_consumer() {
    let tables = tables();
    let mut cells = prefix();
    let pose = burst_pose(FrameTick(16));
    append_cells(&tables, pose.frame, pose.position, pose.style, &mut cells).unwrap();
    assert_eq!(
        bytes(&cells),
        vec![
            [1, 2, 3, 4],
            [96, 79, 0xBB, 0x40],
            [96, 87, 0xBB, 0x60],
            [104, 79, 0xAB, 0x40],
            [104, 87, 0xAB, 0x60],
        ]
    );
}
