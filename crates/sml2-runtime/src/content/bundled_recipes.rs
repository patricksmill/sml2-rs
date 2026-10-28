//! Expanded-level recipes shipped with the port. A recipe holds only
//! authored operations (`sml2_core::level_recipe`) and applies to the
//! player's own extraction when its `sources` list that layout.

use sml2_core::level_recipe::LevelRecipe;

/// Register recipes here. Paths are relative to this source file.
pub(super) const RECIPES: &[&str] = &[
    // Tree Zone 2 (vanilla and DX share its layout).
    include_str!("../../../../assets/expanded/tree-zone-2.json"),
];

pub(super) fn parse(entries: &[&str]) -> Result<Vec<LevelRecipe>, String> {
    let mut recipes: Vec<LevelRecipe> = Vec::with_capacity(entries.len());
    for json in entries {
        let recipe: LevelRecipe =
            serde_json::from_str(json).map_err(|error| format!("bundled recipe: {error}"))?;
        let id = recipe.level_id;
        if id >= 32 {
            return Err(format!("bundled recipe: level {id} is outside 0..32"));
        }
        if recipe.sources.is_empty() {
            return Err(format!(
                "bundled recipe for level {id}: it must list the layouts it was written for"
            ));
        }
        if recipes
            .iter()
            .any(|r| r.level_id == id && r.sources.iter().any(|s| recipe.sources.contains(s)))
        {
            return Err(format!(
                "bundled recipe for level {id}: two recipes claim one layout"
            ));
        }
        recipes.push(recipe);
    }
    Ok(recipes)
}
