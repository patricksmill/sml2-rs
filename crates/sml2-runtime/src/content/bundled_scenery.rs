//! Scenery explicitly approved for distribution with the port.

use super::SessionAssets;
use sml2_core::scenery::SceneryOverlay;
use std::collections::BTreeSet;

/// Register approved exports here. Paths are relative to this source file:
/// `(0, include_str!("../../../../assets/scenery/scenery-00.json")),`
/// Keep this empty until scenery is explicitly selected for release.
pub(super) const LEVELS: &[(u8, &str)] = &[
    // Tree Zone 1.
    (
        1,
        include_str!("../../../../assets/scenery/scenery-01.json"),
    ),
];

/// Scenery for levels as their expanded-level recipe rebuilds them, in the
/// rebuilt map's cells: `scenery-NN-expanded.json` exports.
pub(super) const EXPANDED: &[(u8, &str)] = &[
    // Tree Zone 2, rebuilt by `assets/expanded/tree-zone-2.json`.
    (
        2,
        include_str!("../../../../assets/scenery/scenery-02-expanded.json"),
    ),
];

pub(super) fn apply(assets: &mut SessionAssets, entries: &[(u8, &str)]) -> Result<(), String> {
    // Validate the complete registry before replacing any decoded content.
    for (id, scenery) in parse(assets, entries, "bundled scenery")? {
        let mut content = assets.level(id)?.clone();
        content.scenery = scenery;
        assets.insert_level(id, content);
    }
    Ok(())
}

/// [`apply`] for [`EXPANDED`]; each level needs a recipe to rebuild it.
pub(super) fn apply_expanded(
    assets: &mut SessionAssets,
    entries: &[(u8, &str)],
) -> Result<(), String> {
    let overlays = parse(assets, entries, "bundled expanded scenery")?;
    if let Some((id, _)) = overlays
        .iter()
        .find(|(id, _)| !assets.recipes.iter().any(|r| r.level_id == *id))
    {
        return Err(format!(
            "bundled expanded scenery: level {id} has no recipe"
        ));
    }
    for (id, scenery) in overlays {
        let mut content = assets.level(id)?.clone();
        content.expanded_scenery = Some(scenery);
        assets.insert_level(id, content);
    }
    Ok(())
}

fn parse(
    assets: &SessionAssets,
    entries: &[(u8, &str)],
    what: &str,
) -> Result<Vec<(u8, SceneryOverlay)>, String> {
    let mut seen = BTreeSet::new();
    let mut overlays = Vec::with_capacity(entries.len());
    for &(id, json) in entries {
        if id >= 32 {
            return Err(format!("{what}: level {id} is outside 0..32"));
        }
        if !seen.insert(id) {
            return Err(format!("{what}: level {id} is registered twice"));
        }
        // A level that failed to load keeps its own error for level entry;
        // its scenery has nothing to decorate.
        let Ok(content) = assets.level(id) else {
            continue;
        };
        let scenery = serde_json::from_str::<SceneryOverlay>(json)
            .map_err(|error| format!("{what} for level {id}: {error}"))?;
        let scenery = SceneryOverlay::try_from_paints(
            scenery.paints(),
            content.manifest.metatiles.metatiles.len(),
        )
        .map_err(|error| format!("{what} for level {id}: {error}"))?;
        overlays.push((id, scenery));
    }
    Ok(overlays)
}

#[cfg(test)]
#[path = "bundled_scenery_tests.rs"]
mod tests;
