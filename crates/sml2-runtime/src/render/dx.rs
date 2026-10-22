//! Opening-level DX presentation. Rendering never advances native gameplay.
use super::{hud_row_status, window_coord, window_origin, HudDigits};
use crate::session::{LevelContext, Session, SessionState};
use sml2_core::{
    character::Character,
    level::{HudStrip, LAYOUT_HEIGHT, LAYOUT_WIDTH},
    player::{self, Cell, CellStyle},
    video::{
        self,
        cgb::{self, BackgroundPixel, CgbAttributes, CgbPalettes, IndexedPixel, ObjectPixel},
    },
};
use sml2_dx::Bundle;

impl Session {
    /// Whether the current screen is one the audited DX data covers: the
    /// opening level. Everything else keeps its native presentation.
    pub fn dx_supported(&self) -> bool {
        matches!(&self.state, SessionState::InLevel(ctx) if ctx.manifest.level_id == 0)
    }

    /// Render the audited opening level in direct CGB RGB888 color, drawing
    /// the player with `character`'s graphics. Physics stay native either way.
    pub fn render_dx_frame(
        &self,
        bundle: &Bundle,
        character: Character,
    ) -> Result<Vec<[u8; 3]>, String> {
        let SessionState::InLevel(ctx) = &self.state else {
            return Err("DX presentation requires an active opening level".into());
        };
        if ctx.manifest.level_id != 0 {
            return Err("DX presentation supports only the opening level".into());
        }
        render(ctx, bundle, character)
    }
}

/// One character's `$8000` graphics (normal, fire, VRAM1) and player mappings.
struct PlayerGraphics<'a> {
    normal: &'a [u8],
    fire: &'a [u8],
    bank1: &'a [u8],
    mappings: &'a [Vec<sml2_core::object::MappingEntry>],
}

fn player_graphics(bundle: &Bundle, character: Character) -> Result<PlayerGraphics<'_>, String> {
    match character {
        Character::Mario => Ok(PlayerGraphics {
            normal: &bundle.mario_normal,
            fire: &bundle.mario_fire,
            bank1: &bundle.mario_bank1,
            mappings: &bundle.player_mappings,
        }),
        Character::Luigi => {
            let luigi = bundle.luigi.as_ref().ok_or(
                "This DX installation predates Luigi graphics; re-import the ROM to add them",
            )?;
            Ok(PlayerGraphics {
                normal: &luigi.normal,
                fire: &luigi.fire,
                bank1: &luigi.bank1,
                mappings: &luigi.mappings,
            })
        }
    }
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
    bundle: &Bundle,
    character: Character,
) -> Result<Vec<[u8; 3]>, String> {
    bundle.validate().map_err(|e| e.to_string())?;
    let player = player_graphics(bundle, character)?;
    let bg_palettes = if ctx.sim.paused {
        pause_palettes(&bundle.bg_palettes, ctx.sim.frame_counter & 16 != 0)
    } else {
        bundle.bg_palettes.clone()
    };
    let palettes =
        CgbPalettes::from_bytes(&bg_palettes, &bundle.obj_palettes).map_err(|e| e.to_string())?;
    // Each assembled bank starts at $8000. Signed BG tile addressing adds $800.
    let mut banks: [Vec<u8>; 2] = std::array::from_fn(|bank| {
        let mut bytes = if bank == 1 {
            player.bank1.to_vec()
        } else if ctx.sim.powerup == 3 {
            player.fire.to_vec()
        } else {
            player.normal.to_vec()
        };
        bytes.extend_from_slice(&bundle.background[bank]);
        bytes
    });
    // Level0's ROM config $4E has four $40-byte phases at $9200.
    // The native clock is read, never ticked by rendering.
    let phase = (ctx.tile_animation.phase & 3) as usize;
    if phase != 0 || ctx.frame_n >= 8 {
        banks[0][0x1200..0x1240].copy_from_slice(&bundle.animation[phase * 64..phase * 64 + 64]);
    }
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
            let attr = CgbAttributes::from_byte(bundle.tile_attributes[tile as usize]);
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
        template: bundle.hud_tiles.clone(),
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
            let attr = CgbAttributes::from_byte(bundle.hud_attributes[col]);
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
        tables.player_mappings = player.mappings.to_vec();
        tables.sprite_mappings_a = bundle.object_mappings.clone();
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
                .mapping_for(0, inst.sprite)
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
