//! The `$6109B` metasprite expander used by the castle rooms.
//!
//! A room's 16 records at `$A6B5` are `(x, y, id)`; `id` indexes a table of
//! 64 little-endian pointers, and each sprite is a run of 4-byte records
//! `(dy, dx, tile, attr)` ended by a `$FF` tile. `$3F75` picks the bell
//! table (`$62E6`) and defers to `$6109B`; the coin rooms pass `$51B1`.

/// Pointers in the table.
pub const TABLE_ENTRIES: usize = 64;
/// `$6109B` stops once `sMapDecorOAMIndex` reaches `$28`.
pub const OAM_LIMIT: usize = 0x28;

/// One expanded OAM entry, `(x, y, tile, attr)`.
pub type OamCell = (u8, u8, u8, u8);

/// A metasprite table plus the records it points into.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomSpriteTable {
    base: u16,
    bytes: Vec<u8>,
}

impl RoomSpriteTable {
    /// `base` is the table's bank address (`$51B1`/`$62E6`); pointers are
    /// absolute, so the blob must start there.
    pub fn new(base: u16, bytes: &[u8]) -> Self {
        Self {
            base,
            bytes: bytes.to_vec(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.len() < TABLE_ENTRIES * 2
    }

    /// Expand `id` at `(x, y)`; the ROM adds each metasprite record's
    /// first offset to y and its second offset to x.
    pub fn expand(&self, id: u8, x: u8, y: u8) -> Vec<OamCell> {
        let mut cells = Vec::new();
        if self.is_empty() {
            return cells;
        }
        let index = id as usize;
        if index >= TABLE_ENTRIES {
            return cells;
        }
        let ptr = u16::from_le_bytes([self.bytes[index * 2], self.bytes[index * 2 + 1]]);
        if ptr < self.base {
            return cells;
        }
        let mut at = (ptr - self.base) as usize;
        while at + 4 <= self.bytes.len() && cells.len() < OAM_LIMIT {
            let dy = self.bytes[at];
            let dx = self.bytes[at + 1];
            let tile = self.bytes[at + 2];
            let attr = self.bytes[at + 3];
            at += 4;
            if tile == 0xFF {
                break;
            }
            cells.push((x.wrapping_add(dx), y.wrapping_add(dy), tile, attr));
        }
        cells
    }

    /// Expand every live record of a room's 48-byte table.
    pub fn expand_records(&self, records: &[u8; 48]) -> Vec<OamCell> {
        let mut cells = Vec::new();
        for index in 0..16 {
            let at = index * 3;
            let (x, y, id) = (records[at], records[at + 1], records[at + 2]);
            if x == 0xFF || id == 0xFF {
                continue;
            }
            cells.extend(self.expand(id, x, y));
        }
        cells
    }
}

/// `$5297`: the map's sprite table (bank 1 `$4000`). `$FFC6` holds an *id*,
/// not a tile; each id points at 4-byte `(dy, dx, tile, attr)` records that
/// end when the y-offset is `$80`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpriteMetas {
    bytes: Vec<u8>,
}

impl SpriteMetas {
    pub const BASE: u16 = 0x4000;
    pub const ENTRIES: usize = 242;

    pub fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.len() < Self::ENTRIES * 2
    }

    /// Expand the sprite `id` at `(x, y)`; the record's offsets are signed
    /// bytes relative to the sprite's centre.
    pub fn expand(&self, id: u8, x: u8, y: u8) -> Vec<OamCell> {
        let mut cells = Vec::new();
        if self.is_empty() || id as usize >= Self::ENTRIES {
            return cells;
        }
        let index = id as usize;
        let ptr = u16::from_le_bytes([self.bytes[index * 2], self.bytes[index * 2 + 1]]);
        if ptr < Self::BASE {
            return cells;
        }
        let mut at = (ptr - Self::BASE) as usize;
        while at + 4 <= self.bytes.len() && cells.len() < OAM_LIMIT {
            let (dy, dx, tile, attr) = (
                self.bytes[at],
                self.bytes[at + 1],
                self.bytes[at + 2],
                self.bytes[at + 3],
            );
            at += 4;
            if dy == 0x80 {
                break;
            }
            cells.push((x.wrapping_add(dx), y.wrapping_add(dy), tile, attr));
        }
        cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `$6109B` adds `(dy, dx)` to room records stored as `(x, y, id)`.
    #[test]
    fn room_records_place_the_claw_and_powerup_cells_on_their_rom_axes() {
        let scripts = crate::castle::CastleScripts::from_parts(
            include_bytes!("../../../extracted/map/castle_scripts.bin").to_vec(),
            include_str!("../../../extracted/map/castle_scripts.json"),
        )
        .expect("castle scripts asset");
        let table = RoomSpriteTable::new(0x51B1, scripts.blob("room_metasprites").unwrap());
        let mut records = [0xFF; 48];
        // `$66D4` record 2: x=$98, y=$64, claw frame 5. The top cells
        // have dy=-24 and dx=-2/+6, so swapping either axis changes bytes.
        records[..3].copy_from_slice(&scripts.blob("coin_room_9_sprites").unwrap()[6..9]);
        assert_eq!(
            table.expand_records(&records),
            vec![
                (150, 76, 0xBE, 0),
                (158, 76, 0xBF, 0),
                (150, 84, 0xC0, 0),
                (158, 84, 0xC1, 0),
                (152, 92, 0xC7, 0),
                (152, 100, 0xC6, 0),
            ]
        );
        // ROM frame $0C is the mushroom: distinct top-left/top-right tiles.
        records[..3].copy_from_slice(&[40, 100, 0x0C]);
        assert_eq!(
            table.expand_records(&records),
            vec![
                (40, 100, 0x90, 0),
                (48, 100, 0x91, 0),
                (40, 108, 0x92, 0),
                (48, 108, 0x92, 0x20),
            ]
        );
    }

    #[test]
    fn the_sprite_table_expands_a_mario_frame() {
        let rom = std::fs::read("../../extracted/map/castle_scripts.bin")
            .or_else(|_| std::fs::read("extracted/map/castle_scripts.bin"))
            .expect("castle scripts");
        let json = std::fs::read_to_string("../../extracted/map/castle_scripts.json")
            .or_else(|_| std::fs::read_to_string("extracted/map/castle_scripts.json"))
            .expect("castle script index");
        let meta: serde_json::Value = serde_json::from_str(&json).expect("json");
        let entry = meta["blobs"]
            .as_array()
            .expect("blobs")
            .iter()
            .find(|b| b["name"] == "sprite_metasprites")
            .expect("blob");
        let at = entry["offset"].as_u64().expect("offset") as usize;
        let len = entry["len"].as_u64().expect("len") as usize;
        let table = SpriteMetas::new(&rom[at..at + len]);
        let cells = table.expand(0xBB, 0x28, 0x60);
        assert_eq!(cells.len(), 4, "a 2x2 frame");
        assert_eq!(cells[0], (32, 88, 0x0D, 0x00));
        assert_eq!(cells[1], (40, 88, 0x0D, 0x20));
        assert_eq!(cells[2], (32, 96, 0x0E, 0x00));
        assert_eq!(cells[3], (40, 96, 0x0E, 0x20));
    }

    #[test]
    fn the_coin_rooms_first_sprite_is_a_two_by_three_block() {
        let rom = std::fs::read("../../extracted/map/castle_scripts.bin")
            .or_else(|_| std::fs::read("extracted/map/castle_scripts.bin"))
            .expect("castle scripts");
        let json = std::fs::read_to_string("../../extracted/map/castle_scripts.json")
            .or_else(|_| std::fs::read_to_string("extracted/map/castle_scripts.json"))
            .expect("castle script index");
        let meta: serde_json::Value = serde_json::from_str(&json).expect("json");
        let blobs = meta["blobs"].as_array().expect("blobs");
        let blob = |name: &str| -> &[u8] {
            let entry = blobs
                .iter()
                .find(|b| b["name"] == name)
                .unwrap_or_else(|| panic!("{name}"));
            let at = entry["offset"].as_u64().expect("offset") as usize;
            let len = entry["len"].as_u64().expect("len") as usize;
            &rom[at..at + len]
        };
        let table = RoomSpriteTable::new(0x51B1, blob("room_metasprites"));
        let cells = table.expand(0x00, 0xA0, 0x20);
        // Eight visible tiles plus two `$E0` records whose y wraps to 0, which
        // the hardware hides above the screen.
        assert_eq!(cells.len(), 10);
        assert_eq!(cells[0], (0xA0, 0x20, 0x7F, 0x20));
        assert_eq!(cells[7], (0xA8, 0x38, 0x7F, 0x20));
        assert_eq!(cells[8], (0x80, 0x00, 0x7F, 0x20), "hidden by y = 0");

        let bell = RoomSpriteTable::new(0x62E6, blob("bell_metasprites"));
        let cells = bell.expand(0x00, 0x7C, 0xF0);
        assert_eq!(cells.len(), 6, "bell sprite 0 is a 2x3 block");
        assert_eq!(cells[0], (0x7C, 0xF0, 0x30, 0x00));
    }
}
