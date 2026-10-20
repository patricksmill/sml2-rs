//! Native presentation previews, not console captures. Writes PNGs under /tmp.
use sml2_core::castle_sequence::{
    CastleSequence, SequenceInput, GOLDEN_COIN_OFFSETS, STATE_REVEAL_D,
};
use sml2_core::map::render_castle_sequence;
use sml2_runtime::session::SessionAssets;

fn write(name: &str, assets: &SessionAssets, seq: &CastleSequence) {
    let frame = render_castle_sequence(
        &assets.castle_reveal,
        &assets.castle_scripts,
        &assets.sprite_metas,
        seq,
        (160, 144),
    );
    let file = std::fs::File::create(format!("/tmp/{name}.png")).unwrap();
    let mut encoder = png::Encoder::new(file, 160, 144);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&frame)
        .unwrap();
}

fn main() {
    let assets = SessionAssets::load().unwrap();
    let mut seq = CastleSequence::default();
    seq.enter(&[0x80, 0x80, 0, 0x80, 0x80, 0x80]);
    for frame in 0..65 {
        seq.tick(
            SequenceInput {
                frame,
                ..Default::default()
            },
            0,
            &mut [0; 42],
        );
    }
    write("castle_locked", &assets, &seq);
    seq.enter(&[0x80; 6]);
    for frame in 0..65 {
        seq.tick(
            SequenceInput {
                frame,
                ..Default::default()
            },
            0,
            &mut [0; 42],
        );
    }
    write("castle_open", &assets, &seq);
    seq.tick(
        SequenceInput {
            keys_pressed: 1,
            ..Default::default()
        },
        0,
        &mut [0; 42],
    );
    for _ in 0..63 {
        seq.tick(Default::default(), 0, &mut [0; 42]);
    }
    write("castle_faded", &assets, &seq);

    seq.enter(&[0x80, 0x80, 0, 0x80, 0x80, 0x80]);
    seq.level = 23;
    seq.state = STATE_REVEAL_D;
    let mut progress = [0; 42];
    for (coin, offset) in GOLDEN_COIN_OFFSETS.into_iter().enumerate() {
        progress[offset] = seq.coins[coin];
    }
    let mut rows = 0;
    for frame in 0..2000u16 {
        seq.tick(
            SequenceInput {
                frame: frame as u8,
                ..Default::default()
            },
            14,
            &mut progress,
        );
        if seq.door_open_rows != rows {
            rows = seq.door_open_rows;
            write(&format!("castle_rows_{rows}"), &assets, &seq);
        }
    }
}
