//! DX presentation of any level. Rendering never advances native gameplay.
use super::{hud_row_status, window_coord, window_origin, HudDigits};
use crate::session::{LevelContext, Session, SessionState};
use sml2_core::{
    character::Character,
    level::{HudStrip, TileAnim, LAYOUT_HEIGHT, LAYOUT_WIDTH},
    player::{self, Cell, CellStyle},
    tileset::{BG_TILES_OFFSET, EXTRA_OFFSET, OBJ_TILES_OFFSET},
    video::{
        self,
        cgb::{self, BackgroundPixel, CgbAttributes, CgbPalettes, IndexedPixel, ObjectPixel},
    },
};
use sml2_dx::{LevelGraphics, Presentation};

impl Session {
    /// The level a DX frame would draw: live play or an attract demo, outside
    /// boss arenas and fights. DX's `$36:$7306` gives each arena its own
    /// palettes, VRAM1 graphics and attribute map, and its boss sprites go
    /// through their own draw path; neither is imported yet.
    fn dx_level(&self) -> Option<&LevelContext> {
        let ctx = match &self.state {
            SessionState::InLevel(ctx) => ctx,
            SessionState::Demo(demo) => &demo.ctx,
            _ => return None,
        };
        let boss = ctx.cam.boss_fight_phase != 0
            || ctx.spr.as_ref().is_some_and(|s| !s.boss_oam.is_empty());
        (!ctx.arena_loaded && !boss).then_some(ctx.as_ref())
    }

    /// Whether DX presentation covers the current screen: every level, live
    /// or in a demo, except boss arenas. Title, map, arenas and other screens
    /// keep their native picture.
    pub fn dx_supported(&self) -> bool {
        self.dx_level().is_some()
    }

    /// Render the current level in direct CGB RGB888 color, drawing the player
    /// with `character`'s graphics. Physics stay native either way.
    pub fn render_dx_frame(
        &self,
        dx: &Presentation,
        character: Character,
    ) -> Result<Vec<[u8; 3]>, String> {
        let ctx = self
            .dx_level()
            .ok_or("DX presentation requires an active level")?;
        render(ctx, dx, character)
    }
}

/// One ROM source a level can copy into VRAM0 after loading: its destination
/// in the `$8800` block, base bytes as the native session holds them, and the
/// address both ROMs keep them at.
struct Source<'a> {
    destination: usize,
    bank: u8,
    address: u16,
    base: &'a [u8],
}

/// Every mid-level copy the native session can make: `$25AF`'s arena
/// reloads and `$Bx` scene transfers.
fn mid_level_sources(ctx: &LevelContext) -> Vec<Source<'_>> {
    let mut sources = Vec::new();
    for entry in &ctx.tilesets.entries {
        sources.push(Source {
            destination: BG_TILES_OFFSET,
            bank: entry.bg_bank,
            address: entry.bg_addr,
            base: &entry.bg_tiles,
        });
        sources.push(Source {
            destination: OBJ_TILES_OFFSET,
            bank: entry.obj_bank,
            address: entry.obj_addr,
            base: &entry.obj_tiles,
        });
    }
    sources.push(Source {
        destination: EXTRA_OFFSET,
        bank: 0x07,
        address: 0x7800,
        base: &ctx.tilesets.extra_8e80,
    });
    for block in ctx.scripts.transfer_blocks() {
        // `$Bx` sends a block to `$9200` or `$8B00` by its opcode.
        for destination in [BG_TILES_OFFSET, OBJ_TILES_OFFSET] {
            sources.push(Source {
                destination,
                bank: block.bank,
                address: block.address,
                base: &block.bytes,
            });
        }
    }
    sources
}

/// DX VRAM0 `$8800..$97FF` for this frame. Tiles the native session still
/// holds from the level's load come from DX's own load. DX patches the base
/// game's graphics in place, so a tile that animation, an arena reload or a
/// scene transfer rewrote is found by its native source and read from the
/// same address in the DX banks. A tile with no known source keeps its
/// native bytes.
fn live_background(ctx: &LevelContext, dx: &Presentation, level: &LevelGraphics) -> Vec<u8> {
    const TILE: usize = 16;
    let mut out = level.background[0].clone();
    let (base, live) = (&ctx.bg_base, &ctx.bg_live);
    if base.len() != out.len() || live.len() != out.len() {
        return out;
    }
    let changed = |slot: usize| live[slot * TILE..][..TILE] != base[slot * TILE..][..TILE];
    let mut explained = vec![false; out.len() / TILE];
    // Whole copies first, largest first, so tile animation on top wins.
    let mut sources = if base == live {
        Vec::new()
    } else {
        mid_level_sources(ctx)
    };
    sources.sort_by_key(|s| std::cmp::Reverse(s.base.len()));
    for source in &sources {
        let slots = source.base.len() / TILE;
        let first = source.destination / TILE;
        let fits = |slot: usize| {
            live.get(slot * TILE..slot * TILE + TILE)
                == source
                    .base
                    .get((slot - first) * TILE..(slot - first + 1) * TILE)
        };
        let matched: Vec<usize> = (first..first + slots).filter(|&s| fits(s)).collect();
        // A copy is evident when most of its tiles are present and some of
        // them are not the level's own.
        if matched.len() * 4 < slots * 3 || !matched.iter().any(|&s| changed(s)) {
            continue;
        }
        let Some(bytes) = dx.rom(source.bank, source.address, source.base.len()) else {
            continue;
        };
        for slot in matched {
            if !explained[slot] {
                let at = (slot - first) * TILE;
                out[slot * TILE..slot * TILE + TILE].copy_from_slice(&bytes[at..at + TILE]);
                explained[slot] = true;
            }
        }
    }
    // `$3873` copies one `$40`-byte phase of the configured row to `$9200`.
    // The config can change after the last copy (the goal does), so any row
    // may still be showing; the configured row and the clock's last copied
    // phase win ties, since some rows repeat a phase in the base art only.
    let anim = TileAnim::from_table_value(ctx.sim.tile_anim_config.wrapping_sub(1));
    let region = BG_TILES_OFFSET..BG_TILES_OFFSET + 0x40;
    if ctx.tile_animation_copied {
        let clock = usize::from(ctx.tile_animation.phase & 3);
        let rows = std::iter::once(usize::from(anim.row)).chain(0..16);
        let phases = |row| {
            std::iter::once(clock)
                .chain(0..4)
                .map(move |p| row * 0x100 + p * 0x40)
        };
        let shown = rows
            .flat_map(phases)
            .find(|&at| ctx.anim_rows.get(at..at + 0x40) == Some(&live[region.clone()]));
        if let Some(bytes) = shown.and_then(|at| dx.animation_rows.get(at..at + 0x40)) {
            out[region.clone()].copy_from_slice(bytes);
            explained[region.start / TILE..region.end / TILE].fill(true);
        }
    }
    for slot in (0..explained.len()).filter(|&s| !explained[s] && changed(s)) {
        out[slot * TILE..slot * TILE + TILE]
            .copy_from_slice(&live[slot * TILE..slot * TILE + TILE]);
    }
    out
}

fn pause_palettes(bytes: &[u8], bright_phase: bool) -> Vec<u8> {
    let subtract = if bright_phase { [18, 8, 4] } else { [8, 4, 4] };
    bytes
        .chunks_exact(2)
        .flat_map(|b| {
            let value = u16::from_le_bytes([b[0], b[1]]);
            let mut result = 0u16;
            for (channel, amount) in subtract.into_iter().enumerate() {
                result |= ((value >> (channel * 5)) & 31).saturating_sub(amount) << (channel * 5);
            }
            result.to_le_bytes()
        })
        .collect()
}

fn render(
    ctx: &LevelContext,
    dx: &Presentation,
    character: Character,
) -> Result<Vec<[u8; 3]>, String> {
    let level_id = ctx.manifest.level_id;
    let level = dx
        .level(level_id)
        .ok_or_else(|| format!("no DX graphics for level {level_id}"))?;
    let player = dx.player(character, level.theme);
    let bg_palettes = if ctx.sim.paused {
        pause_palettes(&level.bg_palettes, ctx.sim.frame_counter & 16 != 0)
    } else {
        level.bg_palettes.clone()
    };
    let palettes =
        CgbPalettes::from_bytes(&bg_palettes, &level.obj_palettes).map_err(|e| e.to_string())?;
    let background = [live_background(ctx, dx, level), level.background[1].clone()];
    // Each assembled bank starts at $8000. Signed BG tile addressing adds $800.
    let banks: [Vec<u8>; 2] = std::array::from_fn(|bank| {
        let mut bytes = if bank == 1 {
            player.bank1.clone()
        } else if ctx.sim.powerup == 3 {
            player.fire.clone()
        } else {
            player.normal.clone()
        };
        bytes.extend_from_slice(&background[bank]);
        bytes
    });
    let banks = [&banks[0][..], &banks[1][..]];
    let mut background = vec![
        BackgroundPixel {
            index: IndexedPixel::new(0, 0).unwrap(),
            priority: false,
        };
        160 * 144
    ];
    let sx = ctx.cam.x as i32 - 80;
    let sy = ctx.cam.y as i32 - 72 - ctx.spr.as_ref().map_or(0, |s| s.scroll_pan as i32);
    let world_w = (LAYOUT_WIDTH * 16) as i32;
    let world_h = (LAYOUT_HEIGHT * 16) as i32;
    let (ox, oy) = (window_origin(sx, world_w), window_origin(sy, world_h));
    for py in 0..144 {
        for px in 0..160 {
            let (mut wx, mut wy) = (
                window_coord(sx + px as i32, ox),
                window_coord(sy + py as i32, oy),
            );
            if let Some(pipe) = ctx.spr.as_ref().and_then(|s| s.pipe_background) {
                let (x, y) = pipe.sample(sx + px as i32, sy + py as i32);
                let (dx, dy) = pipe.source_offset();
                let (vx, vy) = if pipe.retains_source(sx + px as i32) {
                    (sx + dx, sy + dy)
                } else {
                    (sx, sy)
                };
                wx = window_coord(x, window_origin(vx, world_w));
                wy = window_coord(y, window_origin(vy, world_h));
            }
            let (cx, cy) = (wx as usize / 16, wy as usize / 16);
            let (tx, ty) = (wx as usize & 15, wy as usize & 15);
            let key = (cy * 256 + cx) as u32;
            let rewritten = ctx
                .sim
                .layout_writes
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| *v);
            let Some(id) = rewritten.or_else(|| ctx.layout_live.get(cx, cy)) else {
                continue;
            };
            let metatile = ctx
                .manifest
                .metatiles
                .get(id)
                .ok_or_else(|| format!("invalid DX metatile {id}"))?;
            let tile = if rewritten == Some(24) {
                0xff
            } else {
                metatile.tiles[(ty / 8) * 2 + tx / 8]
            };
            let attr = CgbAttributes::from_byte(level.tile_attributes[tile as usize]);
            background[py * 160 + px] = BackgroundPixel {
                index: cgb::sample_tile(
                    banks,
                    0x800 + video::bg_tile_offset(tile),
                    attr,
                    (tx & 7) as u8,
                    (ty & 7) as u8,
                )
                .map_err(|e| e.to_string())?,
                priority: attr.priority(),
            };
        }
    }
    let hud = HudStrip {
        template: dx.hud_tiles.clone(),
        ..ctx.hud.clone()
    };
    let row = hud_row_status(
        &hud,
        HudDigits {
            lives: ctx.sim.lives,
            coins_high: ctx.sim.coins_high,
            coins_low: ctx.sim.coins_low,
            money_bag: ctx.obj.money_bag,
            timer_high: ctx.sim.level_timer.high,
            timer_low: ctx.sim.level_timer.low,
        },
    );
    for dy in 0..8 {
        let py = ctx.hud_y as usize + dy;
        if py >= 144 {
            continue;
        }
        for (col, &tile) in row.iter().take(20).enumerate() {
            let attr = CgbAttributes::from_byte(level.hud_attributes[col]);
            for dx in 0..8 {
                background[py * 160 + col * 8 + dx] = BackgroundPixel {
                    index: cgb::sample_tile(
                        banks,
                        0x800 + video::bg_tile_offset(tile),
                        attr,
                        dx as u8,
                        dy as u8,
                    )
                    .map_err(|e| e.to_string())?,
                    priority: attr.priority(),
                };
            }
        }
    }
    let mut objects = vec![None; 160 * 144];
    if let Some(s) = &ctx.spr {
        let mut tables = s.tables.clone();
        tables.player_mappings = player.mappings.clone();
        // `mapping_for` picks mode A or B by level; both hold this level's table.
        tables.sprite_mappings_a = dx.objects(level).to_vec();
        tables.sprite_mappings_b = tables.sprite_mappings_a.clone();
        // Fireballs are staged by $0F71 before Mario's $2D0A draw.
        for &(x, y, frame) in &s.fireballs {
            let cells = player::draw_cells(&tables, frame, x, y, ctx.cam.x, ctx.cam.y, false)
                .ok_or_else(|| format!("missing DX fireball frame {frame}"))?;
            draw_cells(&mut objects, banks, cells, 0)?;
        }
        if let Some((x, y, frame)) = s.player {
            let mut cells = player::draw_cells_styled(
                &tables,
                frame,
                x,
                y,
                ctx.cam.x,
                ctx.cam.y,
                CellStyle {
                    force_palette_one: false,
                    behind_background: false,
                },
            )
            .ok_or_else(|| format!("missing DX Mario frame {frame}"))?;
            for cell in &mut cells {
                if s.player_star_flash {
                    cell.attr = cell.attr.wrapping_add((ctx.sim.star_low >> 1) & 3);
                }
                if s.player_behind_bg {
                    cell.attr |= 0x80;
                }
            }
            draw_cells(&mut objects, banks, cells, s.player_pan)?;
        }
        if let Some((x, y, frame)) = s.player_shield {
            if let Some(cells) =
                player::draw_cells(&tables, frame, x, y, ctx.cam.x, ctx.cam.y, false)
            {
                draw_cells(&mut objects, banks, cells, s.player_pan)?;
            }
        }
        if let (Some((x, y, _)), Some((timer, chain))) = (s.player, s.streak_counter) {
            draw_cells(
                &mut objects,
                banks,
                player::draw_streak_counter(&tables, timer, chain, x, y, ctx.cam.x, ctx.cam.y),
                s.player_pan,
            )?;
        }
        for &(x, y, frame) in &s.bumpers {
            let cells = player::draw_cells(&tables, frame, x, y, ctx.cam.x, ctx.cam.y, false)
                .ok_or_else(|| format!("missing DX bump frame {frame}"))?;
            draw_cells(&mut objects, banks, cells, 0)?;
        }
        for inst in &s.insts {
            let mapping = tables
                .mapping_for(level_id, inst.sprite)
                .ok_or_else(|| format!("missing DX object {}", inst.sprite))?;
            for entry in mapping {
                let x = if inst.flip & 0x20 != 0 {
                    (!entry.x).wrapping_sub(7)
                } else {
                    entry.x
                };
                let y = if inst.flip & 0x40 != 0 {
                    (!entry.y).wrapping_sub(7)
                } else {
                    entry.y
                };
                blit(
                    &mut objects,
                    banks,
                    inst.x as i32 - sx + x as i32,
                    inst.y as i32 - sy + y as i32,
                    entry.tile,
                    entry.attr ^ inst.flip,
                )?;
            }
        }
    }
    if ctx.scene_reload_blank {
        return Ok(vec![[255; 3]; 160 * 144]);
    }
    cgb::compose_frame(&background, &objects, true, &palettes).map_err(|e| e.to_string())
}

fn draw_cells(
    objects: &mut [Option<ObjectPixel>],
    banks: [&[u8]; 2],
    cells: Vec<Cell>,
    pan: u8,
) -> Result<(), String> {
    for cell in cells {
        blit(
            objects,
            banks,
            cell.x as i32 - 8,
            cell.y.wrapping_sub(pan) as i32 - 16,
            cell.tile,
            cell.attr,
        )?;
    }
    Ok(())
}

fn blit(
    objects: &mut [Option<ObjectPixel>],
    banks: [&[u8]; 2],
    x: i32,
    y: i32,
    tile: u8,
    attr: u8,
) -> Result<(), String> {
    if x <= -8 || x >= 160 || y <= -8 || y >= 144 {
        return Ok(());
    }
    let attr = CgbAttributes::from_byte(attr);
    for dy in 0..8 {
        for dx in 0..8 {
            let (px, py) = (x + dx, y + dy);
            if !(0..160).contains(&px) || !(0..144).contains(&py) {
                continue;
            }
            let target = &mut objects[py as usize * 160 + px as usize];
            if target.is_some() {
                continue;
            }
            let index = cgb::sample_tile(banks, tile as usize * 16, attr, dx as u8, dy as u8)
                .map_err(|e| e.to_string())?;
            if index.color() != 0 {
                *target = Some(ObjectPixel {
                    index,
                    behind_background: attr.priority(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_uses_literal_dx_channel_steps_and_preserves_source() {
        let bytes = [0xff, 0x7f, 0x64, 0x00];
        assert_eq!(pause_palettes(&bytes, false), [0x77, 0x6f, 0x00, 0x00]);
        assert_eq!(pause_palettes(&bytes, true), [0xed, 0x6e, 0x00, 0x00]);
        assert_eq!(bytes, [0xff, 0x7f, 0x64, 0x00]);
    }
    #[test]
    fn cgb_overlay_uses_mapping_bank_and_earlier_oam_even_when_behind() {
        let bank0 = [0; 32];
        let mut bank1 = [0; 32];
        bank1[..16].copy_from_slice(&[0x80, 0].repeat(8));
        let mut objects = vec![None; 160 * 144];
        blit(&mut objects, [&bank0, &bank1], 0, 0, 0, 0x8c).unwrap();
        blit(&mut objects, [&bank0, &bank1], 0, 0, 0, 0x0e).unwrap();
        assert_eq!(
            objects[0],
            Some(ObjectPixel {
                index: IndexedPixel::new(1, 4).unwrap(),
                behind_background: true
            })
        );
        assert!(objects[1].is_none());
    }
}
