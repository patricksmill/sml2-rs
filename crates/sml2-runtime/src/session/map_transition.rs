//! Blocking bank-15 door presentation, with gameplay input suspended.

use sml2_core::map_sim::MapSim;

#[derive(Debug)]
pub struct MapScreenTransition {
    pub frame: u16,
    pub source: MapSim,
    pub entering: bool,
    direction: u8,
}

impl MapScreenTransition {
    pub fn new(source: MapSim, direction: u8) -> Self {
        Self { frame: 0, entering: source.screen_id == 0, source, direction }
    }

    pub fn shows_source(&self) -> bool {
        !self.entering || self.frame < 48
    }

    pub fn palettes(&self) -> (u8, u8, u8) {
        const OBJ: [u8; 5] = [0, 0, 0x40, 0x80, 0xD0];
        if self.entering && self.frame < 48 {
            let step = self.frame / 12;
            return ((0xE4u16 << (step * 2)) as u8, OBJ[4 - step as usize], OBJ[4 - step as usize]);
        }
        if self.entering {
            let step = ((self.frame - 48) / 5 + 1).min(4) as usize;
            const BG: [u8; 5] = [0, 0, 0x40, 0x90, 0xE4];
            (BG[step], OBJ[step], OBJ[step])
        } else {
            let step = self.frame.saturating_sub(80) / 5;
            ((0xE4u16 << (step * 2)) as u8, OBJ[4 - step as usize], OBJ[4 - step as usize])
        }
    }

    pub fn advance(&mut self, destination: &mut MapSim, div: u8) -> bool {
        let frame = self.frame;
        self.frame += 1;
        if self.entering && frame < 48 { return false; }
        let turtle = self.entering && destination.screen_id == 7;
        let walk_frame = if self.entering { frame - 48 } else { frame };
        let map = if self.entering { destination } else { &mut self.source };
        map.bookkeeping(div);
        let moving = if turtle {
            walk_frame % 2 == 0
        } else if self.entering && walk_frame < 20 {
            matches!(walk_frame % 5, 1 | 3)
        } else if !self.entering && walk_frame >= 80 {
            matches!((walk_frame - 80) % 5, 1 | 3)
        } else {
            walk_frame % 2 == 0
        };
        if moving { map.transition_walk_step(self.direction); }
        walk_frame + 1 >= if turtle { 96 } else { 100 }
    }
}

#[cfg(test)]
#[path = "tests/map_transition.rs"]
mod tests;
