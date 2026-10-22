//! Native VRAM transfer service for ROM $026D7.
//!
//! Handles progressive DMA-like tile and tilemap copies into VRAM in 64-byte
//! bursts per frame until the remaining length counter reaches zero.

use crate::ending_scene::EndingScene;

pub const TRANSFER_BURST_MASK: u16 = 0x3F; // low 6 bits: 64-byte cadence
pub const VRAM_TILES_BASE: u16 = 0x8000;
pub const VRAM_TILES_END: u16 = 0x9800;
pub const VRAM_TILEMAP_BASE: u16 = 0x9800;
pub const VRAM_TILEMAP_END: u16 = 0x9C00;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferError {
    SourceOutOfBounds { required: usize, available: usize },
    DestinationOutOfBounds { dest_addr: u16, length: usize },
    SceneAssetsMissing,
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceOutOfBounds {
                required,
                available,
            } => {
                write!(
                    f,
                    "transfer source out of bounds: needed {required} bytes, available {available}"
                )
            }
            Self::DestinationOutOfBounds { dest_addr, length } => {
                write!(
                    f,
                    "transfer destination out of bounds at 0x{dest_addr:04X} for {length} bytes"
                )
            }
            Self::SceneAssetsMissing => {
                write!(
                    f,
                    "cannot apply transfer: ending scene has no loaded assets"
                )
            }
        }
    }
}

impl std::error::Error for TransferError {}

/// Active VRAM transfer request parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VramTransfer {
    pub source_bank: u8,
    pub source_addr: u16,
    pub dest_addr: u16,
    pub remaining_bytes: u16,
    pub active: bool,
}

/// Result of stepping one frame of VRAM transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferStepResult {
    pub bytes_transferred: usize,
    pub remaining_bytes: u16,
    pub completed: bool,
}

impl VramTransfer {
    pub fn new(source_bank: u8, source_addr: u16, dest_addr: u16, length: u16) -> Self {
        Self {
            source_bank,
            source_addr,
            dest_addr,
            remaining_bytes: length,
            active: length > 0,
        }
    }

    /// Creates ending request 1 ($4656 -> $8000, len $0800, bank $1A).
    pub fn ending_request_1() -> Self {
        Self::new(0x1A, 0x4656, 0x8000, 0x0800)
    }

    /// Creates ending request 2 ($5096 -> $9800, len $0400, bank $1A).
    pub fn ending_request_2() -> Self {
        Self::new(0x1A, 0x5096, 0x9800, 0x0400)
    }

    /// Computes the offset in a 16KB banked ROM slice for a given CPU address $4000..$7FFF.
    pub const fn bank_offset(cpu_addr: u16) -> usize {
        cpu_addr.wrapping_sub(0x4000) as usize
    }

    /// Computes the absolute ROM offset given bank and CPU address.
    pub const fn rom_offset(bank: u8, cpu_addr: u16) -> usize {
        (bank as usize) * 0x4000 + ((cpu_addr as usize) & 0x3FFF)
    }

    /// Determines the number of bytes that should be copied on this frame.
    ///
    /// Copies until `(remaining_bytes & 0x3F) == 0`.
    /// When remaining_bytes is aligned to 64, this returns 64 (or remaining if less).
    /// When unaligned, this returns the remainder down to the 64-byte boundary.
    pub fn chunk_len(&self) -> usize {
        if self.remaining_bytes == 0 {
            return 0;
        }
        let rem_in_chunk = (self.remaining_bytes & TRANSFER_BURST_MASK) as usize;
        if rem_in_chunk == 0 {
            ((TRANSFER_BURST_MASK + 1) as usize).min(self.remaining_bytes as usize)
        } else {
            rem_in_chunk
        }
    }

    /// Steps one frame of transfer by copying bytes directly from `source` at `src_offset`
    /// to `dest` at `dest_offset`.
    pub fn step_buffer(
        &mut self,
        source: &[u8],
        src_offset: usize,
        dest: &mut [u8],
        dest_offset: usize,
    ) -> Result<TransferStepResult, TransferError> {
        if !self.active || self.remaining_bytes == 0 {
            self.active = false;
            return Ok(TransferStepResult {
                bytes_transferred: 0,
                remaining_bytes: 0,
                completed: true,
            });
        }

        let chunk = self.chunk_len();
        if src_offset + chunk > source.len() {
            return Err(TransferError::SourceOutOfBounds {
                required: src_offset + chunk,
                available: source.len(),
            });
        }
        if dest_offset + chunk > dest.len() {
            return Err(TransferError::DestinationOutOfBounds {
                dest_addr: self.dest_addr,
                length: chunk,
            });
        }

        dest[dest_offset..dest_offset + chunk]
            .copy_from_slice(&source[src_offset..src_offset + chunk]);

        self.source_addr = self.source_addr.wrapping_add(chunk as u16);
        self.dest_addr = self.dest_addr.wrapping_add(chunk as u16);
        self.remaining_bytes = self.remaining_bytes.wrapping_sub(chunk as u16);

        if self.remaining_bytes == 0 {
            self.active = false;
        }

        Ok(TransferStepResult {
            bytes_transferred: chunk,
            remaining_bytes: self.remaining_bytes,
            completed: !self.active,
        })
    }

    /// Steps one frame using a 16KB banked ROM slice as source.
    pub fn step_banked(
        &mut self,
        bank_data: &[u8],
        dest: &mut [u8],
        dest_offset: usize,
    ) -> Result<TransferStepResult, TransferError> {
        let src_offset = Self::bank_offset(self.source_addr);
        self.step_buffer(bank_data, src_offset, dest, dest_offset)
    }

    /// Steps one frame using a full ROM image as source.
    pub fn step_rom(
        &mut self,
        rom: &[u8],
        dest: &mut [u8],
        dest_offset: usize,
    ) -> Result<TransferStepResult, TransferError> {
        let src_offset = Self::rom_offset(self.source_bank, self.source_addr);
        self.step_buffer(rom, src_offset, dest, dest_offset)
    }

    /// Steps one frame and copies directly into the tiles or tilemap of `EndingScene`.
    pub fn step_scene(
        &mut self,
        rom: &[u8],
        scene: &mut EndingScene,
    ) -> Result<TransferStepResult, TransferError> {
        if !self.active || self.remaining_bytes == 0 {
            self.active = false;
            return Ok(TransferStepResult {
                bytes_transferred: 0,
                remaining_bytes: 0,
                completed: true,
            });
        }

        let chunk = self.chunk_len();
        let src_offset = Self::rom_offset(self.source_bank, self.source_addr);
        if src_offset + chunk > rom.len() {
            return Err(TransferError::SourceOutOfBounds {
                required: src_offset + chunk,
                available: rom.len(),
            });
        }
        self.apply_chunk(&rom[src_offset..src_offset + chunk], scene)
    }

    /// [`Self::step_scene`] reading the source from the scene's own
    /// [`crate::ending_scene::EndingAssets::transfer_source`] blocks instead of a
    /// ROM image.
    pub fn step_scene_from_assets(
        &mut self,
        scene: &mut EndingScene,
    ) -> Result<TransferStepResult, TransferError> {
        if !self.active || self.remaining_bytes == 0 {
            self.active = false;
            return Ok(TransferStepResult {
                bytes_transferred: 0,
                remaining_bytes: 0,
                completed: true,
            });
        }
        let chunk = self.chunk_len();
        let assets = scene.assets().ok_or(TransferError::SceneAssetsMissing)?;
        let source = assets
            .transfer_source(self.source_bank, self.source_addr, chunk)
            .ok_or(TransferError::SourceOutOfBounds {
                required: chunk,
                available: 0,
            })?
            .to_vec();
        self.apply_chunk(&source, scene)
    }

    fn apply_chunk(
        &mut self,
        source: &[u8],
        scene: &mut EndingScene,
    ) -> Result<TransferStepResult, TransferError> {
        let chunk = source.len();
        let dest_addr = self.dest_addr;
        let assets = scene
            .assets_mut()
            .ok_or(TransferError::SceneAssetsMissing)?;

        if dest_addr >= VRAM_TILES_BASE && dest_addr + (chunk as u16) <= VRAM_TILES_END {
            let offset = (dest_addr - VRAM_TILES_BASE) as usize;
            let tiles = assets.tiles_mut();
            tiles[offset..offset + chunk].copy_from_slice(source);
        } else if dest_addr >= VRAM_TILEMAP_BASE && dest_addr + (chunk as u16) <= VRAM_TILEMAP_END {
            let offset = (dest_addr - VRAM_TILEMAP_BASE) as usize;
            let tilemap = assets.tilemap_mut();
            tilemap[offset..offset + chunk].copy_from_slice(source);
        } else {
            return Err(TransferError::DestinationOutOfBounds {
                dest_addr,
                length: chunk,
            });
        }

        self.source_addr = self.source_addr.wrapping_add(chunk as u16);
        self.dest_addr = self.dest_addr.wrapping_add(chunk as u16);
        self.remaining_bytes = self.remaining_bytes.wrapping_sub(chunk as u16);

        if self.remaining_bytes == 0 {
            self.active = false;
        }

        Ok(TransferStepResult {
            bytes_transferred: chunk,
            remaining_bytes: self.remaining_bytes,
            completed: !self.active,
        })
    }
}

#[cfg(test)]
mod tests;
