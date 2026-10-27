//! Blocking bank-15 door presentation, with gameplay input suspended.

use sml2_core::map_sim::MapSim;

/// The screen whose walk-in `$3E3BA` hands to `$3E4C1` (`cp $07`): the
/// `$3D6EB` loader's screen, which draws Wario's castle (`$3BD80`).
const FIXED_PALETTE_SCREEN: u8 = 7;

#[derive(Debug)]
pub struct MapScreenTransition {
    pub frame: u16,
    pub source: MapSim,
    pub entering: bool,
    direction: u8,
    pub(crate) fade_frames: u16,
}

impl MapScreenTransition {
    pub fn new(source: MapSim, direction: u8) -> Self {
        Self::new_dx(source, direction, false)
    }

    pub(crate) fn new_dx(source: MapSim, direction: u8, dx: bool) -> Self {
        Self {
            frame: 0,
            entering: source.screen_id == 0,
            source,
            direction,
            fade_frames: if dx { 36 } else { 48 },
        }
    }

    pub fn shows_source(&self) -> bool {
        !self.entering || self.frame < self.fade_frames
    }

    /// `(sBGPalette, sOAMPalette1, sOAMPalette2)` for this frame of the walk.
    /// `destination` is the screen the door loads.
    pub fn palettes(&self, destination: u8) -> (u8, u8, u8) {
        const OBJ: [u8; 5] = [0, 0, 0x40, 0x80, 0xD0];
        if self.entering && self.frame >= self.fade_frames && destination == FIXED_PALETTE_SCREEN {
            // `$3E4C1`: screen 7 skips the fade-in and holds its own palettes.
            return (0xE1, 0xD0, 0x39);
        }
        if self.entering && self.frame < self.fade_frames {
            let step = self.frame / 12;
            return (
                (0xE4u16 << (step * 2)) as u8,
                OBJ[4 - step as usize],
                OBJ[4 - step as usize],
            );
        }
        if self.entering {
            let step = ((self.frame - self.fade_frames) / 5 + 1).min(4) as usize;
            const BG: [u8; 5] = [0, 0, 0x40, 0x90, 0xE4];
            (BG[step], OBJ[step], OBJ[step])
        } else {
            let step = self.frame.saturating_sub(80) / 5;
            (
                (0xE4u16 << (step * 2)) as u8,
                OBJ[4 - step as usize],
                OBJ[4 - step as usize],
            )
        }
    }

    pub fn advance(&mut self, destination: &mut MapSim, div: u8) -> bool {
        let frame = self.frame;
        self.frame += 1;
        if self.entering && frame < self.fade_frames {
            return false;
        }
        // `$3E4C1`: screen 7's walk-in is 48 steps at two frames each, no fade.
        let fixed = self.entering && destination.screen_id == FIXED_PALETTE_SCREEN;
        let walk_frame = if self.entering {
            frame - self.fade_frames
        } else {
            frame
        };
        let map = if self.entering {
            &mut *destination
        } else {
            &mut self.source
        };
        map.bookkeeping(div);
        let moving = if fixed {
            walk_frame % 2 == 0
        } else if self.entering && walk_frame < 20 {
            matches!(walk_frame % 5, 1 | 3)
        } else if !self.entering && walk_frame >= 80 {
            matches!((walk_frame - 80) % 5, 1 | 3)
        } else {
            walk_frame % 2 == 0
        };
        if moving {
            map.transition_walk_step(self.direction);
        }
        let done = walk_frame + 1 >= if fixed { 96 } else { 100 };
        if done && !self.entering {
            // The walk out ran `$3C13E` on the screen it left; the hub reload
            // keeps those bytes.
            destination.set_map_ram(self.source.map_ram());
        }
        done
    }
}

#[cfg(test)]
#[path = "tests/map_transition.rs"]
mod tests;
