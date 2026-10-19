//! Original boot flow: title screen, file select, and save slots.
//!
//! The simulation lives here so the runtime only glues inputs, sound requests
//! and presentation; the ROM state handlers are the authority for every byte.
//! See `docs/plans/2026-10-19-menus-save-flow.md`.

pub mod assets;
pub mod file_select;
pub mod keys;
pub mod render;
pub mod save_slots;
pub mod screen;
pub mod title;

pub use assets::{FrontEndAssets, TILEMAP_LEN, VRAM_LEN};
pub use file_select::{FileSelect, FileSelectAction, FileSelectOutput};
pub use render::{render_file_select, render_title};
pub use save_slots::{SaveSlots, SAVE_IMAGE_LEN, SLOT_COUNT, SLOT_LEN};
pub use title::{TitleAction, TitleScreen};
