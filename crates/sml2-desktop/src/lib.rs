//! Desktop policies outside the parity simulation: settings, input and timing.

pub mod dx_import;
pub mod input;
pub mod library;
pub mod settings;
pub mod timing;
pub mod vanilla_import;

#[cfg(feature = "native")]
pub mod app;
#[cfg(feature = "native")]
pub mod controllers;
#[cfg(feature = "native")]
pub mod game;
#[cfg(feature = "native")]
mod menu;

#[cfg(feature = "native")]
pub mod ui;
