//! Level countdown and its audio requests, ROM `$5D6A..$5E11`.
//! The ending input override at `$5E12` is a separate subsystem.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelTimer {
    pub low: u8,
    pub high: u8,
    pub tick_counter: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimerInput {
    pub dead: u8,
    pub boss_clear: bool,
    pub goal_bell: u8,
    pub current_song: u8,
    pub level_song: u8,
    pub star_high: u8,
}

/// Writes made by the countdown; `None` preserves the caller's current byte.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimerWrites {
    pub dead: Option<u8>,
    pub sfx0: Option<u8>,
    pub sfx1: Option<u8>,
    pub music: Option<u8>,
    pub low_time_flag: Option<u8>,
}

impl LevelTimer {
    /// Level-load initialization: header time is the high BCD byte.
    pub fn new(time_limit_bcd: u8) -> Self {
        Self {
            low: 0,
            high: time_limit_bcd,
            tick_counter: 0x30,
        }
    }

    pub fn step(&mut self, input: TimerInput) -> TimerWrites {
        let mut writes = TimerWrites::default();
        if input.dead != 0 || input.boss_clear || input.goal_bell != 0 {
            return writes;
        }
        self.tick_counter = self.tick_counter.wrapping_sub(1);
        if self.tick_counter != 0 {
            return writes;
        }
        self.tick_counter = 0x30;
        let (low, borrow) = subtract_bcd(self.low, true);
        let (high, borrow) = subtract_bcd(self.high, borrow);
        self.low = low;
        self.high = high;
        if borrow {
            writes.dead = Some(0x80);
            self.low = 0;
            self.high = 0;
            writes.sfx0 = Some(0x0F);
            writes.low_time_flag = Some(0);
            return writes;
        }
        if self.high == 0 && self.low < 0x11 {
            writes.sfx1 = Some(0x0E);
            writes.low_time_flag = Some(1);
        }
        if self.high == 0 {
            if input.current_song != 0 {
                return writes;
            }
            writes.music = Some(if input.level_song & 0xFC == 0x20 {
                input.level_song.wrapping_add(4)
            } else if input.star_high != 0 {
                4
            } else {
                input.level_song
            });
        } else if self.high == 1 && self.low == 0 {
            writes.music = Some(9);
        }
        writes
    }
}

/// `SUB 1` / `SBC 0`, followed by subtraction-mode `DAA`.
/// N stays set; H from the binary subtraction selects a `$06` correction,
/// and C selects `$60` and remains the outgoing borrow after DAA. Using those
/// flags, rather than decimal conversion, also preserves invalid-BCD bytes.
fn subtract_bcd(value: u8, borrow: bool) -> (u8, bool) {
    let operand = u8::from(borrow);
    let carry = value < operand;
    let half_carry = (value & 0x0F) < operand;
    let mut result = value.wrapping_sub(operand);
    if half_carry {
        result = result.wrapping_sub(6);
    }
    if carry {
        result = result.wrapping_sub(0x60);
    }
    (result, carry)
}

impl crate::player_sim::PlayerSim {
    /// Frame-head `$05E4` call to `$5D6A`; apply only the countdown's writes.
    pub fn update_level_timer(&mut self) {
        let writes = self.level_timer.step(TimerInput {
            dead: self.dead,
            boss_clear: self.boss_clear,
            goal_bell: u8::from(self.goal_bell),
            current_song: self.current_song,
            level_song: self.level_song,
            star_high: self.star_high,
        });
        if let Some(flag) = writes.low_time_flag {
            self.low_time_flag = flag;
        }
        if let Some(dead) = writes.dead {
            self.dead = dead;
        }
        if let Some(sfx) = writes.sfx0 {
            self.sfx0 = sfx;
        }
        if let Some(sfx) = writes.sfx1 {
            self.sfx1 = sfx;
        }
        if let Some(music) = writes.music {
            self.music_request = music;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(high: u8, low: u8) -> LevelTimer {
        LevelTimer {
            high,
            low,
            tick_counter: 1,
        }
    }

    #[test]
    fn handler_0x05d6a_player_timeout_preserves_unwritten_audio_bytes() {
        let mut sim = crate::player_sim::PlayerSim::new();
        sim.level_timer = tick(0, 0);
        sim.low_time_flag = 1;
        sim.sfx0 = 0xA5;
        sim.sfx1 = 0x5A;
        sim.music_request = 0x33;
        sim.update_level_timer();
        assert_eq!(
            (sim.dead, sim.sfx0, sim.sfx1, sim.music_request),
            (0x80, 0x0F, 0x5A, 0x33)
        );
        assert_eq!(sim.level_timer, LevelTimer::new(0));
        assert_eq!(sim.low_time_flag, 0);
        let before = sim.level_timer;
        sim.update_level_timer();
        assert_eq!(sim.level_timer, before);
    }

    #[test]
    fn handler_0x05d6a_player_stop_gates_and_music_input() {
        let mut sim = crate::player_sim::PlayerSim::new();
        sim.level_timer = tick(1, 0);
        sim.level_song = 0x22;
        sim.star_high = 1;
        sim.sfx0 = 0xA5;
        sim.sfx1 = 0x5A;
        sim.music_request = 0x33;
        sim.low_time_flag = 0xA5;
        sim.boss_clear = true;
        let before = sim.level_timer;
        sim.update_level_timer();
        assert_eq!(sim.level_timer, before);
        assert_eq!(sim.low_time_flag, 0xA5);
        sim.boss_clear = false;
        sim.goal_bell = true;
        sim.update_level_timer();
        assert_eq!(sim.level_timer, before);
        sim.goal_bell = false;
        sim.current_song = 1;
        sim.update_level_timer();
        assert_eq!(
            (sim.level_timer.high, sim.level_timer.low, sim.music_request),
            (0, 0x99, 0x33)
        );
        sim.level_timer = tick(1, 0);
        sim.current_song = 0;
        sim.update_level_timer();
        assert_eq!((sim.sfx0, sim.sfx1, sim.music_request), (0xA5, 0x5A, 0x26));
        assert_eq!(sim.low_time_flag, 0xA5);
        sim.level_timer = LevelTimer::new(3);
        assert_eq!(sim.low_time_flag, 0xA5);
    }

    #[test]
    fn handler_0x05d6a_warning_and_goal_pickup_share_low_time_byte() {
        let mut sim = crate::player_sim::PlayerSim::new();
        sim.x = 100;
        sim.y = 200;
        sim.level_timer = tick(0, 0x11);
        sim.update_level_timer();
        assert_eq!(sim.low_time_flag, 1);
        sim.pending_item = 6;
        let layout = crate::level::LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
        sim.step(
            &crate::player_sim::PlayerKeys::default(),
            false,
            false,
            &layout,
        );
        assert!(sim.goal_bell);
        assert_eq!(sim.low_time_flag, 0);
        let before = sim.level_timer;
        sim.update_level_timer();
        assert_eq!(sim.level_timer, before);
        assert_eq!(sim.low_time_flag, 0);
    }

    #[test]
    fn subtraction_daa_matches_every_valid_bcd_byte_and_preserves_invalid_flags() {
        for decimal in 0..100u8 {
            let bcd = (decimal / 10) * 16 + decimal % 10;
            assert_eq!(subtract_bcd(bcd, false), (bcd, false));
            let previous = if decimal == 0 { 99 } else { decimal - 1 };
            assert_eq!(
                subtract_bcd(bcd, true),
                ((previous / 10) * 16 + previous % 10, decimal == 0)
            );
        }
        for (value, expected) in [(0x0A, 0x09), (0xA0, 0x99), (0xFA, 0xF9), (0xFF, 0xFE)] {
            assert_eq!(subtract_bcd(value, true), (expected, false));
            assert_eq!(subtract_bcd(value, false), (value, false));
        }
        assert_eq!(subtract_bcd(0, true), (0x99, true));
    }

    #[test]
    fn handler_0x05d6a_initialization_ticks_and_stop_gates() {
        assert_eq!(
            LevelTimer::new(0x12),
            LevelTimer {
                high: 0x12,
                low: 0,
                tick_counter: 48,
            }
        );
        for (counter, expected_counter, expected_low) in
            [(0, 255, 0x24), (1, 48, 0x23), (2, 1, 0x24)]
        {
            let mut timer = LevelTimer {
                tick_counter: counter,
                ..tick(2, 0x24)
            };
            assert_eq!(timer.step(TimerInput::default()), TimerWrites::default());
            assert_eq!(
                (timer.high, timer.low, timer.tick_counter),
                (2, expected_low, expected_counter)
            );
        }
        for input in [
            TimerInput {
                dead: 0x80,
                ..TimerInput::default()
            },
            TimerInput {
                dead: 1,
                ..TimerInput::default()
            },
            TimerInput {
                boss_clear: true,
                ..TimerInput::default()
            },
            TimerInput {
                goal_bell: 0xFF,
                ..TimerInput::default()
            },
        ] {
            let mut timer = tick(1, 1);
            let before = timer;
            assert_eq!(timer.step(input), TimerWrites::default());
            assert_eq!(timer, before);
        }
    }

    #[test]
    fn handler_0x05d6a_bcd_rollover_music_and_timeout() {
        let input = TimerInput {
            current_song: 0x22,
            level_song: 0x21,
            star_high: 1,
            ..TimerInput::default()
        };
        let mut timer = tick(1, 1);
        assert_eq!(
            timer.step(input),
            TimerWrites {
                music: Some(9),
                ..TimerWrites::default()
            }
        );
        assert_eq!((timer.high, timer.low), (1, 0));
        timer.tick_counter = 1;
        assert_eq!(timer.step(input), TimerWrites::default());
        assert_eq!((timer.high, timer.low), (0, 0x99));
        timer = tick(0, 0);
        assert_eq!(
            timer.step(input),
            TimerWrites {
                dead: Some(0x80),
                sfx0: Some(0x0F),
                low_time_flag: Some(0),
                ..TimerWrites::default()
            }
        );
        assert_eq!(
            timer,
            LevelTimer {
                high: 0,
                low: 0,
                tick_counter: 48,
            }
        );
    }

    #[test]
    fn handler_0x05d6a_low_time_threshold_and_music_order() {
        for (before, after, flag, sound) in [
            (0x12, 0x11, None, None),
            (0x11, 0x10, Some(1), Some(0x0E)),
            (0x01, 0x00, Some(1), Some(0x0E)),
        ] {
            let mut timer = tick(0, before);
            let writes = timer.step(TimerInput {
                current_song: 1,
                ..TimerInput::default()
            });
            assert_eq!((timer.low, writes.low_time_flag), (after, flag));
            assert_eq!(
                writes,
                TimerWrites {
                    sfx1: sound,
                    low_time_flag: flag,
                    ..TimerWrites::default()
                }
            );
        }
        for (song, star, expected) in [
            (0, 0, 0),
            (0x1F, 0, 0x1F),
            (0x1F, 1, 4),
            (0x20, 1, 0x24),
            (0x21, 1, 0x25),
            (0x22, 1, 0x26),
            (0x23, 1, 0x27),
            (0x24, 1, 4),
        ] {
            let mut timer = tick(1, 0);
            let writes = timer.step(TimerInput {
                level_song: song,
                star_high: star,
                ..TimerInput::default()
            });
            assert_eq!((timer.high, timer.low), (0, 0x99));
            assert_eq!(
                writes,
                TimerWrites {
                    music: Some(expected),
                    ..TimerWrites::default()
                }
            );
        }
    }
}
