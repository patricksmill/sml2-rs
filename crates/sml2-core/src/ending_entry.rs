//! Wario's scripted exit ($5E12), fade ($03AF), and ending initializer ($2CAE).
//! The state-19 sequence body is a separate, pending subsystem.
use crate::player_sim::PlayerKeys;

#[derive(Debug, Clone, Copy, Default)]
pub struct EndingInput {
    pub held: PlayerKeys,
    pub pressed: PlayerKeys,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum EndingEntryState {
    #[default]
    Gameplay = 4,
    Fade = 35,
    Initialize = 18,
    Sequence = 19,
}

/// Native palette register outputs, retained for the renderer/sequence owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndingPalettes {
    pub background: u8,
    pub object0: u8,
    pub object1: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum EndingFrame {
    Gameplay(EndingInput),
    Fade(EndingPalettes),
    SequenceInitialized,
    /// Explicit downstream dispatch; no state-19 sequence body is implemented here.
    SequencePending,
}

#[derive(Debug, Default)]
pub struct EndingEntry {
    state: EndingEntryState,
    ending_state: u8,
    palettes: Option<EndingPalettes>,
}

impl EndingEntry {
    pub fn state(&self) -> EndingEntryState {
        self.state
    }

    pub fn ending_state(&self) -> u8 {
        self.ending_state
    }

    pub fn palettes(&self) -> Option<EndingPalettes> {
        self.palettes
    }

    /// Dispatch the state present at frame entry. A requested state never
    /// replaces the current frame's gameplay or executes its new body early.
    /// The caller owns the once-per-frame VBlank fade tick, before dispatch.
    pub fn frame_head(
        &mut self,
        boss_phase: &mut u8,
        x: u16,
        y: u16,
        fade: &mut u16,
        input: EndingInput,
    ) -> EndingFrame {
        match self.state {
            EndingEntryState::Gameplay => {
                if *boss_phase != 3 {
                    return EndingFrame::Gameplay(input);
                }
                let mut scripted = EndingInput::default();
                scripted.held.right = true;
                match (x as u8) & 0xFE {
                    0..=159 => scripted.held.b = true,
                    160 => {
                        scripted.held.a = true;
                        scripted.pressed.a = true;
                    }
                    _ => {
                        scripted.held.a = true;
                        if y as u8 >= 224 {
                            self.state = EndingEntryState::Fade;
                        }
                    }
                }
                EndingFrame::Gameplay(scripted)
            }
            EndingEntryState::Fade => {
                // ROM $03E1: four padded records, selected by low & $C0.
                const PALETTES: [[u8; 3]; 4] = [
                    [0x00, 0x00, 0x00],
                    [0x50, 0x40, 0x14],
                    [0x94, 0x90, 0x24],
                    [0xE4, 0xD0, 0x38],
                ];
                let [background, object0, object1] = PALETTES[((*fade as u8) >> 6) as usize];
                let palettes = EndingPalettes {
                    background,
                    object0,
                    object1,
                };
                self.palettes = Some(palettes);
                if *fade == 0 {
                    *fade = 255;
                    self.state = EndingEntryState::Initialize;
                }
                EndingFrame::Fade(palettes)
            }
            EndingEntryState::Initialize => {
                self.ending_state = 0;
                *boss_phase = 0;
                self.state = EndingEntryState::Sequence;
                EndingFrame::SequenceInitialized
            }
            EndingEntryState::Sequence => EndingFrame::SequencePending,
        }
    }
}

#[cfg(test)]
mod tests;
