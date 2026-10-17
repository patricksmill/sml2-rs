use super::*;
use crate::{
    actor::ScreenPosition,
    object::ObjectTables,
    player::{append_cells, CellStyle},
};

fn cell(bytes: [u8; 4]) -> Cell {
    let [y, x, tile, attr] = bytes;
    Cell { y, x, tile, attr }
}

fn bytes(batch: &SpriteBatch) -> Vec<u8> {
    batch
        .cells()
        .iter()
        .flat_map(|c| [c.y, c.x, c.tile, c.attr])
        .collect()
}

#[test]
fn native_sprite_batch_growth_shrink_and_equality_preserve_exact_boundaries() {
    // Private fixture leaves recognizable data beyond the previous frame.
    let mut batch = SpriteBatch {
        cells: [cell([91, 92, 93, 94]); 40],
        previous_used: 1,
    };
    let a = cell([1, 2, 3, 4]);
    let b = cell([5, 6, 7, 8]);
    let c = cell([9, 10, 11, 12]);
    batch.write_frame(&[a, b, c]).unwrap();
    let mut expected = vec![91, 92, 93, 94].repeat(40);
    expected[..12].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    assert_eq!(bytes(&batch), expected);
    assert_eq!(batch.previous_used(), 3);

    batch.write_frame(&[b]).unwrap();
    expected[..12].copy_from_slice(&[5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(bytes(&batch), expected);
    assert_eq!(batch.previous_used(), 1);

    // Equal length clears precisely the next Y byte, preserving all metadata.
    batch.cells[1] = cell([31, 32, 33, 34]);
    batch.write_frame(&[c]).unwrap();
    expected[..8].copy_from_slice(&[9, 10, 11, 12, 0, 32, 33, 34]);
    assert_eq!(bytes(&batch), expected);
    assert_eq!(batch.previous_used(), 1);
}

#[test]
fn native_sprite_batch_zero_full_bounds_and_rejection_are_precise() {
    let mut batch = SpriteBatch::default();
    assert_eq!(bytes(&batch), [0; 160]);
    assert_eq!(batch.previous_used(), 0);
    batch.cells[0] = cell([1, 2, 3, 4]);
    batch.write_frame(&[]).unwrap();
    let mut expected = vec![0; 160];
    expected[..4].copy_from_slice(&[0, 2, 3, 4]);
    assert_eq!(bytes(&batch), expected);
    assert_eq!(batch.previous_used(), 0);

    let full = [cell([201, 202, 203, 204]); 40];
    for _ in 0..2 {
        batch.write_frame(&full).unwrap();
        assert_eq!(bytes(&batch), vec![201, 202, 203, 204].repeat(40));
        assert_eq!(batch.previous_used(), 40);
    }
    let before = batch.clone();
    assert_eq!(
        batch.write_frame(&[cell([9, 8, 7, 6]); 41]),
        Err(SpriteBatchOverflow { attempted: 41 })
    );
    assert_eq!(batch, before);
    batch.write_frame(&[]).unwrap();
    assert_eq!(bytes(&batch), [0; 160]);
    assert_eq!(batch.previous_used(), 0);
}

#[test]
fn native_sprite_batch_clear_preserves_count_for_next_frame_cleanup() {
    let mut batch = SpriteBatch::default();
    batch.write_frame(&[cell([1, 2, 3, 4]); 4]).unwrap();
    batch.clear_cells();
    assert_eq!(bytes(&batch), [0; 160]);
    assert_eq!(batch.previous_used(), 4);
    // Retained count determines the shrink range, even after a scene clear.
    batch.cells[2] = cell([21, 22, 23, 24]);
    batch.cells[4] = cell([41, 42, 43, 44]);
    batch.write_frame(&[cell([5, 6, 7, 8])]).unwrap();
    let mut expected = vec![0; 160];
    expected[..4].copy_from_slice(&[5, 6, 7, 8]);
    expected[16..20].copy_from_slice(&[41, 42, 43, 44]);
    assert_eq!(bytes(&batch), expected);
    assert_eq!(batch.previous_used(), 1);
}

#[test]
fn native_sprite_batch_consumes_concatenated_real_ending_mappings_across_frames() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let mut frame = Vec::new();
    append_cells(
        &tables,
        0xE5,
        ScreenPosition { x: 250, y: 252 },
        CellStyle::default(),
        &mut frame,
    )
    .unwrap();
    append_cells(
        &tables,
        0xEF,
        ScreenPosition { x: 20, y: 30 },
        CellStyle {
            force_palette_one: true,
            behind_background: true,
        },
        &mut frame,
    )
    .unwrap();
    let mut batch = SpriteBatch::default();
    batch.write_frame(&frame).unwrap();
    let mut expected = vec![0; 160];
    expected[..32].copy_from_slice(&[
        252, 250, 99, 0, 252, 2, 100, 0, 4, 250, 115, 0, 4, 2, 116, 0, 30, 20, 187, 0xD0, 30, 28,
        187, 0xF0, 38, 20, 171, 0xD0, 38, 28, 171, 0xF0,
    ]);
    assert_eq!(bytes(&batch), expected);
    assert_eq!(batch.previous_used(), 8);
    frame.clear();
    append_cells(
        &tables,
        0xE5,
        ScreenPosition { x: 1, y: 2 },
        CellStyle::default(),
        &mut frame,
    )
    .unwrap();
    batch.write_frame(&frame).unwrap();
    expected.fill(0);
    expected[..16].copy_from_slice(&[2, 1, 99, 0, 2, 9, 100, 0, 10, 1, 115, 0, 10, 9, 116, 0]);
    assert_eq!(bytes(&batch), expected);
    assert_eq!(batch.previous_used(), 4);
}
