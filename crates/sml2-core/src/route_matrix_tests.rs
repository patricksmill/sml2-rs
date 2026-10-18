//! Route-matrix acceptance for the first slice (levels 12, 19, 28, 29).
//!
//! `tools/route_matrix.py` derives each level's exit cells from the `$4A`/`$4B`
//! metatiles in the decompressed layout.  A matrix row is only accepted once the
//! port actually clears the level from that cell, so these tests load the same
//! extracted level and stand Mario where the `$1BFF` probe samples it.
//!
//! Probe geometry (`PlayerSim::tile` / `$1EFA`): `tile(x+16, y+28)` maps the
//! sample to `((px-8)>>4, (py-16)>>4)`.  `stand_on` inverts that below.

use crate::level::{LevelLayout, LevelManifest};
use crate::player_sim::{PlayerKeys, PlayerSim};
use std::path::PathBuf;

fn load(level: u8) -> (LevelManifest, LevelLayout) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted/levels");
    let manifest: LevelManifest = serde_json::from_str(
        &std::fs::read_to_string(dir.join(format!("level{level:02}.manifest.json")))
            .expect("extracted level manifest (run sml2-extract)"),
    )
    .unwrap();
    let layout = LevelLayout::from_bytes(
        &std::fs::read(dir.join(&manifest.layout.file)).expect("extracted layout"),
    )
    .unwrap();
    (manifest, layout)
}

/// Mario position whose `(x+16, y+28)` probe samples `(col, row)`, biased 8 px
/// into the cell so one frame of gravity cannot move the sample out of it.
fn stand_on(col: usize, row: usize) -> (u16, u16) {
    ((col * 16) as u16 - 8, (row * 16) as u16 - 8)
}

fn assert_bell(level: u8, col: usize, row: usize, tile: u8) {
    let (_manifest, layout) = load(level);
    assert_eq!(
        layout.cells[row * 256 + col],
        tile,
        "level{level:02} source cell ({col},{row})"
    );
    let (x, y) = stand_on(col, row);
    let mut sim = PlayerSim::new();
    sim.x = x;
    sim.y = y;
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert!(
        sim.exit_phase,
        "level{level:02} must arm the exit bell from ({col},{row})"
    );
    assert_eq!(sim.fade_timer, 192, "the exit fade length");
    assert_eq!(sim.music_request, 3, "the level-clear request");
    assert_eq!(
        sim.exit_tile_flag,
        if tile == 0x4B { 0xFF } else { 0 },
        "$4B is the already-cleared twin"
    );
}

#[test]
fn level12_exit_bell_arms_from_the_source_cell() {
    assert_bell(12, 238, 29, 0x4A);
}

/// Level 19 is the slice's two-exit level: the plain `$4A` bell on the floor and
/// the `$4B` twin up the pipe corridor.
#[test]
fn level19_plain_and_alternate_bells_arm_from_their_source_cells() {
    assert_bell(19, 190, 29, 0x4A);
    assert_bell(19, 190, 11, 0x4B);
}

#[test]
fn level28_exit_bell_arms_from_the_source_cell() {
    assert_bell(28, 78, 30, 0x4A);
}

#[test]
fn level29_exit_bell_arms_from_the_source_cell() {
    assert_bell(29, 94, 28, 0x4A);
}

/// Every `$4A`/`$4B` cell in every level must arm the bell from its own cell.
///
/// The source scan finds **35 bells across 25 levels** (7 `$4B` twins, 28 `$4A`
/// bells); the six levels without one end through a boss clear or the castle
/// flow.  The count is pinned so a re-extraction cannot silently shrink the set.
#[test]
fn every_source_exit_cell_arms_the_bell() {
    const EXPECTED_BELLS: usize = 35;
    let mut checked = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for level in 1..=32u8 {
        let (_manifest, layout) = load(level);
        for (idx, &tile) in layout.cells.iter().enumerate() {
            if tile & 0xFE != 0x4A {
                continue;
            }
            let (col, row) = (idx % 256, idx / 256);
            let (x, y) = stand_on(col, row);
            let mut sim = PlayerSim::new();
            sim.x = x;
            sim.y = y;
            sim.step(&PlayerKeys::default(), false, false, &layout);
            checked += 1;
            if !sim.exit_phase {
                failures.push(format!("level{level:02} ({col},{row}) ${tile:02X}: not armed"));
                continue;
            }
            if sim.fade_timer != 192 {
                failures.push(format!("level{level:02} ({col},{row}): fade {}", sim.fade_timer));
            }
            let want = if tile == 0x4B { 0xFF } else { 0 };
            if sim.exit_tile_flag != want {
                failures.push(format!(
                    "level{level:02} ({col},{row}): exit flag {:#04X} != {want:#04X}",
                    sim.exit_tile_flag
                ));
            }
        }
    }
    assert_eq!(checked, EXPECTED_BELLS, "source bell count changed");
    assert!(
        failures.is_empty(),
        "{} of {checked} bell cell(s) failed: {}",
        failures.len(),
        failures.join("; ")
    );
}
