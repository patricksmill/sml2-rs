//! Joypad bits as the ROM's `hKeysHeld`/`hKeysPressed` carry them.
//!
//! The port's runtime packs `PlayerKeys` into the same byte (`pack_joypad`).

pub const A: u8 = 0x01;
pub const B: u8 = 0x02;
pub const SELECT: u8 = 0x04;
pub const START: u8 = 0x08;
pub const RIGHT: u8 = 0x10;
pub const LEFT: u8 = 0x20;
pub const UP: u8 = 0x40;
pub const DOWN: u8 = 0x80;
