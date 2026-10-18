//! Physical storage projection at the legacy slot boundary only.
use super::{ObjectSim, RECORD_LEN, STREAM_BASE};
use crate::actor::{Actor, Position, SpawnOrigin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecordResetError {
    OversizedStream { records: usize },
    UnsupportedAddress { address: u16 },
    MissingSprite { level: u8, definition: u8 },
}

impl ObjectSim {
    pub(super) fn resolve_spawn_origin(&self, pointer: u16) -> Result<SpawnOrigin, RecordResetError> {
        // Six prefix bytes + 84 records + two terminal bytes fill $AB00..$ACFF.
        if self.records.len() > 84 {
            return Err(RecordResetError::OversizedStream { records: self.records.len() });
        }
        let read = |address: u16| -> Result<u8, RecordResetError> {
            match address {
                0xAB00..=0xACFF => {
                    if address < STREAM_BASE { return Ok(0xFF); }
                    let offset = usize::from(address - STREAM_BASE);
                    let index = offset / RECORD_LEN;
                    if let Some(record) = self.records.get(index) {
                        let [xh, xl] = record.x.to_be_bytes();
                        let [yh, yl] = record.y.to_be_bytes();
                        Ok([xh, xl, record.mode_flag, yh, yl, record.anim_index][offset % RECORD_LEN])
                    } else {
                        Ok(if offset - self.records.len() * RECORD_LEN < 2 { 0xFF } else { 0 })
                    }
                }
                0xAD00..=0xAEFF => {
                    let offset = usize::from(address - 0xAD00);
                    let actor = Actor::from_raw_slot(&self.slots[offset / 32].b);
                    Ok(actor.to_raw_slot()[offset % 32])
                }
                _ => Err(RecordResetError::UnsupportedAddress { address }),
            }
        };
        let start = pointer.wrapping_sub(2);
        let mut bytes = [0; 5];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = read(start.wrapping_add(i as u16))?;
        }
        Ok(SpawnOrigin {
            position: Position { x: u16::from_be_bytes([bytes[0], bytes[1]]), y: u16::from_be_bytes([bytes[3], bytes[4]]) },
            marker: bytes[2],
        })
    }
}
