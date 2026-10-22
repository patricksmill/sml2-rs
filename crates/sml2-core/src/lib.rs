//! Core simulation structures and definitions for Super Mario Land 2.

pub mod actor;
pub mod audio;
pub mod camera;
pub mod character;
pub mod castle;
pub mod castle_room;
pub mod room_sprites;
pub mod castle_sequence;
pub mod collision;
pub mod credits_text;
pub mod divider;
pub mod ending_entry;
pub mod easy_mode_card;
pub mod ending_raster;
pub mod ending_render;
pub mod ending_scene;
pub mod ending_sequence;
pub mod ending_sprites;
pub mod front_end;
pub mod hardware_palettes;
pub mod level;
pub mod level_timer;
pub mod map;
pub mod map_sim;
pub mod map_zone_decor;
pub mod object;
mod object_events;
pub mod objects;
pub mod player;
pub mod player_sim;
#[cfg(test)]
#[path = "route_matrix_tests.rs"]
mod route_matrix_tests;
pub mod scene;
pub mod sprite_batch;
pub mod tile_animation;
pub mod tileset;
pub mod video;
pub mod vram_transfer;

/// Mario powerup state (`sCurPowerup` $A216; the draw offsets in
/// [`player::frame_index`] are the ground truth for these labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MarioPowerup {
    #[default]
    Small = 0,
    Super = 1,
    Bunny = 2,
    Fire = 3,
}

/// Mario movement and physics state (mapped from WRAM $A200-$A240).
#[derive(Debug, Clone, Default)]
pub struct MarioState {
    /// World position X (subpixel + pixel, 16-bit).
    pub x: u16,
    /// World position Y (subpixel + pixel, 16-bit).
    pub y: u16,
    /// Horizontal velocity (signed 8-bit fixed point).
    pub vx: i8,
    /// Vertical velocity (signed 8-bit fixed point).
    pub vy: i8,
    /// Powerup state.
    pub powerup: MarioPowerup,
    /// Is Mario currently on the ground?
    pub on_ground: bool,
    /// Is Mario facing right?
    pub facing_right: bool,
    /// Invulnerability/flash timer.
    pub invulnerable_timer: u8,
}

impl MarioState {
    pub fn new(spawn_x: u16, spawn_y: u16) -> Self {
        Self {
            x: spawn_x,
            y: spawn_y,
            facing_right: true,
            ..Default::default()
        }
    }
}
