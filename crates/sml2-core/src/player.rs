//! Player (Mario) draw — decoded from bank-1 code in `main.asm`
//! (`$5267`-`$5520`) and verified against live OAM.
//!
//! ## Frame selection (the `$FFC6` value)
//!
//! `$5377`/`$53CF` pick a base frame from the movement state, then
//! `$5447`-`$549F` apply a powerup-dependent offset:
//!
//! | state                     | right | left |
//! |---------------------------|-------|------|
//! | standing (`hvel == $80`)  | 0     | 4    |
//! | walking (timer 0-11)      | 1-3   | 5-7  |
//! | skidding                  | 13    | 12   |
//! | ducking                   | 14    | 15   |
//! | airborne                  | 8     | 9    |
//! | grounded, not airborne    | 10    | 11   |
//!
//! `hvel` = `sHVelocityIndex` ($A200): `$80` = still, `< $80` = moving left,
//! `> $80` = moving right. `walk_timer` = `sMarioWalkAnimTimer` ($A222;
//! advances 1 per frame, 2 with B held, wraps at `$0C`) — the walk cell is
//! `(timer & $0C) >> 2` (0..2, four frames per pose).
//!
//! Powerup offsets (`sCurPowerup` $A216): `1` (Super) -> `+0`, `0` (Small)
//! -> `+32`, `2` (Bunny) -> `+69` (`+26` more while the flutter pose timer
//! has bit 2 set), `3` (Fire) -> `+112` (`+32` more while
//! `sMarioThrowTimer` runs). The invincibility flicker adds `+32` while
//! `sInvincibilityTimer` bit 2 is set. Special states decoded but not
//! modelled yet: spin-jump table `{$04,$10,$00,$11}` by
//! `($FF97 & $0C) >> 2`, pipe travel (`+16` / walk / `$5426`), moon physics
//! poses 18-21 and the moon overlay frame (`220`/`221`, drawn with `$FFBB`
//! set), camera-pan bob (`$FFC4 -= sCameraPanTimer & 3`), stomp-bounce pose.
//!
//! ## Geometry
//!
//! Common tail (`$54A7`): `$FFC5 = marioX - camX + 96`, `$FFC4 = marioY -
//! camY + 98` (8-bit arithmetic; these doubles as `sMarioScreenX/Y`), then
//! the mapping expander `$5297` writes cells `base + [yoff, xoff]` with the
//! blob's attr. On the player path `$FFBB` is set, so the expander's
//! force-priority branch is skipped (attr bit 7 stays clear); `$FFC7`
//! (1 while the star flash timer bit 2 is set) forces attr bit 4 (OBP1) for
//! the invincibility flicker.

use crate::actor::ScreenPosition;
use crate::object::{MappingEntry, ObjectTables};

/// Base frame index before the powerup offset — `main.asm` $5377/$53CF.
pub fn base_frame_index(
    dir_right: bool,
    on_ground_or_object: bool,
    in_air: bool,
    ducking: bool,
    hvel: u8,
    walk_timer: u8,
) -> u8 {
    let walk = (walk_timer & 0x0C) >> 2;
    if dir_right {
        if ducking {
            14
        } else if on_ground_or_object {
            if hvel >= 0x80 {
                if hvel == 0x80 {
                    0
                } else {
                    1 + walk
                }
            } else {
                13
            }
        } else if in_air {
            8
        } else {
            10
        }
    } else if ducking {
        15
    } else if on_ground_or_object {
        if hvel >= 0x81 {
            12
        } else if hvel == 0x80 {
            4
        } else {
            5 + walk
        }
    } else if in_air {
        9
    } else {
        11
    }
}

/// Final `$FFC6` frame for the common (non-special) states.
#[allow(clippy::too_many_arguments)]
pub fn frame_index(
    dir_right: bool,
    on_ground_or_object: bool,
    in_air: bool,
    ducking: bool,
    hvel: u8,
    walk_timer: u8,
    powerup: u8,
    flutter_pose: bool,
    throw_pose: bool,
) -> u8 {
    let base = base_frame_index(
        dir_right,
        on_ground_or_object,
        in_air,
        ducking,
        hvel,
        walk_timer,
    );
    let off = match powerup {
        0 => 32,
        2 => 69 + if flutter_pose { 26 } else { 0 },
        3 => 112 + if throw_pose { 32 } else { 0 },
        _ => 0,
    };
    base.wrapping_add(off)
}

/// One expanded player OAM cell, in the raw OAM byte space (`$A100` shadow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub y: u8,
    pub x: u8,
    pub tile: u8,
    pub attr: u8,
}

/// Additive style for a mapping: existing palette and priority bits survive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellStyle {
    pub force_palette_one: bool,
    pub behind_background: bool,
}

/// The player's OAM mapping for a frame.
pub fn mapping_for(tables: &ObjectTables, frame: u8) -> Option<&[MappingEntry]> {
    tables
        .player_mappings
        .get(frame as usize)
        .map(Vec::as_slice)
}

/// Append a frame at a wrapping screen position, preserving mapping order.
///
/// A missing frame returns `None` without modifying `cells`; an empty frame
/// succeeds. The caller owns the capacity policy for the complete sprite batch.
pub fn append_cells(
    tables: &ObjectTables,
    frame: u8,
    base: ScreenPosition,
    style: CellStyle,
    cells: &mut Vec<Cell>,
) -> Option<()> {
    let mapping = mapping_for(tables, frame)?;
    let attr_mask = if style.force_palette_one { 0x10 } else { 0 }
        | if style.behind_background { 0x80 } else { 0 };
    cells.extend(mapping.iter().map(|entry| Cell {
        y: base.y.wrapping_add(entry.y as u8),
        x: base.x.wrapping_add(entry.x as u8),
        tile: entry.tile,
        attr: entry.attr | attr_mask,
    }));
    Some(())
}

/// Expand one player frame like the `$5297` expander. `star_flash` = the
/// `$FFC7` value (1 while the invincibility timer flickers) forces attr
/// bit 4; attr bit 7 is never forced on the player path.
pub fn draw_cells(
    tables: &ObjectTables,
    frame: u8,
    mario_x: u16,
    mario_y: u16,
    cam_x: u16,
    cam_y: u16,
    star_flash: bool,
) -> Option<Vec<Cell>> {
    draw_cells_styled(
        tables,
        frame,
        mario_x,
        mario_y,
        cam_x,
        cam_y,
        CellStyle {
            force_palette_one: star_flash,
            behind_background: false,
        },
    )
}

/// `$5297` with an explicit style. The expander sets attr bit 7 on every entry
/// when `$FFBB` is 0 (`$52D5-$52DD`), which is how the console puts Mario
/// behind solid background tiles such as a pipe he is travelling through:
/// pipe travel sets `$FFBB` to 0 at `$530D` and restores 1 for the in-front
/// states.
pub fn draw_cells_styled(
    tables: &ObjectTables,
    frame: u8,
    mario_x: u16,
    mario_y: u16,
    cam_x: u16,
    cam_y: u16,
    style: CellStyle,
) -> Option<Vec<Cell>> {
    let base = ScreenPosition {
        x: (mario_x as u8).wrapping_sub(cam_x as u8).wrapping_add(96),
        y: (mario_y as u8).wrapping_sub(cam_y as u8).wrapping_add(98),
    };
    let mut cells = Vec::new();
    append_cells(tables, frame, base, style, &mut cells)?;
    Some(cells)
}

#[cfg(test)]
#[path = "player_mapping_tests.rs"]
mod mapping_tests;
