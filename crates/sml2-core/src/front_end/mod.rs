//! Original boot flow: title screen, file select, and save slots.
//!
//! The simulation lives here so the runtime only glues inputs, sound requests
//! and presentation; the ROM state handlers are the authority for every byte.
//! Session and save ownership: `DESIGN.md`, progression ownership section.

pub mod assets;
pub mod demo;
pub mod file_select;
pub mod keys;
pub mod render;
pub mod save_slots;
pub mod screen;
pub mod title;

pub use assets::{DEMO_TAPE_LEN, DEMO_TAPES_LEN, FrontEndAssets, TILEMAP_LEN, VRAM_LEN};
pub use demo::{DEMO_LEVELS, DemoTape};
pub use file_select::{FileSelect, FileSelectAction, FileSelectOutput};
pub use render::{render_file_select, render_title};
pub use save_slots::{SAVE_IMAGE_LEN, SLOT_COUNT, SLOT_LEN, SaveSlots};
pub use title::{TitleAction, TitleScreen};
