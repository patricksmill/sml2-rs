//! Output construction shared by the development extractor and ROM importer.
//! Source discovery and cross-checks remain in each import path.

use sml2_core::level::{
    HudStrip, IndexLevel, IndexTilesetMap, LayoutSummary, LevelLayout, LevelManifest, LevelsIndex,
    ObjectStream, SpawnRecord, INDEX_SCHEMA, LAYOUT_HEIGHT, LAYOUT_SIZE, LAYOUT_WIDTH,
};
use sml2_core::tileset::{TilesetEntry, Tilesets, TILESET_SCHEMA};

pub fn tilesets(entries: Vec<TilesetEntry>, extra_8e80: &[u8]) -> Tilesets {
    Tilesets {
        schema: TILESET_SCHEMA.to_string(),
        notes: vec![
            "$25AF (the $6x scene opcode's handler) swaps the level's graphics and metatile \
             definitions in place: bank 7 $7800 -> VRAM $8E80 (0x50, index-independent), \
             bank $268B[i*2] $2699[i*4] -> $9200 (0x600), bank $268B[i*2+1] $2699[i*4+2] -> \
             $8B00 (a literal 0x400, which overruns the 0x380-byte enemy region by 0x20 and \
             lands on the first two tiles of the $8E80 block), and $26B5[i] -> sMetatileMap."
                .to_string(),
            "The copies land inside the $8800-$97FF tile block the port holds as \
             levelNN.bgblock.bin ($8E80 = 0x680, $9200 = 0xA00, $8B00 = 0x300), and the OBJ \
             space $8000-$8FFF shares that VRAM ($8800-$8FFF = objblock.bin's high half), so \
             applying a swap repatches both."
                .to_string(),
        ],
        extra_8e80: extra_8e80.to_vec(),
        entries,
    }
}

pub fn hud(template: &[u8]) -> HudStrip {
    HudStrip {
        template: template.to_vec(),
        window_x: 7,
        window_y: 136,
        digit_columns: vec![2, 3, 7, 8, 9, 13, 14, 17, 18, 19],
    }
}

pub fn layout_summary(file: String, layout: &LevelLayout) -> LayoutSummary {
    LayoutSummary {
        file,
        width: LAYOUT_WIDTH as u16,
        height: LAYOUT_HEIGHT as u16,
        size: LAYOUT_SIZE as u32,
        distinct_metatile_ids: layout.distinct_metatile_ids(),
        most_common: layout.most_common_metatile(),
    }
}

pub fn object_stream(
    stream_rom: u32,
    packed_len: usize,
    records: Vec<SpawnRecord>,
) -> ObjectStream {
    ObjectStream {
        table_label: "UnknownData_0xE037".to_string(),
        table_bank: 0x03,
        stream_rom,
        packed_len: packed_len as u32,
        records,
    }
}

pub fn index_level(manifest: &LevelManifest) -> IndexLevel {
    IndexLevel {
        level_id: manifest.level_id,
        header_label: manifest.header_label.clone(),
        tileset_map_index: manifest.tileset_map.index,
        graphics_file: manifest.graphics.file.clone(),
        layout_file: manifest.layout.file.clone(),
        spawn_count: manifest.spawns.records.len() as u32,
    }
}

pub fn levels_index(
    levels: Vec<IndexLevel>,
    map_used_by: &[Vec<u8>],
    boss_map_usage: &[(u8, u8)],
) -> LevelsIndex {
    let tileset_maps = map_used_by
        .iter()
        .enumerate()
        .skip(1)
        .map(|(n, used_by)| IndexTilesetMap {
            index: n as u8,
            file: format!("tilemaps/mappings/{n:02}.bin"),
            used_by_levels: used_by.clone(),
            boss_gfx_indices: boss_map_usage
                .iter()
                .filter(|(m, _)| *m as usize == n)
                .map(|(_, g)| *g)
                .collect(),
        })
        .collect();
    LevelsIndex {
        schema: INDEX_SCHEMA.to_string(),
        levels,
        tileset_maps,
        notes: vec![
            "tile animation: each level's manifest carries tile_anim {table_value, \
             enabled, row, speed} from UnknownData_0x35EA ($35EA, 32 bytes; the \
             engine stores value+1 in sTileAnimConfig, 0 = off). While enabled, \
             the 4 VRAM tiles at $9200 are replaced by 64 bytes from \
             extracted/tileanim.bin at row*$100 + phase*$40 (phases 0..3) every \
             time sTileAnimTimer reaches speed*2. Phase 0 equals the static \
             tileset art."
                .to_string(),
            "HUD: extracted/hud.json is the in-level status bar — a 20-tile row \
             (template from ROM $07EA, copied to the window map $9C00 by $07DB) \
             drawn by the window layer at rWY=136/rWX=7 (bottom 8px of screen). \
             $076B patches digit tiles ($80 + value) each frame at columns \
             2-3 (lives), 7-9 (coins), 13-14 (money bags), 17-19 (time)."
                .to_string(),
            "OBJ tiles: levelNN.objblock.bin is the $8000-$8FFF object tile \
             space (4 KB) — low half $8000-$87FF = GFX_Mario / GFX_MarioDark / \
             GFX_MarioMoon picked by the level gfx config's low nibble \
             (LoadMarioGFX $0669: 0/1/else), high half $8800-$8FFF = the same \
             bytes as bgblock.bin[0..$800]. OAM mapping tile ids index this \
             block directly ($8000 + t*16)."
                .to_string(),
            "metatile = [top-left, top-right, bottom-left, bottom-right] tile numbers; \
             a tile id < 128 reads VRAM $9000 + t*16, >= 128 reads $8800 + (t-128)*16 \
             (LCDC bit 4 = 0). Proof: ScrollLevelMap ($0AFB) writes the staged tile \
             bytes to dest, dest+1, dest+$20, dest+$21."
                .to_string(),
            "layout = 256x48 metatile ids, row-major, row stride 256 bytes; \
             Read_Level_Data decompresses exactly $3000 bytes to $B000-$DFFF; \
             block pipelines address it as $B000 + row*$100 + col. \
             levelNN.decompressed.bin is relative to this index."
                .to_string(),
            "screen top-left = (camera_x - 80, camera_y - 72); live-verified against \
             PyBoy screenshots at 94-97% exact pixels (remainder = sprites)."
                .to_string(),
            "tileset maps 25-28 are not referenced by level headers; the boss/special \
             GFX path ($25AF, sLevelGfxIndex via the $26B5 pointer table) selects \
             them (see boss_gfx_indices)."
                .to_string(),
            "spawns: packed bank-$03 streams (UnknownData_0xE037, indexed by sCurLevel) \
             expanded by $EBBB into 6-byte records at $AB06: [0/1] world X (hi, 8-aligned \
             lo), [2] mode flag (0 = spawns always, 1 = skipped in Easy Mode; the game \
             writes 2 once spawned), [3/4] world Y (hi, 8-aligned lo), [5] object/anim \
             index. Records are sorted by X; the scan ($8C31) matches them against the \
             camera-driven cursor, and $EBB6 writes $FF $FF after the last record."
                .to_string(),
            "collision: each level's metatile_classes[128] gives every id's \
             engine collision class (solid < $38 blocks+stands; slope $38-$47 \
             stands, never blocks; coin $48/$49; exit $4A/$4B; water $4C-$4F; \
             hazard $50/$51; moon $52-$57; decor $58+). The ranges are global, \
             decoded from the home.asm probes and live-verified (see \
             sml2-core/src/collision.rs)."
                .to_string(),
            "objects: each spawn record carries sprite_id (anim -> sprite table \
             $8B61/$8BC1 by sprite_map_index) and its hitbox [x0,x1,y0,y1]; the \
             full tables (anim maps, extent records, OAM sprite mappings) live in \
             extracted/objects.json, and each level's whole sprite set is drawn to \
             renders/levelNN.sprites.png (16 cells per row, one per sprite id; \
             ids the level never spawns are omitted). objects.json also carries \
             anim_profiles_a/b: per-anim behavior handler ROM offsets (bank $16) \
             plus static scans of each handler's type byte(s) ($FFE2) and its \
             rst-dispatch sub-state code addresses (live-verified: the level-0 \
             walker's handler runs every other frame while active)."
                .to_string(),
            "renders: extracted/renders/levelNN.png is the full 4096x768 world render \
             (16px per metatile, header BGP palette; animated tiles show their static \
             base) and levelNN.metatiles.png the 128 definitions; raw per-file tile \
             sheets live under extracted/gfx/."
                .to_string(),
            "generated by: cargo run -p sml2-extract (from the sml2-rs repo root; \
             reads SML2_DECOMP_ROOT / ../sml2decomp)"
                .to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_keeps_unused_maps_and_boss_only_usage_in_table_order() {
        let index = levels_index(
            Vec::new(),
            &[vec![99], vec![0, 3], Vec::new(), Vec::new()],
            &[(3, 6), (2, 4), (3, 1)],
        );
        assert_eq!(index.schema, INDEX_SCHEMA);
        assert_eq!(index.tileset_maps.len(), 3);
        assert_eq!(
            index.tileset_maps[0],
            IndexTilesetMap {
                index: 1,
                file: "tilemaps/mappings/01.bin".into(),
                used_by_levels: vec![0, 3],
                boss_gfx_indices: Vec::new(),
            }
        );
        assert!(index.tileset_maps[1].used_by_levels.is_empty());
        assert_eq!(index.tileset_maps[1].boss_gfx_indices, [4]);
        assert_eq!(index.tileset_maps[2].boss_gfx_indices, [6, 1]);
    }
}
