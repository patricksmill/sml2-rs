use super::*;
use crate::video::{PalettePixel, PaletteSource};

fn metas() -> SpriteMetas {
    let scripts = crate::castle::CastleScripts::from_parts(
        include_bytes!("../../../../extracted/map/castle_scripts.bin").to_vec(),
        include_str!("../../../../extracted/map/castle_scripts.json"),
    )
    .unwrap();
    SpriteMetas::new(scripts.blob("sprite_metasprites").unwrap())
}

fn font() -> Vec<u8> {
    let ending = crate::ending_scene::EndingAssets::from_json(include_str!(
        "../../../../extracted/ending/scene.json"
    ))
    .unwrap();
    ending
        .transfer_source(0x1A, 0x4656, EASY_CARD_FONT_LEN)
        .unwrap()
        .to_vec()
}

/// Sprite 240 is eight letters on one row, "EASY" and "MODE" with a
/// one-cell gap, at OAM (52, 80): screen columns 44..116, rows 64..72.
#[test]
fn the_card_draws_easy_mode_in_one_row_at_its_rom_position() {
    let frame: Vec<_> =
        render_easy_card_output(&font(), &metas(), (160, 144), OutputMode::PaletteIndices)
            .into_iter()
            .map(PalettePixel::from_token)
            .collect();
    let lit: Vec<(usize, usize)> = (0..160 * 144)
        .filter(|&i| frame[i].source != PaletteSource::Bg)
        .map(|i| (i % 160, i / 160))
        .collect();
    assert!(!lit.is_empty());
    assert!(lit
        .iter()
        .all(|&(x, y)| (44..116).contains(&x) && (64..72).contains(&y)));
    // The gap cell between the words stays background.
    assert!(lit.iter().all(|&(x, _)| !(76..84).contains(&x)));
    // Each of the eight letter cells has ink.
    for cell in [0, 1, 2, 3, 5, 6, 7, 8] {
        let x0 = 44 + cell * 8;
        assert!(
            lit.iter().any(|&(x, _)| (x0..x0 + 8).contains(&x)),
            "cell {cell}"
        );
    }
    assert!(frame
        .iter()
        .filter(|p| p.source == PaletteSource::Bg)
        .all(|p| p.shade == 0));
}
