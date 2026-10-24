//! Scenery explicitly approved for distribution with the port.

use super::SessionAssets;
use sml2_core::scenery::SceneryOverlay;
use std::collections::BTreeSet;

/// Register approved exports here. Paths are relative to this source file:
/// `(0, include_str!("../../../../assets/scenery/scenery-00.json")),`
/// Keep this empty until scenery is explicitly selected for release.
pub(super) const LEVELS: &[(u8, &str)] = &[];

pub(super) fn apply(assets: &mut SessionAssets, entries: &[(u8, &str)]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut overlays = Vec::with_capacity(entries.len());
    for &(id, json) in entries {
        if id >= 32 {
            return Err(format!("bundled scenery: level {id} is outside 0..32"));
        }
        if !seen.insert(id) {
            return Err(format!("bundled scenery: level {id} is registered twice"));
        }
        let content = assets
            .level(id)
            .map_err(|error| format!("bundled scenery for level {id}: {error}"))?;
        let scenery = serde_json::from_str::<SceneryOverlay>(json)
            .map_err(|error| format!("bundled scenery for level {id}: {error}"))?;
        let scenery = SceneryOverlay::try_from_paints(
            scenery.paints(),
            content.manifest.metatiles.metatiles.len(),
        )
        .map_err(|error| format!("bundled scenery for level {id}: {error}"))?;
        overlays.push((id, scenery));
    }
    // Validate the complete registry before replacing any decoded content.
    for (id, scenery) in overlays {
        let mut content = assets.level(id)?.clone();
        content.scenery = scenery;
        assets.insert_level(id, content);
    }
    Ok(())
}

#[cfg(test)]
#[path = "bundled_scenery_tests.rs"]
mod tests;
