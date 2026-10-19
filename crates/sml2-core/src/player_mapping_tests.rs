use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .expect("standalone extracted object tables")
}

fn bytes(cells: &[Cell]) -> Vec<[u8; 4]> {
    cells.iter().map(|c| [c.y, c.x, c.tile, c.attr]).collect()
}

#[test]
fn native_metasprite_append_concatenates_real_ending_poses() {
    let tables = tables();
    let mut cells = vec![Cell {
        y: 1,
        x: 2,
        tile: 3,
        attr: 4,
    }];
    assert_eq!(
        append_cells(
            &tables,
            0xE5,
            ScreenPosition { x: 250, y: 252 },
            CellStyle::default(),
            &mut cells,
        ),
        Some(())
    );
    assert_eq!(
        append_cells(
            &tables,
            0xEF,
            ScreenPosition { x: 20, y: 30 },
            CellStyle {
                force_palette_one: true,
                behind_background: true
            },
            &mut cells,
        ),
        Some(())
    );
    assert_eq!(
        bytes(&cells),
        vec![
            [1, 2, 3, 4],
            [252, 250, 99, 0],
            [252, 2, 100, 0],
            [4, 250, 115, 0],
            [4, 2, 116, 0],
            [30, 20, 187, 0xD0],
            [30, 28, 187, 0xF0],
            [38, 20, 171, 0xD0],
            [38, 28, 171, 0xF0],
        ]
    );
}

#[test]
fn native_metasprite_append_preserves_every_attribute_with_all_styles() {
    let mut tables = tables();
    tables.player_mappings = vec![(0..=255)
        .map(|attr| MappingEntry {
            y: -1,
            x: 1,
            tile: 0x5A,
            attr,
        })
        .collect()];
    for (force_palette_one, behind_background, expected_mask) in [
        (false, false, 0x00),
        (true, false, 0x10),
        (false, true, 0x80),
        (true, true, 0x90),
    ] {
        let mut cells = Vec::new();
        assert_eq!(
            append_cells(
                &tables,
                0,
                ScreenPosition { x: 255, y: 0 },
                CellStyle {
                    force_palette_one,
                    behind_background
                },
                &mut cells,
            ),
            Some(())
        );
        let expected: Vec<_> = (0..=255)
            .map(|attr| [255, 0, 0x5A, attr | expected_mask])
            .collect();
        assert_eq!(bytes(&cells), expected);
    }
}

#[test]
fn native_metasprite_append_wraps_signed_offsets_at_native_screen_input() {
    let mut tables = tables();
    tables.player_mappings = vec![vec![
        MappingEntry {
            y: -127,
            x: -128,
            tile: 3,
            attr: 0xFF,
        },
        MappingEntry {
            y: 127,
            x: 127,
            tile: 7,
            attr: 0x21,
        },
        MappingEntry {
            y: -1,
            x: -1,
            tile: 9,
            attr: 0x02,
        },
    ]];
    let mut cells = Vec::new();
    assert_eq!(
        append_cells(
            &tables,
            0,
            ScreenPosition { x: 0, y: 0 },
            CellStyle::default(),
            &mut cells,
        ),
        Some(())
    );
    assert_eq!(
        bytes(&cells),
        vec![
            [129, 128, 3, 0xFF],
            [127, 127, 7, 0x21],
            [255, 255, 9, 0x02],
        ]
    );
    cells.clear();
    append_cells(
        &tables,
        0,
        ScreenPosition { x: 255, y: 255 },
        CellStyle::default(),
        &mut cells,
    )
    .unwrap();
    assert_eq!(
        bytes(&cells),
        vec![
            [128, 127, 3, 0xFF],
            [126, 126, 7, 0x21],
            [254, 254, 9, 0x02],
        ]
    );
}

#[test]
fn native_metasprite_append_distinguishes_missing_and_empty_without_mutation() {
    let mut tables = tables();
    tables.player_mappings = vec![vec![]];
    let mut cells = vec![Cell {
        y: 11,
        x: 22,
        tile: 33,
        attr: 44,
    }];
    for (frame, expected) in [(0, Some(())), (1, None), (255, None)] {
        assert_eq!(
            append_cells(
                &tables,
                frame,
                ScreenPosition::default(),
                CellStyle::default(),
                &mut cells
            ),
            expected
        );
        assert_eq!(bytes(&cells), vec![[11, 22, 33, 44]]);
    }
    assert_eq!(draw_cells(&tables, 0, 0, 0, 0, 0, false), Some(vec![]));
    assert_eq!(draw_cells(&tables, 1, 0, 0, 0, 0, false), None);
}

#[test]
fn native_metasprite_append_legacy_draw_cells_projects_and_styles_real_player_frame() {
    let tables = tables();
    // Different world high bytes deliberately exercise the existing byte projection.
    let plain = draw_cells(&tables, 0, 0x1201, 0x34FE, 0x56FF, 0x7803, false).unwrap();
    assert_eq!(
        bytes(&plain),
        vec![
            [81, 91, 1, 0x20],
            [81, 99, 0, 0x20],
            [89, 91, 17, 0x20],
            [89, 99, 16, 0x20],
            [97, 90, 35, 0x20],
            [97, 98, 34, 0x20],
            [105, 90, 55, 0x20],
            [105, 98, 54, 0x20],
        ]
    );
    let flash = draw_cells(&tables, 0, 0x1201, 0x34FE, 0x56FF, 0x7803, true).unwrap();
    assert_eq!(
        bytes(&flash),
        vec![
            [81, 91, 1, 0x30],
            [81, 99, 0, 0x30],
            [89, 91, 17, 0x30],
            [89, 99, 16, 0x30],
            [97, 90, 35, 0x30],
            [97, 98, 34, 0x30],
            [105, 90, 55, 0x30],
            [105, 98, 54, 0x30],
        ]
    );
}

#[test]
fn pipe_travel_puts_every_player_cell_behind_the_background() {
    // $52D5-$52DD: with $FFBB cleared the expander sets attr bit 7 on every
    // entry, which is what lets a pipe's solid BG tiles cover Mario while he
    // travels through it ($530D clears $FFBB for sPipeTravelDirection >= $10).
    let t = tables();
    let front = draw_cells(&t, 0, 100, 100, 0, 0, false).expect("frame 0");
    assert!(front.iter().all(|c| c.attr & 0x80 == 0), "walking is in front");

    let behind = draw_cells_styled(
        &t,
        0,
        100,
        100,
        0,
        0,
        CellStyle {
            force_palette_one: false,
            behind_background: true,
        },
    )
    .expect("frame 0");
    assert!(!behind.is_empty());
    assert!(
        behind.iter().all(|c| c.attr & 0x80 != 0),
        "every cell carries the priority bit: {:?}",
        behind.iter().map(|c| c.attr).collect::<Vec<_>>()
    );
    // The mapping's own bits survive alongside it.
    assert_eq!(behind.len(), front.len());
    for (a, b) in front.iter().zip(behind.iter()) {
        assert_eq!((a.tile, a.x, a.y), (b.tile, b.x, b.y));
        assert_eq!(b.attr, a.attr | 0x80);
    }
}
