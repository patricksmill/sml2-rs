//! `$19` owns loading before the cursor's `$1A` dispatch.
//!
//! `$0348` waits for LY=$91 and disables the LCD; `$2B8B` copies $1800
//! graphics bytes, then `$3000C` copies $400 tilemap bytes. The LCD is enabled
//! with $83 only after slot validation and cursor initialization.
//!
//! The native loading budget is a proven minimum of source work, quantized
//! against the Game Boy's 70,224 T-cycle frame. CopyMem costs 52 T/byte, minus
//! 4 for the final branch plus 16 for RET, on each of five calls. The tilemap
//! loop costs 48 T/byte, minus 4 for its final branch. This is elapsed work,
//! not CPU emulation or a movie-derived delay.
//!
//! Timing remains partial: this minimum excludes setup, slot-validation paths,
//! the initial LY=$91 wait, and the LCD restart-to-VBlank wait. Native updates
//! have no shared LCD phase clock, so this does not establish exact hardware
//! sample alignment. Those waits must not be guessed from a recording.
//!
//! `$0264` samples input before `$19`, then `$1A` samples again after loading.
//! Loading work never polls input; it must not remember transient busy input.

use super::{pack_joypad, Session, SessionState};
use sml2_core::player_sim::PlayerKeys;

const GRAPHICS_COPY_CYCLES: u32 = 0x1800 * 52 + 5 * (16 - 4);
const TILEMAP_COPY_CYCLES: u32 = 0x400 * 48 - 4;
const MINIMUM_LOADING_CYCLES: u32 = GRAPHICS_COPY_CYCLES + TILEMAP_COPY_CYCLES;
const FRAME_CYCLES: u32 = 456 * 154;

/// Native elapsed loading work. See module documentation for timing limits.
pub struct FileSelectLoading {
    remaining_cycles: u32,
    started: bool,
}

impl Default for FileSelectLoading {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSelectLoading {
    pub fn new() -> Self {
        Self {
            remaining_cycles: MINIMUM_LOADING_CYCLES,
            started: false,
        }
    }

    /// Pending `$19` still presents the title; active transfers have LCD off.
    pub fn lcd_disabled(&self) -> bool {
        self.started
    }

    fn advance(&mut self) -> bool {
        self.started = true;
        self.remaining_cycles = self.remaining_cycles.saturating_sub(FRAME_CYCLES);
        self.remaining_cycles == 0
    }
}

impl Session {
    pub(super) fn advance_file_select_loading(&mut self, keys: PlayerKeys) {
        let SessionState::FileSelectLoading(loading) = &mut self.state else {
            return;
        };
        let sample_input = !loading.started;
        let complete = loading.advance();
        if sample_input {
            self.prev_keys_held = pack_joypad(
                keys.right,
                keys.left,
                keys.up,
                keys.down,
                keys.a,
                keys.b,
                keys.select,
                keys.start,
            );
        }
        if complete {
            // `$30000` seeds and advances dispatch, without calling `$3005F`.
            self.enter_file_select();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{FrameInput, SessionAssets};
    use sml2_core::front_end::keys::SELECT;
    use sml2_core::front_end::save_slots::fresh_template;

    #[test]
    fn source_copy_budget_has_an_explicit_completion_boundary() {
        assert_eq!(GRAPHICS_COPY_CYCLES, 319_548);
        assert_eq!(TILEMAP_COPY_CYCLES, 49_148);
        let mut loading = FileSelectLoading::new();
        assert!(!loading.lcd_disabled());
        for tick in 1..=5 {
            assert!(!loading.advance(), "source work remains after tick {tick}");
            assert!(loading.lcd_disabled());
            assert_eq!(
                loading.remaining_cycles,
                MINIMUM_LOADING_CYCLES - tick * FRAME_CYCLES
            );
        }
        assert!(loading.advance());
        assert_eq!(loading.remaining_cycles, 0);
    }

    #[test]
    fn loading_samples_keys_once_then_seeds_without_updating_cursor() {
        let mut session = Session::new(SessionAssets::load().unwrap());
        let corrupt = [0xA5; 0x50];
        session.slots.write_slot(0, &corrupt);
        session.state = SessionState::FileSelectLoading(FileSelectLoading::new());
        let held = PlayerKeys {
            select: true,
            ..PlayerKeys::default()
        };
        session.advance(FrameInput { keys: held, div: 0 });
        assert_eq!(session.prev_keys_held, SELECT, "ReadJoypad precedes $19");
        for _ in 1..5 {
            assert!(matches!(session.state, SessionState::FileSelectLoading(_)));
            assert_eq!(session.slots.slot(0), Some(&corrupt));
            assert!(session
                .render_frame((160, 144))
                .iter()
                .all(|&pixel| pixel == 255));
            session.advance(FrameInput {
                keys: PlayerKeys::default(),
                div: 0,
            });
            assert_eq!(
                session.prev_keys_held, SELECT,
                "busy work does not poll input"
            );
        }
        session.advance(FrameInput {
            keys: PlayerKeys::default(),
            div: 0,
        });
        assert_eq!(session.slots.slot(0), Some(&fresh_template()));
        let SessionState::FileSelect(fs) = &session.state else {
            panic!("loader completes to $1A")
        };
        assert_eq!((fs.cursor(), fs.menu_state()), ((40, 16), 0));
        assert_eq!(session.sound_driver.active_song, 13);
        session.advance(FrameInput { keys: held, div: 0 });
        let SessionState::FileSelect(fs) = &session.state else {
            panic!("cursor drops")
        };
        assert_eq!(fs.cursor(), (40, 18));
        assert_eq!(fs.cursor_sprite(), 16);
        assert!(fs.cursor_behind_bg());
        assert!(!fs.easy_mode(), "loading input is not a menu action");
    }
}
