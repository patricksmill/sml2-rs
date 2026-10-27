//! DX `$26:$6B0B`: eight levels change palette sets as Mario crosses regions.
//!
//! The dispatcher runs once per gameplay frame on Mario's X+8 and Y+32
//! (`$02:$40FD` stages them at `$AF30-$AF33`). Each level's branch loads a
//! bank-`$26` BG or OBJ set, or writes a few colors straight into OBJ
//! palette 5, and remembers the section in `$FFAD`; a section already
//! loaded is skipped and positions without a trigger keep the last one.
//! Level loads clear `$FFAD` (`$24:$793C`) with the level's own palettes.
//! Presentation only: the native simulation never reads it.

/// What `$6B0B` has loaded since the level started.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegionalPalettes {
    /// `$FFAD`: the section the level's branch last loaded.
    pub selector: u8,
    /// The bank-`$26` BG set last loaded; `None` keeps the level's own.
    pub bg: Option<u16>,
    /// The bank-`$26` OBJ set last loaded; `None` keeps the level's own.
    pub obj: Option<u16>,
    /// `$FF6A`/`$FF6B` writes over the OBJ set, `(byte index, value)`.
    pub obj_writes: Vec<(u8, u8)>,
}

impl RegionalPalettes {
    /// One `$6B0B` pass for level `level_id` with Mario at `(x, y)`.
    pub fn update(&mut self, level_id: u8, x: u16, y: u16) {
        let x = x.wrapping_add(8);
        let y = y.wrapping_add(32);
        let ([x_lo, page], [y_lo, y_hi]) = (x.to_le_bytes(), y.to_le_bytes());
        match level_id {
            0x0C => match page {
                1 | 0x0E => self.load(0, Some(0x6100), None),
                2 | 4 | 0x0A | 0x0D => self.load(1, Some(0x6140), None),
                5 | 7 | 9 => self.load(2, Some(0x6180), None),
                _ => {}
            },
            0x15 => {
                // `$6BC4` leaves the compared byte in A: on page 5 it is the
                // Y+32 low byte by the time `$6BDC` tests 9 and `$0B`.
                let mut a = page;
                if a == 5 {
                    if y_hi != 2 {
                        return;
                    }
                    a = y_lo;
                    if a >= 0x5D {
                        return self.load(1, Some(0x60C0), None);
                    }
                }
                if a == 9 {
                    self.load(1, Some(0x60C0), None);
                } else if a == 0x0B && y_hi == 2 && y_lo < 0x15 {
                    self.load(0, Some(0x6080), None);
                }
            }
            0x19 => {
                // `$6C5B`: half-pages counted in eighths, only ever increasing.
                let half = if x_lo & 0x80 != 0 { 8 } else { 0 };
                let section = page.rotate_left(4).wrapping_add(half);
                if section <= self.selector {
                    return;
                }
                self.selector = section;
                if (0x18..=0x70).contains(&section) && section % 8 == 0 {
                    let offset = u16::from(section - 0x18) / 8 * 0x40;
                    self.bg = Some(0x5A80 + offset);
                    self.obj = Some(0x5D80 + offset);
                    self.obj_writes.clear();
                }
            }
            0x03 => match page {
                0 => self.load(0, Some(0x5A00), None),
                1 => self.load(1, Some(0x5A40), None),
                0x0A if self.selector != 1 => {
                    self.bg = Some(0x5A40);
                    self.write(1, 0x2A, &[0xFF, 0x7F, 0xC8, 0x11]);
                }
                5 => self.write(0, 0x2A, &[0x95, 0x09, 0x9C, 0x2E]),
                6 => self.write(1, 0x2A, &[0xFF, 0x7F, 0xC8, 0x11]),
                _ => {}
            },
            0x07 => match page {
                7 => self.write(0, 0x2A, &[0xD9, 0x7B, 0x45, 0x6B]),
                8 => self.write(1, 0x2A, &[0xB5, 0x56, 0x4A, 0x29]),
                _ => {}
            },
            0x16 => match page {
                8 | 0x0C => self.write(0, 0x2C, &[0x7D, 0x26]),
                9 | 0x0B => self.write(1, 0x2C, &[0x94, 0x52]),
                _ => {}
            },
            0x0D => match page {
                0 if y_hi != 0 => self.load(0, Some(0x61C0), None),
                1 | 5 if y_hi != 0 => self.load(0x10, Some(0x6200), None),
                6 if y_hi != 0 => self.load(1, Some(0x6240), None),
                8 | 0x0B => self.load(2, Some(0x6280), None),
                _ => {}
            },
            0x18 => match (page, y_hi) {
                (8, 2) => self.load(1, None, Some(0x62C0)),
                (0, 2) => self.load(2, None, Some(0x6300)),
                (5, 1) => self.load(3, None, Some(0x6340)),
                _ => {}
            },
            _ => {}
        }
    }

    /// A branch's set load, skipped when `$FFAD` already names the section.
    fn load(&mut self, selector: u8, bg: Option<u16>, obj: Option<u16>) {
        if self.selector == selector {
            return;
        }
        self.selector = selector;
        if bg.is_some() {
            self.bg = bg;
        }
        if obj.is_some() {
            self.obj = obj;
            self.obj_writes.clear();
        }
    }

    /// A branch's direct OBJ palette RAM write from `index` on, auto-incrementing.
    fn write(&mut self, selector: u8, index: u8, bytes: &[u8]) {
        if self.selector == selector {
            return;
        }
        self.selector = selector;
        for (offset, &value) in bytes.iter().enumerate() {
            let at = index + offset as u8;
            self.obj_writes.retain(|&(i, _)| i != at);
            self.obj_writes.push((at, value));
        }
    }

    /// The BG and OBJ palette bytes in effect, from the level's own sets and
    /// `rom`'s bank-`$26` reads.
    pub fn resolve<'a>(
        &self,
        bg: &'a [u8],
        obj: &'a [u8],
        rom: impl Fn(u16) -> Option<&'a [u8]>,
    ) -> Result<(Vec<u8>, Vec<u8>), String> {
        let source = |set: Option<u16>, own: &'a [u8]| match set {
            Some(address) => rom(address)
                .map(<[u8]>::to_vec)
                .ok_or_else(|| format!("DX palette ${address:04X} missing; re-import the DX ROM")),
            None => Ok(own.to_vec()),
        };
        let bg = source(self.bg, bg)?;
        let mut obj = source(self.obj, obj)?;
        for &(index, value) in &self.obj_writes {
            if let Some(byte) = obj.get_mut(usize::from(index)) {
                *byte = value;
            }
        }
        Ok((bg, obj))
    }
}

#[cfg(test)]
mod tests {
    use super::RegionalPalettes;

    /// Mario at a page and Y+32 high byte, X+8 low byte `$40`.
    fn at(page: u8, y_hi: u8) -> (u16, u16) {
        (
            u16::from_le_bytes([0x40, page]).wrapping_sub(8),
            u16::from_le_bytes([0x40, y_hi]).wrapping_sub(32),
        )
    }

    fn walk(level: u8, steps: &[(u16, u16)]) -> RegionalPalettes {
        let mut state = RegionalPalettes::default();
        for &(x, y) in steps {
            state.update(level, x, y);
        }
        state
    }

    #[test]
    fn a_new_level_keeps_its_own_palettes_until_a_trigger_loads() {
        let state = RegionalPalettes::default();
        let (bg, obj) = state.resolve(&[1; 64], &[2; 64], |_| None).unwrap();
        assert_eq!((bg, obj), (vec![1; 64], vec![2; 64]));
        // Section 0 is what `$FFAD` holds at load, so `$6B6C` skips it.
        let state = walk(0x0C, &[at(1, 1)]);
        assert_eq!(state.bg, None);
    }

    #[test]
    fn mario_zone_conveyor_sections_keep_the_last_set_between_triggers() {
        let mut state = RegionalPalettes::default();
        for (page, selector, bg) in [
            (2, 1, Some(0x6140)),
            (3, 1, Some(0x6140)),
            (5, 2, Some(0x6180)),
            (0x0A, 1, Some(0x6140)),
            (0x0E, 0, Some(0x6100)),
        ] {
            let (x, y) = at(page, 1);
            state.update(0x0C, x, y);
            assert_eq!((state.selector, state.bg), (selector, bg), "page {page}");
        }
    }

    #[test]
    fn level_15_uses_the_y_byte_on_page_5_and_page_b_needs_a_low_y() {
        // Page 9, then page `$0B` low enough on row 2 returns to set 0.
        let state = walk(0x15, &[at(9, 2)]);
        assert_eq!((state.selector, state.bg), (1, Some(0x60C0)));
        let state = walk(0x15, &[at(9, 2), (0x0B40 - 8, 0x0214 - 32)]);
        assert_eq!((state.selector, state.bg), (0, Some(0x6080)));
        let state = walk(0x15, &[at(9, 2), (0x0B40 - 8, 0x0215 - 32)]);
        assert_eq!(state.selector, 1, "Y+32 at $0215 is past $6BF1's bound");
        // Page 5 on row 2: Y+32 low byte `$5D` loads set 1, `$5C` nothing.
        let state = walk(0x15, &[(0x0540 - 8, 0x025D - 32)]);
        assert_eq!(state.bg, Some(0x60C0));
        let state = walk(0x15, &[(0x0540 - 8, 0x025C - 32)]);
        assert_eq!(state.bg, None);
        // ...unless that byte is itself 9, which `$6BDC` then compares.
        let state = walk(0x15, &[(0x0540 - 8, 0x0209 - 32)]);
        assert_eq!(state.bg, Some(0x60C0));
    }

    #[test]
    fn level_19_counts_half_pages_and_never_goes_back() {
        let mut state = RegionalPalettes::default();
        state.update(0x19, 0x0180 - 8, 0x100);
        assert_eq!((state.selector, state.bg), (0x18, Some(0x5A80)));
        assert_eq!(state.obj, Some(0x5D80));
        state.update(0x19, 0x0200 - 8, 0x100);
        assert_eq!((state.selector, state.bg), (0x20, Some(0x5AC0)));
        state.update(0x19, 0x0180 - 8, 0x100);
        assert_eq!(state.selector, 0x20, "walking back keeps the furthest set");
        state.update(0x19, 0x0780 - 8, 0x100);
        assert_eq!(
            (state.selector, state.bg),
            (0x78, Some(0x5AC0)),
            "past $70 nothing loads"
        );
    }

    #[test]
    fn levels_3_7_and_16_write_obj_palette_5_colors() {
        let rom = |_| Some(&[0u8; 64][..]);
        let state = walk(0x03, &[at(6, 1)]);
        let (_, obj) = state.resolve(&[0; 64], &[0; 64], rom).unwrap();
        assert_eq!(&obj[0x2A..0x2E], &[0xFF, 0x7F, 0xC8, 0x11]);
        let state = walk(0x03, &[at(6, 1), at(5, 1)]);
        let (_, obj) = state.resolve(&[0; 64], &[0; 64], rom).unwrap();
        assert_eq!(&obj[0x2A..0x2E], &[0x95, 0x09, 0x9C, 0x2E]);
        // Page `$0A` loads `$5A40` and the same colors as page 6.
        let state = walk(0x03, &[at(0x0A, 1)]);
        assert_eq!((state.selector, state.bg), (1, Some(0x5A40)));
        let state = walk(0x07, &[at(8, 1)]);
        assert_eq!(
            state.obj_writes,
            vec![(0x2A, 0xB5), (0x2B, 0x56), (0x2C, 0x4A), (0x2D, 0x29)]
        );
        let state = walk(0x16, &[at(9, 1), at(0x0C, 1)]);
        assert_eq!(state.obj_writes, vec![(0x2C, 0x7D), (0x2D, 0x26)]);
    }

    #[test]
    fn levels_d_and_18_gate_on_mario_y() {
        assert_eq!(walk(0x0D, &[at(1, 0)]).bg, None, "row 0 skips $6F13");
        assert_eq!(walk(0x0D, &[at(1, 1)]).bg, Some(0x6200));
        assert_eq!(walk(0x0D, &[at(8, 0)]).bg, Some(0x6280));
        assert_eq!(walk(0x0D, &[at(8, 0), at(0, 1)]).bg, Some(0x61C0));
        assert_eq!(walk(0x18, &[at(8, 1)]).obj, None);
        assert_eq!(walk(0x18, &[at(8, 2)]).obj, Some(0x62C0));
        assert_eq!(walk(0x18, &[at(5, 1)]).obj, Some(0x6340));
    }

    #[test]
    fn other_levels_and_untriggered_pages_change_nothing() {
        for level in [0x00, 0x01, 0x0B, 0x1F] {
            assert_eq!(
                walk(level, &[at(1, 1), at(9, 2)]),
                RegionalPalettes::default()
            );
        }
        assert_eq!(walk(0x07, &[at(6, 1)]), RegionalPalettes::default());
    }
}
