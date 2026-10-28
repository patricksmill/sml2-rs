//! Loading boundary for extracted content. Sources are consumed before ticking;
//! sessions hold decoded content and never reopen files on level transitions.

use sml2_core::level::{HudStrip, LevelLayout, LevelManifest};
use sml2_core::map::{
    MapNodeRecords, MapNodes, MapWalk, OverworldManifest, OverworldTilemaps, OverworldTiles,
};
use sml2_core::object::ObjectTables;
use sml2_core::scene::SceneScripts;
use sml2_core::scenery::SceneryOverlay;
use sml2_core::tileset::Tilesets;
use sml2_core::world::World;
use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

mod bundled_scenery;

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
    pub edge_map: Option<Vec<u8>>,
    pub scene_map: Option<Vec<u8>>,
    /// Approved visual scenery, independent of the simulated layout.
    pub scenery: SceneryOverlay,
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
            .filter(|b| b.len() == World::ROM.screens());
        let scene_map = source
            .read(&format!("levels/level{nn:02}.scene.bin"))
            .ok()
            .filter(|b| b.len() == World::ROM.screens());
        Ok(Self {
            manifest,
            layout,
            bg_base,
            objblock,
            edge_map,
            scene_map,
            scenery: SceneryOverlay::empty(),
        })
    }
}

/// All assets required for running both overworld navigation and any level.
pub struct SessionAssets {
    /// Cartridge $0134..=$014B metadata for GBC boot-palette selection.
    pub cartridge_header: [u8; 0x18],
    pub music_bank: Vec<u8>,
    pub nodes: MapNodes,
    pub node_records: MapNodeRecords,
    pub walk: MapWalk,
    pub overworld_tilemaps: OverworldTilemaps,
    pub overworld_manifest: OverworldManifest,
    pub overworld_tiles: OverworldTiles,
    /// Raw ROM blocks used by the Pumpkin and Tree tile animation copies.
    pub zone_animation_tiles: Vec<u8>,
    /// The bank-24 room blobs (`map/castle_scripts.*`).
    pub castle_scripts: sml2_core::castle::CastleScripts,
    /// `ending/scene.json` and `ending/credits.bin`.
    pub ending_scene: Option<sml2_core::ending_scene::EndingAssets>,
    pub credits_text: Vec<u8>,
    /// Bank 25 `$5000` tiles for the bell-bonus rooms (screens 13/14).
    pub bell_room_tiles: sml2_core::map::BellRoomTiles,
    /// `room_common_tiles.bin`: `$3F58`'s bank-6 block at `$8000`.
    pub room_common_tiles: sml2_core::map::RoomCommonTiles,
    /// `castle_reveal_tiles.bin` + `castle_reveal_tilemap.bin`.
    pub castle_reveal: sml2_core::map::CastleRevealTiles,
    /// `$5297`'s sprite table (bank 1 `$4000`), carried inside `castle_scripts`.
    pub sprite_metas: sml2_core::room_sprites::SpriteMetas,
    pub hud: HudStrip,
    pub anim_rows: Vec<u8>,
    pub scripts: SceneScripts,
    pub tilesets: Tilesets,
    pub arena_layout: Option<LevelLayout>,
    pub object_tables: ObjectTables,
    /// Title-screen and file-select VRAM/tilemaps (`frontend/*.bin`).
    pub front_end: sml2_core::front_end::FrontEndAssets,
    /// Approved surroundings for fixed screens, maps, rooms, and boss arenas.
    pub scene_scenery: crate::scene_scenery::SceneScenery,
    levels: std::collections::BTreeMap<u8, Result<LevelContent, String>>,
}

impl SessionAssets {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        Self::load_from(&ExtractedContent::discover())
    }

    pub fn load_from(source: &dyn ContentSource) -> Result<Self, Box<dyn Error>> {
        Self::load_from_with_scenery(source, bundled_scenery::LEVELS)
    }

    fn load_from_with_scenery(
        source: &dyn ContentSource,
        scenery: &[(u8, &str)],
    ) -> Result<Self, Box<dyn Error>> {
        let cartridge_header = load_cartridge_header(source)?;
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
        let sprite_metas = sml2_core::room_sprites::SpriteMetas::new(
            castle_scripts.blob("sprite_metasprites").unwrap_or(&[]),
        );
        let ending_scene = source
            .read_text("ending/scene.json")
            .ok()
            .and_then(|text| sml2_core::ending_scene::EndingAssets::from_json(&text).ok());
        let credits_text = source.read("ending/credits.bin").unwrap_or_default();
        let zone_animation_tiles = source.read("map/zone_animations.bin").unwrap_or_default();
        let bell_room_tiles = load_bell_room_tiles_from(source);
        let room_common_tiles = sml2_core::map::RoomCommonTiles::from_bytes(
            &source.read("map/room_common_tiles.bin").unwrap_or_default(),
        )
        .unwrap_or_default();
        let castle_reveal = sml2_core::map::CastleRevealTiles::from_bytes(
            &source
                .read("map/castle_reveal_tiles.bin")
                .unwrap_or_default(),
            &source
                .read("map/castle_reveal_tilemap.bin")
                .unwrap_or_default(),
        )
        .unwrap_or_default();
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
        let mut assets = Self {
            cartridge_header,
            music_bank,
            nodes,
            node_records,
            walk,
            overworld_tilemaps,
            overworld_manifest,
            overworld_tiles,
            zone_animation_tiles,
            castle_scripts,
            ending_scene,
            credits_text,
            bell_room_tiles,
            room_common_tiles,
            castle_reveal,
            sprite_metas,
            hud,
            anim_rows,
            scripts,
            tilesets,
            arena_layout,
            object_tables,
            front_end,
            scene_scenery: crate::scene_scenery::SceneScenery::bundled()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
            levels,
        };
        bundled_scenery::apply(&mut assets, scenery)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        Ok(assets)
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

// Legacy extracted trees predate cartridge_header.bin. This is the exact
// supported baserom.gb $0134..=$014B header (MARIOLAND2, title sum $C9, Nintendo
// old licensee $01), used only when metadata is absent. Malformed metadata and
// all other read errors remain errors; no ROM is opened by the runtime.
const LEGACY_SML2_HEADER: [u8; 0x18] = [
    0x4D, 0x41, 0x52, 0x49, 0x4F, 0x4C, 0x41, 0x4E, 0x44, 0x32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 4, 2,
    1, 1,
];

fn load_cartridge_header(source: &dyn ContentSource) -> io::Result<[u8; 0x18]> {
    match source.read("cartridge_header.bin") {
        Ok(bytes) => bytes.try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "cartridge_header.bin must contain 24 bytes ($0134..=$014B)",
            )
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(LEGACY_SML2_HEADER),
        Err(error) => Err(error),
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
        Some(mut s) => {
            match source
                .read_text("scenes/transfers.json")
                .map_err(|e| e.to_string())
                .and_then(|text| sml2_core::scene::SceneTransferAssets::from_json(&text))
            {
                Ok(assets) => s.set_transfer_assets(assets),
                Err(error) => println!("note: scene transfer graphics missing — {error}"),
            }
            s
        }
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
