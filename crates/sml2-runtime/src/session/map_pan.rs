//! Hub camera return: `$3C518` reloads through `$3F156`, then fades via `$3EA9A`.

/// The native loader needs a presentation interval for the work the ROM did with
/// the LCD disabled: four 32×32 banked map copies (~680k T-cycles), the 6 KiB
/// graphics copy (~319k), and the 32×32 viewport copy (~340k). At 70,224 cycles
/// per console frame, their fixed transfer work rounds up to 20 frames.
/// This is a bulk-load budget, not emulation of node-specific CPU timing.
pub const MAP_PAN_RELOAD_FRAMES: u16 = 20;

/// `$3EA9A`: four palette writes, each after three `$3E9EC` frame waits.
pub const MAP_PAN_RETURN_FRAMES: u16 = MAP_PAN_RELOAD_FRAMES + 12;

#[derive(Debug, Default)]
pub struct MapPanReturn {
    pub frame: u16,
}

impl MapPanReturn {
    /// `$3F03D`: a level's return to the hub runs `$3EA9A` after the load.
    /// That load is not modelled (docs/frame-clock.md), so start at the fade.
    pub(crate) fn fade_in() -> Self {
        Self {
            frame: MAP_PAN_RELOAD_FRAMES,
        }
    }

    pub(crate) fn advance(&mut self) -> bool {
        self.frame += 1;
        self.frame >= MAP_PAN_RETURN_FRAMES
    }

    pub(crate) fn palettes(&self) -> (u8, u8) {
        let step = (self.frame.saturating_sub(MAP_PAN_RELOAD_FRAMES) / 3).min(4) as usize;
        // BGP shifts E4 into the blank palette two bits at a time. OBP1 comes
        // from ROM $3EA5A + 2*e; the existing map renderer uses one OBJ palette.
        const BG: [u8; 5] = [0x00, 0x00, 0x40, 0x90, 0xE4];
        const OBJ: [u8; 5] = [0x00, 0x00, 0x40, 0x80, 0xD1];
        (BG[step], OBJ[step])
    }
}

#[cfg(test)]
#[path = "tests/map_pan.rs"]
mod tests;
