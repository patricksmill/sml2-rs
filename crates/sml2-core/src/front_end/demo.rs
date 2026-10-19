//! `$38E5`: the attract-demo run-length input decoder.
//!
//! The four tapes live at `$7C00-$7FFF` in the title bank. Each page is a
//! sequence of `(keys, count)` pairs; `keys = $FF` ends the demo. The ROM keeps
//! the page in `$A300`, decrements the current count every frame, and moves to
//! the next pair when it reaches zero (a zero count is skipped).

use super::assets::DEMO_TAPE_LEN;

/// `$4121`: the four demo levels, indexed by `sDemoIndex & 3`.
pub const DEMO_LEVELS: [u8; 4] = [0x00, 0x11, 0x0E, 0x14];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoTape {
    data: [u8; DEMO_TAPE_LEN],
    pos: usize,
    previous: u8,
}

impl DemoTape {
    pub fn new(bytes: &[u8]) -> Self {
        let mut data = [0u8; DEMO_TAPE_LEN];
        let len = bytes.len().min(DEMO_TAPE_LEN);
        data[..len].copy_from_slice(&bytes[..len]);
        Self {
            data,
            pos: 0,
            previous: 0,
        }
    }

    /// One frame: `(held, pressed)` or `None` at the `$FF` terminator.
    pub fn step(&mut self) -> Option<(u8, u8)> {
        loop {
            let keys = *self.data.get(self.pos)?;
            if keys == 0xFF {
                return None;
            }
            let count = *self.data.get(self.pos + 1)?;
            if count == 0 {
                self.pos += 2;
                continue;
            }
            self.data[self.pos + 1] = count - 1;
            let pressed = keys & !self.previous;
            self.previous = keys;
            return Some((keys, pressed));
        }
    }

    /// True when the next read would hit `$FF` (or the page is exhausted).
    pub fn ended(&self) -> bool {
        self.pos + 1 >= DEMO_TAPE_LEN || self.data[self.pos] == 0xFF
    }

    pub fn position(&self) -> usize {
        self.pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_hold_for_their_count_and_emit_the_press_edge() {
        let mut tape = DemoTape::new(&[0x10, 3, 0x11, 2, 0xFF]);
        assert_eq!(tape.step(), Some((0x10, 0x10)));
        assert_eq!(tape.step(), Some((0x10, 0x00)));
        assert_eq!(tape.step(), Some((0x10, 0x00)));
        assert_eq!(tape.step(), Some((0x11, 0x01)));
        assert_eq!(tape.step(), Some((0x11, 0x00)));
        assert_eq!(tape.step(), None);
        assert!(tape.ended());
    }

    #[test]
    fn zero_counts_are_skipped_before_the_next_pair() {
        let mut tape = DemoTape::new(&[0x10, 0, 0x08, 1, 0xFF]);
        assert_eq!(tape.step(), Some((0x08, 0x08)));
        assert_eq!(tape.step(), None);
    }

    #[test]
    fn a_truncated_page_ends_instead_of_reading_past_it() {
        let mut tape = DemoTape::new(&[0x10]);
        assert_eq!(tape.step(), None);
        assert!(tape.ended());
    }
}
