use super::*;

impl CastleSequence {
    /// $30697: arrive, leave, wait for input, then fade into level 24.
    pub(super) fn door_tick(&mut self, input: SequenceInput, out: &mut SequenceOut) {
        let frame = ((input.frame & 0x0C) >> 2) as usize;
        match self.menu {
            0 | 1 => {
                let arriving = self.menu == 0;
                self.sprite_id = if arriving {
                    [0xBB, 0xBC, 0xBB, 0xBD][frame]
                } else {
                    [0xB8, 0xB9, 0xB8, 0xBA][frame]
                };
                self.menu_timer = self.menu_timer.wrapping_sub(1);
                if self.menu_timer == 0 {
                    if arriving {
                        self.menu = 2;
                    } else {
                        self.fade_low = 0;
                        self.fade_high = 0;
                        self.state = STATE_MAP;
                        out.done = true;
                        out.transition = Some(CastleTransition::ReturnToMap { walked_out: true });
                    }
                } else {
                    self.walk_y = if arriving {
                        136 + (self.menu_timer >> 1)
                    } else {
                        168 - (self.menu_timer >> 1)
                    };
                }
            }
            2 => {
                self.sprite_id = 0xBB;
                // $46E1: six coins plus A/Start/Up enters; B does nothing.
                if self.golden_count == 6 && input.keys_pressed & 0x49 != 0 {
                    self.menu = 3;
                    self.fade_low = 64;
                } else if input.keys_pressed == 0x80 {
                    self.menu = 1;
                    self.menu_timer = 64;
                }
            }
            3 => {
                const PALETTES: [(u8, u8, u8); 4] = [
                    (0xFF, 0xFF, 0xFF),
                    (0xFA, 0xEA, 0xBE),
                    (0xE9, 0xE5, 0x79),
                    (0xE4, 0xD0, 0x38),
                ];
                (self.bgp, self.obp1, self.obp2) = PALETTES[((self.fade_low & 0xF0) >> 4) as usize];
                self.sprite_id = 0xBB;
                if self.fade_low == 0 {
                    self.level = 24;
                    self.state = 0x16;
                    out.done = true;
                    out.transition = Some(CastleTransition::EnterLevel(24));
                }
            }
            _ => unreachable!("source door menu is bounded to 0..3"),
        }
    }
}
