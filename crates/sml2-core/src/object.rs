//! Object taxonomy tables: how a spawn record's `anim` byte becomes a
//! drawable, collidable object.
//!
//! ## The chain (all verified against `main.asm` bank $02 + bank $03)
//!
//! 1. Spawned slot stores the 6 record bytes at +0..+5 (`$8C8D`); +5 = anim.
//! 2. `$8240` maps `anim -> sprite id` through a 96-byte table at ROM $8B61
//!    (CPU $4B61, used when `sSpriteMapIndex == 0`) or a 112-byte table at
//!    ROM $8BC1 (CPU $4BC1, when `sSpriteMapIndex != 0`, which covers anims
//!    0..111). The result goes to slot+11 / staged `$FFDB`.
//!    `sSpriteMapIndex` is set at level load (`main.asm` ~$9050):
//!    `1` when `sCurLevel > $0A`, else `0` (see [`sprite_map_index`]).
//! 3. `$81CF` copies the sprite's 4-byte record (via the pointer table at
//!    ROM $8737 / $8935) into slot+19..+22 — the collision extents.
//! 4. `$9048` (scene-state path) copies a second 4-byte record (table at ROM
//!    $905B) into the same slot field.
//! 5. `$C000` (bank 3) draws the sprite: `sprite id -> pointer` through ROM
//!    $C0B1 (168 entries) / $CF11 (228 entries) into the
//!    `gfx/spritemappings*.asm` data — `[Yoff, Xoff, tile, attr]` groups
//!    terminated by a first byte `$80` — expanded into OAM with X-flip bit 5
//!    and Y-flip bit 6 of `sSpriteFlipFlags`. The offsets land in the RAW OAM
//!    byte, so the on-screen position is `obj + offset - (8, 16)`.
//!
//! Extents are stored as four signed bytes; the value distribution is most
//! consistent with `[x0, x1, y0, y1]` offsets (e.g. sprite 0: -13..-3 x
//! -6..+6, an 11x13 box offset left of the object position).

use serde::{Deserialize, Serialize};

/// Schema id written into `objects.json`.
pub const OBJECTS_SCHEMA: &str = "sml2-objects/4";

/// The `sSpriteMapIndex` value a level is loaded with (main.asm ~$9050:
/// `ld a,[sCurLevel] / cp $0A / ret c` — so level ids **`>= $0A`** use the "B"
/// tables, and level `$0A` itself is mode B).
pub fn sprite_map_index(level_id: u8) -> u8 {
    u8::from(level_id >= 0x0A)
}

/// One sprite's 4-byte collision extents, `[x0, x1, y0, y1]`.
pub type Extents = [i8; 4];

/// One sprite-mapping entry: `[y offset, x offset, tile, attr]`. The offsets
/// are signed (the engine adds them to the object position mod 256 — an
/// offset of `$F0` is -16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingEntry {
    pub y: i8,
    pub x: i8,
    pub tile: u8,
    pub attr: u8,
}

/// One anim's behavior profile (bank $16).
///
/// `handler` is the per-anim entry point (the bank-$16 update reads the anim
/// and `rst $28`-dispatches through the `$58040` (96) / `$59C07` (112) `dw`
/// tables). `type_sets` collects the `ld a, N / ldh [$FFE2], a` immediates
/// found in the handler's code region — the type byte drives collision vs
/// Mario (e.g. the level-0 walker sets $4E = harmful). `states` collects the
/// targets of every inline `rst $28` (`$EF`) sub-dispatch table in the
/// region — the handler's state-machine code addresses. Both are static
/// scans of the region up to the next handler entry: exact for what they
/// list, but not exhaustive (shared helpers/downstream code are not
/// followed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimProfile {
    pub handler: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_sets: Vec<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<u32>,
}

/// Global object tables extracted from the ROM (bank $02/$03/$16).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectTables {
    pub schema: String,
    pub notes: Vec<String>,
    /// anim -> sprite id, mode A (`sSpriteMapIndex == 0`), 96 entries.
    pub anim_to_sprite_a: Vec<u8>,
    /// anim -> sprite id, mode B, 112 entries (anims 0..111).
    pub anim_to_sprite_b: Vec<u8>,
    /// Distinct 4-byte extent records for mode A/B.
    pub hitbox_records_a: Vec<Extents>,
    pub hitbox_records_b: Vec<Extents>,
    /// sprite id -> index into `hitbox_records_*`.
    pub hitbox_index_a: Vec<u16>,
    pub hitbox_index_b: Vec<u16>,
    /// Scene-state path records (ROM $905B table).
    pub draw_records: Vec<Extents>,
    pub draw_index: Vec<u16>,
    /// sprite id -> OAM mapping, mode A (168) / mode B (228).
    pub sprite_mappings_a: Vec<Vec<MappingEntry>>,
    pub sprite_mappings_b: Vec<Vec<MappingEntry>>,
    /// anim -> behavior profile (bank $16 per-anim code), mode A (96) /
    /// mode B (112).
    /// The boss handlers' shared step profile (file `$B1A3`, 38 bytes): the
    /// `$B173`-family pickers all read this one table, differing only in the
    /// `c` limit they pass (8/12/15/18/21/24/27/36). Values are 0-3.
    #[serde(default)]
    pub boss_picker_profile: Vec<u8>,
    /// The def-0 boss's sweep profile (file `$9595`, 112 bytes): a 27-byte
    /// 0-3 ramp then zeros, indexed by `B_PHASE` through `$B16B`.
    #[serde(default)]
    pub boss_sweep_profile: Vec<u8>,
    /// anim -> handler for the *boss* path (`$92DF`'s `rst $28` table, bank 2
    /// `$92E8`). Unlike the level profiles (`$58040`/`$59C07`, bank 22), these
    /// entries are **bank-2** addresses — `rst $28` jumps inside the bank that
    /// `$3C94` mapped, which is 2 — so a boss slot must not run the level's
    /// handler for the same anim id, and the port's first extraction of this
    /// table was off by `$50000`. The ROM declares no length; the extractor
    /// keeps the leading run of in-window values (29).
    #[serde(default)]
    pub boss_anim_handlers: Vec<u32>,
    pub anim_profiles_a: Vec<AnimProfile>,
    pub anim_profiles_b: Vec<AnimProfile>,
    /// Player (Mario) draw frames: the bank-1 mapping table at ROM $4000
    /// (242 `dw`) resolved to `gfx/spritemappings.asm` blobs; indexed by the
    /// player draw's `$FFC6` value (see [`crate::player`]).
    pub player_mappings: Vec<Vec<MappingEntry>>,
}

impl ObjectTables {
    /// Sprite id for a spawn record in a level (`anim` = record byte 5).
    pub fn sprite_for(&self, level_id: u8, anim: u8) -> Option<u8> {
        let table = if sprite_map_index(level_id) == 0 {
            &self.anim_to_sprite_a
        } else {
            &self.anim_to_sprite_b
        };
        table.get(anim as usize).copied()
    }

    /// Collision extents for a sprite id in a level's mode.
    pub fn hitbox_for(&self, level_id: u8, sprite: u8) -> Option<Extents> {
        let (records, index) = if sprite_map_index(level_id) == 0 {
            (&self.hitbox_records_a, &self.hitbox_index_a)
        } else {
            (&self.hitbox_records_b, &self.hitbox_index_b)
        };
        let i = *index.get(sprite as usize)? as usize;
        records.get(i).copied()
    }

    /// `$9048`: the sprite -> extent record lookup for the *scene-state* path.
    /// `$81CF` picks `$4737`/`$4935` (the level tables, [`hitbox_for`]) but the
    /// boss driver's `$9048` uses `hl = $505B` unconditionally — the third
    /// table, extracted as `draw_records`/`draw_index`. Both copy four bytes
    /// into `$FFE3-$FFE6` (the port's `B_HB_TOP..B_HB_RIGHT`).
    ///
    /// [`hitbox_for`]: ObjectTables::hitbox_for
    pub fn draw_extent_for(&self, sprite: u8) -> Option<Extents> {
        let i = *self.draw_index.get(sprite as usize)? as usize;
        self.draw_records.get(i).copied()
    }

    /// OAM mapping for a sprite id in a level's mode.
    pub fn mapping_for(&self, level_id: u8, sprite: u8) -> Option<&[MappingEntry]> {
        let table = if sprite_map_index(level_id) == 0 {
            &self.sprite_mappings_a
        } else {
            &self.sprite_mappings_b
        };
        table.get(sprite as usize).map(Vec::as_slice)
    }

    /// Behavior handler ROM offset for an anim in a level's mode.
    pub fn behavior_for(&self, level_id: u8, anim: u8) -> Option<u32> {
        self.profile_for(level_id, anim).map(|p| p.handler)
    }

    /// anim -> handler for the boss (`$9048`/`$92DF`) path.
    pub fn boss_anim_handler(&self, anim: u8) -> Option<u32> {
        self.boss_anim_handlers.get(anim as usize).copied()
    }

    /// Full behavior profile for an anim in a level's mode.
    pub fn profile_for(&self, level_id: u8, anim: u8) -> Option<&AnimProfile> {
        let table = if sprite_map_index(level_id) == 0 {
            &self.anim_profiles_a
        } else {
            &self.anim_profiles_b
        };
        table.get(anim as usize)
    }
}
