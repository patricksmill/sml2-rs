//! Reusable native SML2 runtime.
//!
//! `Session::advance` accepts explicit tick inputs and returns deferred effects.
//! Content loading, CLI/window hosting, and read-only rendering have separate owners.
//! Original diagnostic play and frozen replay adapters preserve their existing loops.

pub mod audio_output;
pub mod content;
pub mod host;
pub mod play_tape;
pub mod present;
pub mod render;
pub mod session;

mod cli;
pub mod divider_clock;
mod game;
mod image_output;
mod play;
mod replay;
mod viewer;

pub use cli::run_cli;
pub use session::{Boot, FrameEffects, FrameInput, Session};

/// Native GB screen; the HUD strip and parity checks are anchored to it.
pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

#[cfg(test)]
mod tests;
