//! Original boot flow: title screen, file select, and save slots.
//!
//! The simulation lives here so the runtime only glues inputs, sound requests
//! and presentation; the ROM state handlers are the authority for every byte.
//! See `docs/plans/2026-10-19-menus-save-flow.md`.

pub mod save_slots;

pub use save_slots::{SaveSlots, SLOT_COUNT, SLOT_LEN};
