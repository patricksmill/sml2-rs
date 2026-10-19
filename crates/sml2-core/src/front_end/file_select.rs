//! `$19`/`$1A` and the seven `sMenuState` handlers: the file select.
//!
//! The screen owns the cursor animation, the Easy and erase toggles, and the
//! confirm animation. Slot storage and validation stay in [`SaveSlots`]; a
//! commit is reported as a typed action so the runtime can enter the overworld
//! or apply the erase result.

use super::keys::{A, DOWN, LEFT, RIGHT, SELECT, START};
use super::save_slots::{fresh_template, SaveSlots, SLOT_COUNT, SLOT_LEN};

/// `$30000`: the file select requests song 13.
pub const MUSIC: u8 = 13;
/// `$30000`: cursor seed (`$FFC2`).
pub const CURSOR_START_X: u8 = 40;
/// `$30000`: cursor seed (`$FFC0`).
pub const CURSOR_START_Y: u8 = 16;
/// The row the cursor settles on (`$7C`).
pub const SLOT_ROW_Y: u8 = 0x7C;

/// The state-6 erase animation's four sprite ids (`$41C2`).
const ERASE_FRAMES: [u8; 4] = [0x10, 0x00, 0x11, 0x04];

/// What a commit asked the host to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileSelectAction {
    #[default]
    None,
    /// `$30227` load path: the slot's record is ready to enter the map.
    Load { slot: usize, record: [u8; SLOT_LEN] },
    /// `$30227` erase path: the slot now holds the fresh template.
    Erased { slot: usize },
}

/// Sound requests produced by this frame (0 = none).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileSelectOutput {
    pub action: FileSelectAction,
    pub sfx0: u8,
    pub sfx2: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSelect {
    menu_state: u8,
    timer: u8,
    cursor_x: u8,
    cursor_y: u8,
    sprite: u8,
    cursor_behind_bg: bool,
    cursor_hop_active: bool,
    cursor_anim_y: u8,
    erase_hop_y: u8,
    erase_hop_active: bool,
    clear_mode: bool,
    easy_mode: bool,
    walk_timer: u8,
    mario_dir: u8,
    frame: u32,
    /// `sSaveFileIndex` (`$A2A3`), set at the confirm commit.
    pub save_file_index: u8,
    sfx0: u8,
    sfx2: u8,
    music: u8,
}

impl FileSelect {
    /// `$30000`: validate all slots, seed the cursor, request SFX 0 = 5 and
    /// song 13 (`take_music`).
    pub fn new(slots: &mut SaveSlots) -> Self {
        slots.validate_and_init();
        Self {
            menu_state: 0,
            timer: 0,
            cursor_x: CURSOR_START_X,
            cursor_y: CURSOR_START_Y,
            sprite: 16,
            cursor_behind_bg: false,
            cursor_hop_active: false,
            cursor_anim_y: 0,
            erase_hop_y: 0,
            erase_hop_active: false,
            clear_mode: false,
            easy_mode: false,
            walk_timer: 0,
            mario_dir: 1,
            frame: 0,
            save_file_index: 0,
            sfx0: 5,
            sfx2: 0,
            music: MUSIC,
        }
    }

    /// The one-shot loader music request.
    pub fn take_music(&mut self) -> u8 {
        std::mem::take(&mut self.music)
    }

    pub fn menu_state(&self) -> u8 {
        self.menu_state
    }

    pub fn timer(&self) -> u8 {
        self.timer
    }

    pub fn cursor(&self) -> (u8, u8) {
        (self.cursor_x, self.cursor_y)
    }

    pub fn cursor_sprite(&self) -> u8 {
        self.sprite
    }

    pub fn cursor_behind_bg(&self) -> bool {
        self.cursor_behind_bg
    }

    pub fn cursor_anim_y(&self) -> u8 {
        self.cursor_anim_y
    }

    /// The erase animation draws a second Mario sprite at `(x, hop_y)` while
    /// the timer is in its upper half (`$30171`).
    pub fn erase_hop(&self) -> Option<u8> {
        self.erase_hop_active.then_some(self.erase_hop_y)
    }

    pub fn clear_mode(&self) -> bool {
        self.clear_mode
    }

    pub fn easy_mode(&self) -> bool {
        self.easy_mode
    }

    pub fn walk_timer(&self) -> u8 {
        self.walk_timer
    }

    /// One frame: the `$3005F` loop clears the hop latch, runs the menu-state
    /// handler, then applies the erase-mode SFX2 tick.
    pub fn step(&mut self, slots: &mut SaveSlots, held: u8, pressed: u8) -> FileSelectOutput {
        self.frame = self.frame.wrapping_add(1);
        self.cursor_hop_active = false;
        let action = match self.menu_state {
            0 => {
                self.state_drop();
                FileSelectAction::None
            }
            1 => self.state_input(held, pressed),
            2 => {
                self.state_right();
                FileSelectAction::None
            }
            3 => {
                self.state_left();
                FileSelectAction::None
            }
            4 => {
                self.state_erase_toggle();
                FileSelectAction::None
            }
            5 => self.state_confirm(slots),
            6 => {
                self.state_erase_animation();
                FileSelectAction::None
            }
            _ => FileSelectAction::None,
        };

        // `$3009B`: while erase mode is armed (and not in its own toggle
        // state), every even frame ticks SFX 2 = 4.
        if self.clear_mode && self.menu_state != 4 && self.frame & 1 == 0 {
            self.sfx2 = 4;
        }

        FileSelectOutput {
            action,
            sfx0: std::mem::take(&mut self.sfx0),
            sfx2: std::mem::take(&mut self.sfx2),
        }
    }

    /// `$43D6`: the cursor drops from `$10` to the `$7C` row.
    fn state_drop(&mut self) {
        self.sprite = 16;
        self.cursor_behind_bg = false;
        if self.cursor_y >= 0x30 {
            self.sprite = 10;
            self.cursor_behind_bg = true;
        }
        if self.cursor_y >= SLOT_ROW_Y {
            self.menu_state = 1;
            self.mario_dir = 1;
            self.cursor_y = SLOT_ROW_Y;
            return;
        }
        self.cursor_y = self.cursor_y.wrapping_add(2);
        if self.cursor_y >= 0x40 {
            self.cursor_y = self.cursor_y.wrapping_add(1);
        }
    }

    /// `$4326`: the slot row's input.
    fn state_input(&mut self, held: u8, pressed: u8) -> FileSelectAction {
        // The ROM compares `hKeysPressed` with `cp`, so a multi-button press
        // (e.g. START+A) matches nothing.
        if pressed == SELECT {
            if !self.clear_mode {
                self.easy_mode = !self.easy_mode;
                self.sprite = self.standing_sprite();
            }
            return FileSelectAction::None;
        }

        match held & 0x30 {
            RIGHT => {
                if self.cursor_x < 0x88 {
                    self.menu_state = 2;
                    self.timer = 32;
                    self.sprite = ((self.walk_timer & 0x0C) >> 2).wrapping_add(1);
                }
            }
            LEFT => {
                if self.cursor_x >= 0x30 {
                    self.menu_state = 3;
                    self.timer = 32;
                    self.sprite = ((self.walk_timer & 0x0C) >> 2).wrapping_add(5);
                }
            }
            _ => {}
        }
        if self.menu_state != 1 {
            return FileSelectAction::None;
        }

        if pressed == DOWN || pressed == START {
            if self.cursor_x < 0x80 {
                self.menu_state = 5;
                self.timer = 64;
                self.sfx0 = 5;
                return FileSelectAction::None;
            }
        } else if pressed == A && self.cursor_x == 0x88 {
            self.menu_state = 4;
            self.timer = 55;
            self.sfx0 = 1;
            return FileSelectAction::None;
        }

        self.sprite = self.standing_sprite();
        FileSelectAction::None
    }

    /// `$42F1`: cursor right one slot, 32 frames.
    fn state_right(&mut self) {
        self.mario_dir = 1;
        self.tick_walk();
        self.sprite = ((self.walk_timer & 0x0C) >> 2).wrapping_add(1);
        self.cursor_x = self.cursor_x.wrapping_add(1);
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.menu_state = 1;
        }
    }

    /// `$42C8`: cursor left one slot, 32 frames.
    fn state_left(&mut self) {
        self.mario_dir = 0xFF;
        self.tick_walk();
        self.sprite = ((self.walk_timer & 0x0C) >> 2).wrapping_add(5);
        self.cursor_x = self.cursor_x.wrapping_sub(1);
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.menu_state = 1;
        }
    }

    /// `$426D`: the erase-mode toggle hop.
    fn state_erase_toggle(&mut self) {
        self.sprite = 9;
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0x2B {
            self.clear_mode = !self.clear_mode;
            self.mario_dir = 0xFF;
            self.sfx0 = 8;
            if self.clear_mode {
                self.easy_mode = false;
            }
        }
        if self.timer >= 0x2B {
            self.cursor_y = self.cursor_y.wrapping_sub(2);
            return;
        }
        if self.timer >= 0x0B {
            self.cursor_hop_active = true;
            return;
        }
        self.cursor_y = self.cursor_y.wrapping_add(2);
        if self.timer == 0 {
            self.menu_state = 1;
            self.cursor_y = SLOT_ROW_Y;
        }
    }

    /// `$41C6`: the confirm drop, then the load/erase commit.
    fn state_confirm(&mut self, slots: &mut SaveSlots) -> FileSelectAction {
        self.sprite = 16;
        self.cursor_y = self.cursor_y.wrapping_add(1);
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return FileSelectAction::None;
        }

        // `$301E0`: `(($FFC2 & $60) >> 5) - 1`.
        let index = ((self.cursor_x & 0x60) >> 5).wrapping_sub(1);
        self.save_file_index = index;
        if index as usize >= SLOT_COUNT {
            return FileSelectAction::None;
        }
        let slot = index as usize;

        if self.clear_mode {
            slots.erase_slot(slot);
            self.timer = 0x80;
            self.menu_state = 6;
            self.clear_mode = false;
            self.cursor_anim_y = 144;
            self.sfx2 = 5;
            FileSelectAction::Erased { slot }
        } else {
            let record = slots.load_slot(slot).unwrap_or_else(fresh_template);
            FileSelectAction::Load { slot, record }
        }
    }

    /// `$4140`: the post-erase animation, then back to the slot row.
    fn state_erase_animation(&mut self) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.finish_erase();
            return;
        }

        // `$30159`: the hop sprite id and position.
        self.sprite = (((self.timer >> 1) & 0xF0) >> 4).wrapping_add(0xD3);
        self.cursor_behind_bg = false;
        if self.timer >= 0x40 {
            self.erase_hop_y = (self.timer >> 2).wrapping_add(0x70);
            self.erase_hop_active = true;
        } else {
            self.erase_hop_active = false;
        }

        // `$3017C`: the erase row slides back.
        let step = ((self.timer & 0xF0) >> 4).wrapping_sub(4);
        self.cursor_anim_y = self.cursor_anim_y.wrapping_sub(step);
        if self.timer < 0x40 && self.cursor_anim_y >= SLOT_ROW_Y {
            self.cursor_anim_y = SLOT_ROW_Y;
            self.sprite = 16;
            self.finish_erase();
            return;
        }
        self.cursor_y = self.cursor_anim_y;
        let index = (((self.timer & 0x18) << 1) >> 4) as usize;
        self.sprite = ERASE_FRAMES[index];
    }

    fn finish_erase(&mut self) {
        self.cursor_y = SLOT_ROW_Y;
        self.menu_state = 1;
        self.clear_mode = false;
        self.cursor_behind_bg = false;
        self.erase_hop_active = false;
    }

    /// `$440D`: the shared walk-animation timer, +1 per call, wrap `$0C`.
    fn tick_walk(&mut self) {
        self.walk_timer = self.walk_timer.wrapping_add(1);
        if self.walk_timer >= 0x0C {
            self.walk_timer = 0;
        }
    }

    /// `$431A`: right = 0, left = 4.
    fn standing_sprite(&self) -> u8 {
        ((self.mario_dir ^ 0xFF).wrapping_add(2)) << 1
    }
}

#[cfg(test)]
mod tests;
