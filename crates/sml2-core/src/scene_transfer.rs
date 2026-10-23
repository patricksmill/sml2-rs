//! Blocking timing of `$Bx` tile transfers (`$216C -> $2202 -> $22B9`).
use super::{SceneScripts, SceneVm, OPERANDS};
use crate::vram_transfer::{VramTransfer, TRANSFER_BURST_MASK};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneTransferBlock {
    pub bank: u8,
    pub address: u16,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneTransferAssets {
    pub schema: String,
    pub blocks: Vec<SceneTransferBlock>,
}

impl SceneTransferAssets {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let assets: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if assets.schema != "sml2-scene-transfers/1" {
            return Err("unsupported scene transfer asset schema".into());
        }
        for block in &assets.blocks {
            if block.address < 0x4000 || usize::from(block.address) + block.bytes.len() > 0x8000 {
                return Err("scene transfer source crosses its ROM bank".into());
            }
        }
        Ok(assets)
    }

    /// Extract only the source ranges referenced by complete scene instructions.
    pub fn extract(rom: &[u8], scripts: &SceneScripts) -> Result<Self, String> {
        let mut assets = Self {
            schema: "sml2-scene-transfers/1".into(),
            blocks: vec![],
        };
        for id in 0..32 {
            let Some(mut pc) = scripts.script(id) else {
                continue;
            };
            while let Some(op) = scripts.byte_at(pc) {
                if op == 0xFF {
                    break;
                }
                let Some(len) = OPERANDS[usize::from(op >> 4)] else {
                    break;
                };
                if op >> 4 == 0xB {
                    let request = scripts
                        .tile_transfer_request(pc, op)
                        .ok_or("truncated tile transfer")?;
                    let length = usize::from(request.remaining_bytes);
                    if request.source_addr < 0x4000
                        || usize::from(request.source_addr) + length > 0x8000
                    {
                        return Err("scene transfer source crosses its ROM bank".into());
                    }
                    let offset = VramTransfer::rom_offset(request.source_bank, request.source_addr);
                    let bytes = rom
                        .get(offset..offset + length)
                        .ok_or("scene transfer ROM source missing")?;
                    let block = SceneTransferBlock {
                        bank: request.source_bank,
                        address: request.source_addr,
                        bytes: bytes.to_vec(),
                    };
                    if !assets.blocks.contains(&block) {
                        assets.blocks.push(block);
                    }
                }
                pc = pc.wrapping_add(1 + u16::from(len));
            }
        }
        Ok(assets)
    }

    fn source(&self, bank: u8, address: u16, length: usize) -> Option<&[u8]> {
        self.blocks.iter().find_map(|block| {
            if block.bank != bank {
                return None;
            }
            let offset = usize::from(address.checked_sub(block.address)?);
            block.bytes.get(offset..offset + length)
        })
    }
}

impl SceneScripts {
    pub fn set_transfer_assets(&mut self, assets: SceneTransferAssets) {
        self.transfers = assets;
    }

    /// Every ROM source a `$Bx` tile transfer can copy from.
    pub fn transfer_blocks(&self) -> &[SceneTransferBlock] {
        &self.transfers.blocks
    }

    fn tile_transfer_request(&self, pc: u16, opcode: u8) -> Option<VramTransfer> {
        let bank = self.byte_at(pc.wrapping_add(1))?;
        let address = u16::from_le_bytes([
            self.byte_at(pc.wrapping_add(2))?,
            self.byte_at(pc.wrapping_add(3))?,
        ]);
        let (dest, length) = match opcode & 15 {
            1 => (0x9200, 0x600),
            3 => (0x8B00, 0x400),
            _ => (0x8B00, 0x380),
        };
        Some(VramTransfer::new(bank, address, dest, length))
    }
}

/// One completed `$26D7` burst, applied to both views of shared VRAM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneVramWrite {
    pub address: u16,
    pub bytes: Vec<u8>,
}

impl SceneVramWrite {
    pub fn apply_tile_blocks(&self, bg: &mut [u8], obj: Option<&mut [u8]>) -> Result<(), String> {
        let start = usize::from(self.address);
        let end = start + self.bytes.len();
        if start < 0x8000
            || end > 0x9800
            || bg.len() < 0x1000
            || obj.as_ref().is_some_and(|o| o.len() < 0x1000)
        {
            return Err("scene transfer destination is outside the tile buffers".into());
        }
        let copy = |base: usize, buffer: &mut [u8]| {
            let lo = start.max(base);
            let hi = end.min(base + buffer.len());
            if lo < hi {
                buffer[lo - base..hi - base].copy_from_slice(&self.bytes[lo - start..hi - start]);
            }
        };
        copy(0x8800, bg);
        if let Some(obj) = obj {
            copy(0x8000, obj);
        }
        Ok(())
    }
}

impl SceneVm {
    pub fn take_transfer_write(&mut self) -> Option<SceneVramWrite> {
        self.transfer_write.take()
    }

    pub(super) fn begin_tile_transfer(&mut self, opcode: u8, scripts: &SceneScripts) -> bool {
        self.wait_for_tile_transfer(opcode);
        let Some(request) = scripts.tile_transfer_request(self.pc, opcode) else {
            return false;
        };
        if scripts
            .transfers
            .source(
                request.source_bank,
                request.source_addr,
                usize::from(request.remaining_bytes),
            )
            .is_none()
        {
            return false;
        }
        self.tile_transfer = Some(request);
        self.copy_transfer_burst(scripts);
        true
    }

    pub(super) fn copy_transfer_burst(&mut self, scripts: &SceneScripts) {
        let Some(request) = self.tile_transfer.as_mut().filter(|r| r.active) else {
            return;
        };
        let count = request.chunk_len();
        let source = scripts
            .transfers
            .source(request.source_bank, request.source_addr, count)
            .expect("scene transfer source validated at opcode entry");
        let mut write = SceneVramWrite {
            address: request.dest_addr,
            bytes: vec![0; count],
        };
        request
            .step_buffer(source, 0, &mut write.bytes, 0)
            .expect("validated scene transfer burst");
        self.transfer_write = Some(write);
    }

    pub(super) fn wait_for_tile_transfer(&mut self, opcode: u8) {
        // `$2202/$222E/$224E`: B1 copies $600 bytes to $9200, B3 copies
        // $400 to $8B00, and every other low nibble copies $380 to $8B00.
        let length: u16 = match opcode & 0x0F {
            1 => 0x600,
            3 => 0x400,
            _ => 0x380,
        };
        // `$22D5` waits for every 64-byte burst serviced by `$26D7`.
        // `$21E8` then waits once more. The opcode-fetch step spends the
        // first transfer wait, leaving `bursts` waits before the next opcode.
        self.opcode_wait = (length / (TRANSFER_BURST_MASK + 1)) as u8;
        self.opcode_transition_tail = false;
    }
}

#[cfg(test)]
#[path = "scene_transfer_tests.rs"]
mod tests;
