//! Collision semantics of the layout's metatile ids.
//!
//! Collision is **id-range based and global to the whole game** — there is no
//! per-level collision table. The engine (`home.asm`) samples the `$B000`
//! layout at probe points around Mario (sampler `$1EFA`: address
//! `$B000 + ((y-16)>>4)*$100 + ((x-8)>>4)`, result kept in `sCollisionTile`
//! `$A20D`) and compares the raw id against fixed thresholds and ranges.
//! Decoded consumers, all live-verified against PyBoy (2026-10-11, level 0:
//! overwriting layout cells and observing movement — see the WP notes):
//!
//! * `< $38` — **solid**: blocks horizontal movement (wall probes
//!   `$1E7C`/`$1E8D` compare `cp $38`, push-back at `$188B` zeroes H speed)
//!   and is standable (floor probes `$1E36`/`$1E3B` sample three rows below
//!   Mario, `cp $48`). Verified: `$27` (Tree Zone 1 fill), `$2B` and `$37`
//!   all block walking and all stand; `$38` does not block.
//! * `$38-$47` — **slope/platform**: standable (id `< $48`) but never blocks
//!   movement (`>= $38`). In real levels these ids form diagonal runs
//!   (level 4) and horizontal platform runs (level 8). Verified for `$38`
//!   and `$47` (stand: yes, block: no).
//! * `$48/$49` — **coin**: collected on touch (`$1D75`: `and $FE; cp $48` ->
//!   sfx 7, BCD coin-counters). Verified: `$48` does not stand/block.
//! * `$4A/$4B` — **exit**: level clear (`and $FE; cp $4A` -> fanfare,
//!   `$FF9B = 24`, fade).
//! * `$4C-$4F` — **water**: enables swimming (`$1C82`: `and $FC; cp $4C` ->
//!   `sSwimming = $FF`; the water movement tables). Verified: `$50` at the
//!   same probe kills while `$4C-$4F` is the swim class by code.
//! * `$50/$51` — **hazard**: kills Mario (`and $FE; cp $50` -> death state
//!   9). Verified live (die flag set, state 9).
//! * `$52-$57` — **moon / low gravity**: sets the `sMoonPhysics` bit
//!   (`cp $52` .. `cp $58` window).
//! * `$58-$7F` — **passable decor** (air, backgrounds, structures). `$7E`
//!   is additionally the spin-jump tile (`$1DFE`), and the surface row ids
//!   like `$7A` are decor drawn over the solid fill below them. Verified:
//!   `$60`/`$7A` neither block nor stand.
//!
//! Within the solid range, dedicated consumers handle sub-specials: pipes
//! `$05` and `$08-$0F` (`$188B`/`$1432`: walk-into entry, direction encoded
//! in the id's low bits), fireball breakables `$06` (`$3386`), spin bounce
//! `$04/$05` (`$129F` area). `$48/$49` coins double as the walk-over collect
//! and are checked by the same touch probes that test walls.

use serde::{Deserialize, Serialize};

/// Collision class of one layout cell / metatile id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TileClass {
    /// Blocks movement and is standable (`id <= $37`).
    Solid,
    /// Standable, never blocks; slopes and platform tops (`$38-$47`).
    Slope,
    /// Collectible coin (`$48/$49`).
    Coin,
    /// Level exit (`$4A/$4B`).
    Exit,
    /// Water; enables swimming (`$4C-$4F`).
    Water,
    /// Deadly tile (`$50/$51`).
    Hazard,
    /// Low-gravity ("moon") region (`$52-$57`).
    Moon,
    /// Passable decor / air / background (`$58-$7F`).
    Decor,
}

/// Classify a layout metatile id (0..128) by the engine's global id ranges.
pub fn classify(id: u8) -> TileClass {
    match id {
        0x00..=0x37 => TileClass::Solid,
        0x38..=0x47 => TileClass::Slope,
        0x48..=0x49 => TileClass::Coin,
        0x4A..=0x4B => TileClass::Exit,
        0x4C..=0x4F => TileClass::Water,
        0x50..=0x51 => TileClass::Hazard,
        0x52..=0x57 => TileClass::Moon,
        _ => TileClass::Decor,
    }
}

/// True when the id blocks horizontal movement (wall probes `cp $38`).
pub fn blocks_movement(id: u8) -> bool {
    classify(id) == TileClass::Solid
}

/// True when the id is standable (floor probes `cp $48`).
pub fn standable(id: u8) -> bool {
    matches!(classify(id), TileClass::Solid | TileClass::Slope)
}
