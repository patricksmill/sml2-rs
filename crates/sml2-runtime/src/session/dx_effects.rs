//! DX RGB555 scene operations (`$0C:$706B`, `$0F:$7700/$77A1`).
//! Sources remain immutable: projection is independent of render count.

/// Saturate each RGB555 channel separately; never carry into a neighbor.
pub(crate) fn adjust(bytes: &[u8], delta: [i16; 3]) -> Vec<u8> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .flat_map(|pair| {
            let color = u16::from_le_bytes([pair[0], pair[1]]);
            let mut out = 0;
            for (channel, change) in delta.into_iter().enumerate() {
                let value = ((color >> (channel * 5)) & 31) as i16;
                out |= ((value + change).clamp(0, 31) as u16) << (channel * 5);
            }
            out.to_le_bytes()
        })
        .collect()
}

/// `$0C:$706B`: timer `$3F/$2F/$1F/$0F` subtracts RGB 16/4/4;
/// timer zero uploads literal black even when green/blue remain nonzero.
pub(crate) fn castle_entry(bytes: &[u8], timer: u8) -> Vec<u8> {
    if timer == 0 {
        return vec![0; bytes.len()];
    }
    let steps = 64u16.saturating_sub(u16::from(timer)).div_ceil(16) as i16;
    adjust(bytes, [-16 * steps, -4 * steps, -4 * steps])
}

/// `$20:$6A66`'s `$D300` buffer: 32 RGB555 `$7FFF` words. Coin rooms
/// (`$26:$765A`) and map returns (`$0F:$78AD`) upload it to BG and OBJ
/// before their loads; boss reloads (`$36:$7746`) blank the whole frame.
pub(crate) fn white(len: usize) -> Vec<u8> {
    [0xff, 0x7f].repeat(len / 2)
}

/// `$0C:$7118`: palette index `$9C`, four flashing frames in every 32.
pub(crate) fn castle_coin(bytes: &mut [u8], frame: u8) {
    let colors = if frame & 31 < 4 {
        [0x3f, 0x0b, 0xb8, 0x06]
    } else {
        [0x7a, 0x1a, 0x85, 0x00]
    };
    bytes[0x1c..0x20].copy_from_slice(&colors);
}

/// `$0F:$7700`: the first wait precedes the request; the next VBlank uploads it.
pub(crate) fn map_exit(bytes: &[u8], waits: u16, period: u16) -> Vec<u8> {
    let steps = if waits < 2 {
        0
    } else {
        (1 + (waits - 2) / period).min(4)
    };
    adjust(bytes, [steps as i16 * 10; 3])
}

/// `$0F:$77A1`: restore the original source before each derived step.
pub(crate) fn map_return(bytes: &[u8], waits: u16) -> Vec<u8> {
    let step = (waits / 3).min(4);
    let increase = match step {
        0 => 31,
        1 => 30,
        2 => 20,
        3 => 10,
        _ => 0,
    };
    if step == 0 {
        white(bytes.len())
    } else {
        adjust(bytes, [increase; 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_arithmetic_saturates_without_cross_channel_carry() {
        // RGB (31, 1, 0), (0, 31, 1), (1, 0, 31).
        let colors = [0x3f, 0x00, 0xe0, 0x07, 0x01, 0x7c];
        assert_eq!(
            adjust(&colors, [10; 3]),
            [0x7f, 0x29, 0xea, 0x2f, 0x4b, 0x7d]
        );
        assert_eq!(
            adjust(&colors, [-16, -4, -4]),
            [0x0f, 0x00, 0x60, 0x03, 0x00, 0x6c]
        );
        assert_eq!(colors, [0x3f, 0, 0xe0, 7, 1, 0x7c]);
    }

    #[test]
    fn castle_steps_follow_timer_edges_and_force_black_at_zero() {
        let original = [0xff, 0x7f].repeat(32);
        for (timer, word) in [
            (64, 0x7fff),
            (63, 0x6f6f),
            (48, 0x6f6f),
            (47, 0x5ee0),
            (32, 0x5ee0),
            (31, 0x4e60),
            (16, 0x4e60),
            (15, 0x3de0),
            (1, 0x3de0),
            (0, 0),
        ] {
            assert_eq!(
                castle_entry(&original, timer),
                (word as u16).to_le_bytes().repeat(32),
                "timer {timer}"
            );
        }
        assert_eq!(original, [0xff, 0x7f].repeat(32));
    }

    #[test]
    fn exit_uploads_follow_waits_and_preserve_source() {
        let original = [0, 0].repeat(32);
        for (waits, word) in [
            (0, 0),
            (1, 0),
            (2, 0x294a),
            (10, 0x294a),
            (11, 0x5294),
            (20, 0x7bde),
            (29, 0x7fff),
            (36, 0x7fff),
        ] {
            assert_eq!(
                map_exit(&original, waits, 9),
                (word as u16).to_le_bytes().repeat(32)
            );
        }
        assert_eq!(map_exit(&original, 6, 5), [0x4a, 0x29].repeat(32));
        assert_eq!(map_exit(&original, 7, 5), [0x94, 0x52].repeat(32));
        assert_eq!(original, [0; 64]);
    }

    #[test]
    fn return_restores_original_before_each_step_and_coin_flash_changes_only_two_colors() {
        let original = [0x00, 0x00].repeat(32);
        for (waits, word) in [
            (0, 0x7fff),
            (2, 0x7fff),
            (3, 0x7bde),
            (5, 0x7bde),
            (6, 0x5294),
            (9, 0x294a),
            (12, 0),
        ] {
            assert_eq!(
                map_return(&original, waits),
                (word as u16).to_le_bytes().repeat(32)
            );
        }
        for frame in 0..64u8 {
            let mut bytes = [0xa5; 64];
            castle_coin(&mut bytes, frame);
            assert_eq!(&bytes[..0x1c], &[0xa5; 0x1c]);
            assert_eq!(&bytes[0x20..], &[0xa5; 32]);
            assert_eq!(
                &bytes[0x1c..0x20],
                if frame & 31 < 4 {
                    &[0x3f, 0x0b, 0xb8, 0x06]
                } else {
                    &[0x7a, 0x1a, 0x85, 0]
                }
            );
        }
    }
}
