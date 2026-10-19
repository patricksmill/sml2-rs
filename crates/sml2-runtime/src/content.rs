//! Loading boundary for extracted content. Sources are consumed before ticking;
//! sessions hold decoded content and never reopen files on level transitions.

use sml2_core::level::{HudStrip, LevelLayout, LevelManifest};
use sml2_core::map::{
    MapNodeRecords, MapNodes, MapWalk, OverworldManifest, OverworldTilemaps, OverworldTiles,
};
use sml2_core::object::ObjectTables;
use sml2_core::scene::SceneScripts;
use sml2_core::tileset::Tilesets;
use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

/// Relative asset reads, implemented by extracted directories or in-memory bundles.
/// No source is retained by a session after loading.
pub trait ContentSource {
    fn read(&self, relative: &str) -> io::Result<Vec<u8>>;

    fn read_text(&self, relative: &str) -> io::Result<String> {
        String::from_utf8(self.read(relative)?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

pub struct ExtractedContent {
    root: PathBuf,
}

impl ExtractedContent {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn discover() -> Self {
        Self::new(extracted_dir())
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl ContentSource for ExtractedContent {
    fn read(&self, relative: &str) -> io::Result<Vec<u8>> {
        fs::read(self.root.join(relative))
    }
}

#[derive(Clone)]
pub struct LevelContent {
    pub manifest: LevelManifest,
    pub layout: LevelLayout,
    pub bg_base: Vec<u8>,
    pub objblock: Vec<u8>,
    pub edge_map: Option<[u8; 48]>,
    pub scene_map: Option<Vec<u8>>,
}

impl LevelContent {
    pub fn load_from(source: &dyn ContentSource, id: u8) -> Result<Self, Box<dyn Error>> {
        let nn = usize::from(id) + 1;
        let manifest: LevelManifest = serde_json::from_str(
            &source.read_text(&format!("levels/level{nn:02}.manifest.json"))?,
        )?;
        let layout =
            LevelLayout::from_bytes(&source.read(&format!("levels/{}", manifest.layout.file))?)?;
        let bg_base = source.read(&format!("levels/level{nn:02}.bgblock.bin"))?;
        let objblock = source.read(&format!("levels/level{nn:02}.objblock.bin"))?;
        let edge_map = source
            .read(&format!("levels/level{nn:02}.edge.bin"))
            .ok()
            .and_then(|b| b.try_into().ok());
        let scene_map = source
            .read(&format!("levels/level{nn:02}.scene.bin"))
            .ok()
            .filter(|b| b.len() == 48);
        Ok(Self {
            manifest,
            layout,
            bg_base,
            objblock,
            edge_map,
            scene_map,
        })
    }
}

/// All assets required for running both overworld navigation and any level.
pub struct SessionAssets {
    pub music_bank: Vec<u8>,
    pub nodes: MapNodes,
    pub node_records: MapNodeRecords,
    pub walk: MapWalk,
    pub overworld_tilemaps: OverworldTilemaps,
    pub overworld_manifest: OverworldManifest,
    pub overworld_tiles: OverworldTiles,
    /// The bank-24 room blobs (`map/castle_scripts.*`).
    pub castle_scripts: sml2_core::castle::CastleScripts,
    /// Bank 25 `$5000` tiles for the bell-bonus rooms (screens 13/14).
    pub bell_room_tiles: sml2_core::map::BellRoomTiles,
    pub hud: HudStrip,
    pub anim_rows: Vec<u8>,
    pub scripts: SceneScripts,
    pub tilesets: Tilesets,
    pub arena_layout: Option<LevelLayout>,
    pub object_tables: ObjectTables,
    /// Title-screen and file-select VRAM/tilemaps (`frontend/*.bin`).
    pub front_end: sml2_core::front_end::FrontEndAssets,
    levels: std::collections::BTreeMap<u8, Result<LevelContent, String>>,
}

impl SessionAssets {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        Self::load_from(&ExtractedContent::discover())
    }

    pub fn load_from(source: &dyn ContentSource) -> Result<Self, Box<dyn Error>> {
        let music_bank = source.read("audio/music_bank04.bin")?;
        if music_bank.len() != 0x4000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "audio/music_bank04.bin must contain 16384 bytes",
            )
            .into());
        }
        let nodes = load_map_nodes_from(source);
        let node_records = load_map_node_records_from(source);
        let walk = load_map_walk_from(source);
        let overworld_tilemaps = load_overworld_tilemaps_from(source);
        let overworld_manifest = load_overworld_manifest_from(source);
        let overworld_tiles = load_overworld_tiles_from(source);
        let castle_scripts = load_castle_scripts_from(source);
        let bell_room_tiles = load_bell_room_tiles_from(source);
        let hud: HudStrip = serde_json::from_str(&source.read_text("hud.json")?)?;
        let anim_rows = source.read("tileanim.bin")?;
        let scripts = load_scene_scripts_from(source);
        let tilesets = load_tilesets_from(source);
        let arena_layout = load_arena_layout_from(source);
        let object_tables: ObjectTables = serde_json::from_str(&source.read_text("objects.json")?)?;
        let front_end = sml2_core::front_end::FrontEndAssets::new(
            source.read("frontend/title_vram.bin")?,
            source.read("frontend/title_tilemap.bin")?,
            source.read("frontend/file_select_vram.bin")?,
            source.read("frontend/file_select_tilemap.bin")?,
            source.read("frontend/demo_tapes.bin")?,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Cache successes and failures: preserve per-level error reporting at
        // entry, while ensuring a tick never reads from the source.
        let levels = (0..32)
            .map(|id| {
                (
                    id,
                    LevelContent::load_from(source, id).map_err(|e| e.to_string()),
                )
            })
            .collect();
        Ok(Self {
            music_bank,
            nodes,
            node_records,
            walk,
            overworld_tilemaps,
            overworld_manifest,
            overworld_tiles,
            castle_scripts,
            bell_room_tiles,
            hud,
            anim_rows,
            scripts,
            tilesets,
            arena_layout,
            object_tables,
            front_end,
            levels,
        })
    }
    /// Immutable content prepared before simulation starts.
    pub fn level(&self, id: u8) -> Result<&LevelContent, String> {
        self.levels
            .get(&id)
            .ok_or_else(|| format!("level {id} is not loaded"))?
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Replace one level before constructing a session (e.g. editor playtest).
    pub fn insert_level(&mut self, id: u8, level: LevelContent) {
        self.levels.insert(id, Ok(level));
    }
}

/// Locate `extracted/` whether run from the repo root or a crate dir.
pub(crate) fn extracted_dir() -> PathBuf {
    for cand in [
        "extracted",
        "../extracted",
        "../../extracted",
        "sml2-rs/extracted",
    ] {
        let p = PathBuf::from(cand);
        if p.join("levels").is_dir() {
            return p;
        }
    }
    PathBuf::from("extracted")
}

pub(crate) fn load_map_nodes() -> MapNodes {
    load_map_nodes_from(&ExtractedContent::discover())
}

pub(crate) fn load_scene_scripts() -> SceneScripts {
    load_scene_scripts_from(&ExtractedContent::discover())
}

pub(crate) fn load_tilesets() -> Tilesets {
    load_tilesets_from(&ExtractedContent::discover())
}

pub(crate) fn load_arena_layout() -> Option<LevelLayout> {
    load_arena_layout_from(&ExtractedContent::discover())
}

/// The overworld's node tables (`extracted/map/nodes.bin`). One ROM-wide blob,
/// loaded like the scene scripts, and flagged when absent for the same reason:
/// with no table the hand-off cannot index `sLevelProgress`, so it would silently
/// stop marking progress.
pub(crate) fn load_map_nodes_from(source: &dyn ContentSource) -> MapNodes {
    match source
        .read("map/nodes.bin")
        .ok()
        .as_deref()
        .and_then(MapNodes::from_bytes)
    {
        Some(n) => n,
        None => {
            println!("note: map node tables missing — the hand-off cannot mark progress");
            MapNodes::default()
        }
    }
}

pub(crate) fn load_map_walk_from(source: &dyn ContentSource) -> MapWalk {
    match source
        .read("map/walk.bin")
        .ok()
        .as_deref()
        .and_then(MapWalk::from_bytes)
    {
        Some(w) => w,
        None => {
            println!("note: map walk table missing — overworld walk streams will be empty");
            MapWalk::default()
        }
    }
}

pub(crate) fn load_map_node_records_from(source: &dyn ContentSource) -> MapNodeRecords {
    match source
        .read("map/node_records.bin")
        .ok()
        .as_deref()
        .and_then(MapNodeRecords::from_bytes)
    {
        Some(r) => r,
        None => {
            println!("note: map node records missing");
            MapNodeRecords::default()
        }
    }
}

pub(crate) fn load_overworld_tilemaps_from(
    source: &dyn ContentSource,
) -> sml2_core::map::OverworldTilemaps {
    match source
        .read("map/tilemaps.bin")
        .ok()
        .as_deref()
        .and_then(sml2_core::map::OverworldTilemaps::from_bytes)
    {
        Some(tm) => tm,
        None => {
            println!("note: map tilemaps missing");
            sml2_core::map::OverworldTilemaps::default()
        }
    }
}

pub(crate) fn load_overworld_manifest_from(
    source: &dyn ContentSource,
) -> sml2_core::map::OverworldManifest {
    match source
        .read_text("map/route_patches.json")
        .ok()
        .and_then(|s| sml2_core::map::OverworldManifest::from_json_str(&s).ok())
    {
        Some(m) => m,
        None => {
            println!("note: map route_patches.json missing");
            sml2_core::map::OverworldManifest::default()
        }
    }
}

pub(crate) fn load_overworld_tiles_from(
    source: &dyn ContentSource,
) -> sml2_core::map::OverworldTiles {
    match source
        .read("map/tiles.bin")
        .ok()
        .as_deref()
        .and_then(sml2_core::map::OverworldTiles::from_bytes)
    {
        Some(t) => t,
        None => {
            println!("note: map tiles.bin missing");
            sml2_core::map::OverworldTiles::default()
        }
    }
}

pub(crate) fn load_castle_scripts_from(
    source: &dyn ContentSource,
) -> sml2_core::castle::CastleScripts {
    let bytes = source.read("map/castle_scripts.bin").ok();
    let manifest = source.read_text("map/castle_scripts.json").ok();
    match (bytes, manifest) {
        (Some(bytes), Some(manifest)) => {
            match sml2_core::castle::CastleScripts::from_parts(bytes, &manifest) {
                Ok(s) => s,
                Err(e) => {
                    println!("note: map castle_scripts: {e}");
                    sml2_core::castle::CastleScripts::default()
                }
            }
        }
        _ => {
            println!("note: map castle_scripts missing");
            sml2_core::castle::CastleScripts::default()
        }
    }
}

pub(crate) fn load_bell_room_tiles_from(
    source: &dyn ContentSource,
) -> sml2_core::map::BellRoomTiles {
    match source
        .read("map/bell_room_tiles.bin")
        .ok()
        .as_deref()
        .and_then(sml2_core::map::BellRoomTiles::from_bytes)
    {
        Some(t) => t,
        None => {
            println!("note: map bell_room_tiles.bin missing");
            sml2_core::map::BellRoomTiles::default()
        }
    }
}

/// The bank-5 scene script table (`extracted/scenes/script_table.bin`). It is
/// one ROM-wide blob rather than per level, and the missing-file flag matches
/// the two level maps: with no table the scene VM is inert, and the run says so
/// instead of quietly never running a script.
pub(crate) fn load_scene_scripts_from(source: &dyn ContentSource) -> SceneScripts {
    match source
        .read("scenes/script_table.bin")
        .ok()
        .as_deref()
        .and_then(SceneScripts::from_bytes)
    {
        Some(s) => s,
        None => {
            println!("note: scene script table missing — no scene script can run");
            SceneScripts::default()
        }
    }
}

/// The `$6x` opcode's tileset swaps (`extracted/scenes/tilesets.json`). Also one
/// ROM-wide table — `$25AF`'s three sources are indexed only by
/// `sLevelGfxIndex`. Without it an arena keeps the level's own tiles and
/// metatile definitions, which draws the right *layout* with the wrong art.
pub(crate) fn load_tilesets_from(source: &dyn ContentSource) -> Tilesets {
    match source
        .read_text("scenes/tilesets.json")
        .ok()
        .and_then(|t| Tilesets::from_json(&t).ok())
    {
        Some(t) => t,
        None => {
            println!("note: tileset table missing — a scene's $6x swap cannot run");
            Tilesets::default()
        }
    }
}

/// The `$6x` reload's level data (`extracted/scenes/arena_layout.bin`).
///
/// `$25AF` hardcodes `sLevelLayoutIndex = 3` and `sLevelBank = 19`, so the same
/// blob loads for **every** arena: the boss rooms are authored as screens of
/// that one layout and the per-arena look comes from the tileset swap. A level's
/// own layout at those cells holds that level's tiles, which is why an arena
/// rendered from it comes out as ordinary level walls.
pub(crate) fn load_arena_layout_from(source: &dyn ContentSource) -> Option<LevelLayout> {
    match source.read("scenes/arena_layout.bin") {
        Ok(b) => match LevelLayout::from_bytes(&b) {
            Ok(l) => Some(l),
            Err(e) => {
                println!("note: scenes/arena_layout.bin unusable — {e}");
                None
            }
        },
        Err(_) => {
            println!(
                "note: scenes/arena_layout.bin missing — a $6x swap cannot reload the arena room"
            );
            None
        }
    }
}
