//! Runtime behavior tests, grouped by the subsystem or route they exercise.

use crate::{
    content::load_map_nodes,
    game::*,
    play::{run_play, PlayOutcome},
    play_tape::PlayTape,
    render::{render_view, SpriteCtx},
    SCREEN_H, SCREEN_W,
};
use sml2_core::{
    ending_entry::{EndingEntry, EndingEntryState, EndingFrame, EndingInput},
    level::{HudStrip, LevelLayout, LevelManifest},
    map::MapNodes,
    object::ObjectTables,
    player_sim::{PlayerKeys, PlayerSim},
    scene::{boss_def_index, boss_def_portable, SceneScripts, SceneStep, SceneVm},
    video,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

mod boss_entry;
mod front_end;
mod game;
mod played_routes;
mod scene_pipe;
mod session;
