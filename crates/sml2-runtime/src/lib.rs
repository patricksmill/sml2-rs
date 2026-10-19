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
mod divider_clock;
mod game;
mod image_output;
mod play;
mod replay;
mod viewer;

pub use cli::run_cli;
pub use session::{FrameEffects, FrameInput, Session};

/// Native GB screen; the HUD strip and parity checks are anchored to it.
pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

#[cfg(test)]
use content::load_map_nodes;
#[cfg(test)]
use game::*;
#[cfg(test)]
use play::{run_play, PlayOutcome};
#[cfg(test)]
use play_tape::PlayTape;
#[cfg(test)]
use render::{render_view, SpriteCtx};
#[cfg(test)]
use sml2_core::{
    ending_entry::{EndingEntry, EndingEntryState, EndingFrame, EndingInput},
    level::{HudStrip, LevelLayout, LevelManifest},
    map::MapNodes,
    object::ObjectTables,
    player_sim::{PlayerKeys, PlayerSim},
    scene::{boss_def_index, boss_def_portable, SceneScripts, SceneStep, SceneVm},
    video,
};
#[cfg(test)]
use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(test)]
mod circuit_boss_entry_tests;
#[cfg(test)]
mod door_boss_entry_tests;
#[cfg(test)]
mod patrol_boss_entry_tests;
#[cfg(test)]
mod played_route_tests;
#[cfg(test)]
mod scene_pipe_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
mod sweep_boss_entry_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tree_boss_entry_tests;
