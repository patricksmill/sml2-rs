//! CGB ending projection, preserving the live scene's raster and transfers.
use crate::session::{LevelContext, Session};
use sml2_core::{
    character::Character,
    video::cgb::{self, BackgroundPixel, CgbAttributes, CgbPalettes, IndexedPixel, ObjectPixel},
};

pub(super) fn render(
    session: &Session,
    ctx: &LevelContext,
    dx: &sml2_dx::Presentation,
    character: Character,
    view: (usize, usize),
) -> Result<Vec<[u8; 3]>, String> {
    render_scene(
        session,
        ctx,
        &dx.banks,
        &dx.player(character, 0).mappings,
        view,
    )
}

fn render_scene(
    session: &Session,
    ctx: &LevelContext,
    banks: &std::collections::BTreeMap<u8, Vec<u8>>,
    mappings: &[Vec<sml2_core::object::MappingEntry>],
    view: (usize, usize),
) -> Result<Vec<[u8; 3]>, String> {
    let bank = |id: u8, address: usize, len: usize| -> Result<&[u8], String> {
        banks
            .get(&id)
            .and_then(|b| b.get(address - 0x4000..address - 0x4000 + len))
            .ok_or_else(|| "DX ending assets require re-import".into())
    };
    let scene = &ctx.ending_scene;
    let sequence = &ctx.ending_sequence;
    if sequence.substate < 5 || !scene.display.display_enabled {
        return Ok(vec![[255; 3]; view.0 * view.1]);
    }
    let native = scene.assets().ok_or("ending scene missing")?;
    let baseline = session
        .assets
        .ending_scene
        .as_ref()
        .ok_or("ending source missing")?;
    let mut vram = bank(0x11, 0x5800, 0x1800)?.to_vec();
    let font = baseline
        .transfer_source(0x1a, 0x4656, 0x800)
        .ok_or("ending font missing")?;
    let dx_font = bank(0x1a, 0x4656, 0x800)?;
    for (i, tile) in native.tiles().chunks_exact(16).enumerate() {
        if tile != &baseline.tiles()[i * 16..i * 16 + 16] {
            // The first transfer replaces $8000 in complete 64-byte bursts.
            if i < 128 && tile == &font[i * 16..i * 16 + 16] {
                vram[i * 16..i * 16 + 16].copy_from_slice(&dx_font[i * 16..i * 16 + 16]);
            } else {
                return Err(format!("unrecognized ending tile transfer {i}"));
            }
        }
    }
    let mut tilemap = bank(0x1a, 0x548e, 0x400)?.to_vec();
    let later = baseline
        .transfer_source(0x1a, 0x5096, 0x400)
        .ok_or("ending map source missing")?;
    let dx_later = bank(0x1a, 0x5096, 0x400)?;
    for i in 0..0x400 {
        if native.tilemap()[i] != baseline.tilemap()[i] {
            tilemap[i] = if native.tilemap()[i] == later[i] {
                dx_later[i]
            } else {
                native.tilemap()[i]
            };
        }
    }
    let attrs = bank(0x29, 0x5200, 0x240)?;
    let palettes = CgbPalettes::from_bytes(bank(0x29, 0x6769, 64)?, bank(0x29, 0x67a9, 64)?)
        .map_err(|e| e.to_string())?;
    let banks = [&vram[..], &vram[..]];
    let mut bg = Vec::with_capacity(160 * 144);
    for y in 0..144 {
        let line = sml2_core::ending_raster::ending_projection(
            sequence.frame,
            y as u8,
            sequence.raster_y,
            sequence.substate,
        );
        let wy = (y as u8).wrapping_add(scene.scroll_y);
        for x in 0..160 {
            let wx = (x as u8)
                .wrapping_add(scene.scroll_x)
                .wrapping_add(line.scroll_x as u8);
            let cell = usize::from(wy / 8) * 32 + usize::from(wx / 8);
            let attr = CgbAttributes::from_byte(attrs.get(cell).copied().unwrap_or(0));
            let offset = sml2_core::ending_raster::background_tile_offset(
                tilemap[cell],
                scene.display.background_unsigned_tiles || line.unsigned_bg_tiles,
            );
            let index = if scene.display.background_enabled {
                cgb::sample_tile(banks, offset, attr, wx % 8, wy % 8).map_err(|e| e.to_string())?
            } else {
                IndexedPixel::new(0, 0).unwrap()
            };
            bg.push(BackgroundPixel {
                index,
                priority: attr.priority(),
            });
        }
    }
    let mut tables = session.assets.object_tables.clone();
    tables.player_mappings = mappings.to_vec();
    let cells = sequence.projected_cells(&tables);
    let mut objects = vec![None; 160 * 144];
    if scene.display.objects_enabled {
        for y in 0..144i32 {
            let mut on_line: Vec<_> = cells
                .iter()
                .enumerate()
                .filter(|(_, c)| (0..8).contains(&(y + 16 - i32::from(c.y))))
                .take(10)
                .collect();
            on_line.sort_by_key(|&(i, c)| (c.x, i));
            for (_, cell) in on_line {
                let row = y + 16 - i32::from(cell.y);
                let attr = CgbAttributes::from_byte(cell.attr);
                for dx in 0..8i32 {
                    let x = i32::from(cell.x) - 8 + dx;
                    if !(0..160).contains(&x) {
                        continue;
                    }
                    let index = cgb::sample_tile(
                        banks,
                        usize::from(cell.tile) * 16,
                        attr,
                        dx as u8,
                        row as u8,
                    )
                    .map_err(|e| e.to_string())?;
                    let at = y as usize * 160 + x as usize;
                    if index.color() != 0 && objects[at].is_none() {
                        objects[at] = Some(ObjectPixel {
                            index,
                            behind_background: cell.attr & 0x80 != 0,
                        });
                    }
                }
            }
        }
    }
    let pixels = cgb::compose_frame(&bg, &objects, true, &palettes).map_err(|e| e.to_string())?;
    Ok(super::view::letterbox(&pixels, (160, 144), view, [0; 3]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ending_and_credits_render_through_live_font_and_map_transfers() {
        let mut game = Session::new(crate::session::SessionAssets::load().unwrap());
        game.begin_level(0, 0).unwrap();
        let assets = game.assets.ending_scene.clone().unwrap();
        let tables = game.assets.object_tables.clone();
        let credits = game.assets.credits_text.clone();
        let mut checked = 0;
        for _ in 0..20000 {
            let crate::session::SessionState::InLevel(ctx) = &mut game.state else {
                panic!("level")
            };
            if ctx.ending_scene.assets().is_none() {
                let (mut state, mut fade) = (4, 256);
                ctx.ending_scene.load(assets.clone(), &mut state, &mut fade);
            }
            ctx.ending_sequence
                .step(&mut sml2_core::ending_sequence::EndingStepContext {
                    scene: &mut ctx.ending_scene,
                    rom: None,
                    credits_text: Some(&credits),
                    object_tables: Some(&tables),
                    decrement_fade: true,
                });
            if ctx.ending_sequence.frame % 64 == 0 {
                // Only the ending renderer's imported banks are needed here.
                let mut banks = std::collections::BTreeMap::new();
                for id in [0x11, 0x1a, 0x29] {
                    banks.insert(id, vec![0; 0x4000]);
                }
                let palette = banks.get_mut(&0x29).unwrap();
                palette[0x2769..0x27a9].copy_from_slice(&[0x1f, 0].repeat(32));
                let ctx = game.level_ctx().unwrap();
                let pixels =
                    render_scene(&game, ctx, &banks, &tables.player_mappings, (384, 216)).unwrap();
                assert_eq!(pixels.len(), 384 * 216);
                if ctx.ending_sequence.substate >= 5 && ctx.ending_scene.display.display_enabled {
                    assert_eq!(pixels[36 * 384 + 112], [255, 0, 0]);
                }
                checked += 1;
            }
        }
        assert!(checked > 100);
    }
}
