use sml2_core::player_sim::PlayerKeys;

/// Deterministic held input, sampled once per native gameplay frame. CLI tapes
/// retain their original single-byte grammar; native callers can hold several
/// buttons together without inventing additional command-line characters.
pub enum PlayTape {
    Ascii(String),
    Buttons(Vec<PlayerKeys>),
}

impl From<String> for PlayTape {
    fn from(tape: String) -> Self {
        Self::Ascii(tape)
    }
}

impl From<Vec<PlayerKeys>> for PlayTape {
    fn from(tape: Vec<PlayerKeys>) -> Self {
        Self::Buttons(tape)
    }
}

impl PlayTape {
    pub fn frame(&self, index: usize) -> Option<PlayerKeys> {
        match self {
            Self::Buttons(tape) => tape.get(index).copied(),
            Self::Ascii(tape) => {
                let d = *tape.as_bytes().get(index)? as char;
                Some(PlayerKeys {
                    right: d == 'R',
                    left: d == 'L',
                    up: d == 'U',
                    down: d == 'D',
                    a: d == 'A',
                    b: d == 'B',
                    start: d == 'S',
                    select: d == 'E',
                })
            }
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        match self {
            Self::Ascii(tape) => tape.len(),
            Self::Buttons(tape) => tape.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buttons(keys: PlayerKeys) -> [bool; 8] {
        [
            keys.right,
            keys.left,
            keys.up,
            keys.down,
            keys.a,
            keys.b,
            keys.start,
            keys.select,
        ]
    }

    #[test]
    fn ascii_preserves_each_legacy_button_and_neutral_byte() {
        let tape = PlayTape::Ascii("RLUDABSE".into());
        for index in 0..8 {
            assert_eq!(
                buttons(tape.frame(index).unwrap()),
                std::array::from_fn(|i| i == index)
            );
        }
        let neutral = PlayTape::Ascii(".?rludabseé".into());
        for index in 0..neutral.len() {
            assert_eq!(buttons(neutral.frame(index).unwrap()), [false; 8]);
        }
        assert!(tape.frame(8).is_none());
        assert!(tape.frame(9).is_none());
        assert!(PlayTape::Ascii(String::new()).frame(0).is_none());
    }

    #[test]
    fn typed_frames_preserve_every_simultaneous_button_combination() {
        let frames = (0..=255u8)
            .map(|mask| PlayerKeys {
                right: mask & 1 != 0,
                left: mask & 2 != 0,
                up: mask & 4 != 0,
                down: mask & 8 != 0,
                a: mask & 16 != 0,
                b: mask & 32 != 0,
                start: mask & 64 != 0,
                select: mask & 128 != 0,
            })
            .collect::<Vec<_>>();
        let tape = PlayTape::Buttons(frames);
        for mask in 0..256usize {
            assert_eq!(
                buttons(tape.frame(mask).unwrap()),
                std::array::from_fn(|i| mask & (1 << i) != 0)
            );
        }
        assert!(tape.frame(256).is_none());
        assert!(PlayTape::Buttons(Vec::new()).frame(0).is_none());
    }
}
