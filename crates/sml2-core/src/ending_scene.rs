//! Native scene load for home ROM $29DD; the ending controller is a separate owner.
use serde::{Deserialize, Serialize};

use crate::{ending_entry::EndingPalettes, sprite_batch::SpriteBatch, video::bg_tile_offset};

pub const ENDING_ASSET_SCHEMA: &str = "sml2-ending/1";
pub const ENDING_TILES_LEN: usize = 0x1800;
pub const ENDING_TILEMAP_LEN: usize = 0x400;

/// Original 2bpp data and tile IDs, independent of any output palette or texture pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AssetFile")]
pub struct EndingAssets {
    schema: String,
    tiles: Vec<u8>,
    tilemap: Vec<u8>,
}

#[derive(Deserialize)]
struct AssetFile {
    schema: String,
    tiles: Vec<u8>,
    tilemap: Vec<u8>,
}

impl TryFrom<AssetFile> for EndingAssets {
    type Error = String;

    fn try_from(file: AssetFile) -> Result<Self, Self::Error> {
        if file.schema != ENDING_ASSET_SCHEMA {
            return Err(format!("unsupported ending asset schema: {}", file.schema));
        }
        Self::new(file.tiles, file.tilemap)
    }
}

impl EndingAssets {
    pub fn new(tiles: Vec<u8>, tilemap: Vec<u8>) -> Result<Self, String> {
        if tiles.len() != ENDING_TILES_LEN || tilemap.len() != ENDING_TILEMAP_LEN {
            return Err(format!(
                "ending assets require 6144 tile bytes and 1024 tile IDs, got {} and {}",
                tiles.len(),
                tilemap.len()
            ));
        }
        Ok(Self {
            schema: ENDING_ASSET_SCHEMA.into(),
            tiles,
            tilemap,
        })
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("ending assets: {e}"))
    }

    /// VRAM $8000..$97FF: 384 original 16-byte tiles.
    pub fn tiles(&self) -> &[u8] {
        &self.tiles
    }

    /// The complete 32x32 background map, row-major.
    pub fn tilemap(&self) -> &[u8] {
        &self.tilemap
    }

    /// Signed BG addressing selects $8800..$97FF within the shared tile data.
    pub fn background_tile(&self, id: u8) -> &[u8; 16] {
        let offset = 0x800 + bg_tile_offset(id);
        self.tiles[offset..offset + 16].try_into().unwrap()
    }

    /// OBJ IDs select $8000..$8FFF, sharing tile data with the background.
    pub fn object_tile(&self, id: u8) -> &[u8; 16] {
        let offset = usize::from(id) * 16;
        self.tiles[offset..offset + 16].try_into().unwrap()
    }
}

/// Logical interpretation of $C3; the renderer need not emulate LCD hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EndingDisplayMode {
    pub display_enabled: bool,
    pub window_enabled: bool,
    pub window_upper_map: bool,
    pub background_unsigned_tiles: bool,
    pub background_upper_map: bool,
    pub tall_sprites: bool,
    pub objects_enabled: bool,
    pub background_enabled: bool,
}

/// Rendering state owned by the ending scene. No gameplay or sequence dispatch lives here.
#[derive(Debug, Clone)]
pub struct EndingScene {
    assets: Option<EndingAssets>,
    pub scroll_x: u8,
    pub scroll_y: u8,
    pub palettes: Option<EndingPalettes>,
    /// Forty sprite cells in native OAM order.
    pub objects: SpriteBatch,
    pub display: EndingDisplayMode,
    pub credits_sprite_y: u8,
}

impl Default for EndingScene {
    fn default() -> Self {
        Self {
            assets: None,
            scroll_x: 0,
            scroll_y: 0,
            palettes: None,
            objects: SpriteBatch::default(),
            display: EndingDisplayMode::default(),
            credits_sprite_y: 0,
        }
    }
}

impl EndingScene {
    pub fn assets(&self) -> Option<&EndingAssets> {
        self.assets.as_ref()
    }

    /// Complete logical effects of $29DD. The caller owns the ending substate
    /// and shared fade timer; other world/sequence state is not touched.
    pub fn load(&mut self, assets: EndingAssets, ending_state: &mut u8, fade: &mut u16) {
        self.assets = Some(assets);
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.palettes = Some(EndingPalettes {
            background: 0xE1,
            object0: 0xD2,
            object1: 0x39,
        });
        self.objects.clear_cells();
        self.display = EndingDisplayMode {
            display_enabled: true,
            window_enabled: false,
            window_upper_map: true,
            background_unsigned_tiles: false,
            background_upper_map: false,
            tall_sprites: false,
            objects_enabled: true,
            background_enabled: true,
        };
        *fade = 0x100;
        self.credits_sprite_y = 44;
        *ending_state = ending_state.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests;
