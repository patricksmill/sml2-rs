//! Optional scenery authoring companion; the game never depends on this crate.

pub use sml2_desktop::{dx_import, input, settings, vanilla_import};

#[cfg(feature = "native")]
pub mod app;
