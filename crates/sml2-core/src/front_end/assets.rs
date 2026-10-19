//! Title-screen and file-select VRAM/tilemap bundles.
//!
//! Both screens use the full `$8000-$97FF` background window and a 32x32
//! tilemap. The extractor composes the window from the ROM slices the loaders
//! copy; the runtime renderer only needs these four blobs.

/// `$8000-$97FF`: 384 tiles of 16 bytes.
pub const VRAM_LEN: usize = 0x1800;
/// One 32x32 background map.
pub const TILEMAP_LEN: usize = 0x400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontEndAssets {
    title_vram: Vec<u8>,
    title_tilemap: Vec<u8>,
    file_select_vram: Vec<u8>,
    file_select_tilemap: Vec<u8>,
}

impl FrontEndAssets {
    pub fn new(
        title_vram: Vec<u8>,
        title_tilemap: Vec<u8>,
        file_select_vram: Vec<u8>,
        file_select_tilemap: Vec<u8>,
    ) -> Result<Self, String> {
        for (name, blob, len) in [
            ("title_vram", &title_vram, VRAM_LEN),
            ("title_tilemap", &title_tilemap, TILEMAP_LEN),
            ("file_select_vram", &file_select_vram, VRAM_LEN),
            ("file_select_tilemap", &file_select_tilemap, TILEMAP_LEN),
        ] {
            if blob.len() != len {
                return Err(format!(
                    "{name} is {} bytes, expected {len}",
                    blob.len()
                ));
            }
        }
        Ok(Self {
            title_vram,
            title_tilemap,
            file_select_vram,
            file_select_tilemap,
        })
    }

    /// `$41E5`'s 0x1800 bytes, loaded at `$8000` by `$14043`.
    pub fn title_vram(&self) -> &[u8] {
        &self.title_vram
    }

    /// `$59E5`'s 0x400 bytes, copied to `$9800` by `$14043`.
    pub fn title_tilemap(&self) -> &[u8] {
        &self.title_tilemap
    }

    /// The file select's `$8000-$97FF` window, composed by `$2B8B`.
    pub fn file_select_vram(&self) -> &[u8] {
        &self.file_select_vram
    }

    /// `$4A52`'s 0x400 bytes, copied to `$9800` by `$30000`.
    pub fn file_select_tilemap(&self) -> &[u8] {
        &self.file_select_tilemap
    }
}

#[cfg(test)]
mod tests;
