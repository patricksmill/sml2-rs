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
        false,
    )
}

pub(super) fn render_background(
    session: &Session,
    ctx: &LevelContext,
    dx: &sml2_dx::Presentation,
    character: Character,
) -> Result<Vec<[u8; 3]>, String> {
    render_scene(
        session,
        ctx,
        &dx.banks,
        &dx.player(character, 0).mappings,
        (160, 144),
        true,
    )
}

fn render_scene(
    session: &Session,
    ctx: &LevelContext,
    banks: &std::collections::BTreeMap<u8, Vec<u8>>,
    mappings: &[Vec<sml2_core::object::MappingEntry>],
    view: (usize, usize),
    background_only: bool,
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
    let background = baseline
        .transfer_source(0x10, 0x6000, 0x1000)
        .ok_or("ending background source missing; re-extract assets")?;
    let dx_background = bank(0x10, 0x6000, 0x1000)?;
    for (i, tile) in native.tiles().chunks_exact(16).enumerate() {
        if tile != &baseline.tiles()[i * 16..i * 16 + 16] {
            // The first transfer replaces $8000 in complete 64-byte bursts.
            if i < 128 && tile == &font[i * 16..i * 16 + 16] {
                vram[i * 16..i * 16 + 16].copy_from_slice(&dx_font[i * 16..i * 16 + 16]);
            } else if i >= 128 && tile == &background[(i - 128) * 16..(i - 127) * 16] {
                vram[i * 16..i * 16 + 16]
                    .copy_from_slice(&dx_background[(i - 128) * 16..(i - 127) * 16]);
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
    // `$1A:$5E00..$5EC1` changes the authored sets when each live
    // substate advances; later substates retain those writes.
    let (palette_bank, bg_address, obj_address, attr_bank, attr_address, attr_len) =
        match sequence.substate {
            0..=7 => (0x29, 0x6769, 0x67a9, 0x29, 0x5200, 0x240),
            8 => (0x1a, 0x5ec1, 0x5f81, 0x1a, 0x5fc1, 0x2a0),
            9 => (0x1a, 0x5f01, 0x5f81, 0x1a, 0x5fc1, 0x2a0),
            _ => (0x1a, 0x5f41, 0x5f81, 0x1a, 0x5fc1, 0x2a0),
        };
    let attrs = bank(attr_bank, attr_address, attr_len)?;
    let palettes = CgbPalettes::from_bytes(
        bank(palette_bank, bg_address, 64)?,
        bank(palette_bank, obj_address, 64)?,
    )
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
    if scene.display.objects_enabled && !background_only {
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
    fn authored_credit_sets_switch_at_live_substates_and_render_without_advancing() {
        let mut game = Session::new(crate::session::SessionAssets::load().unwrap());
        game.begin_level(0, 0).unwrap();
        let mut banks = std::collections::BTreeMap::new();
        for id in [0x10, 0x11, 0x1a, 0x29] {
            banks.insert(id, vec![0; 0x4000]);
        }
        // Distinct whole sets make the selected source observable at every pixel.
        banks.get_mut(&0x29).unwrap()[0x2769..0x27a9]
            .copy_from_slice(&[0x1f, 0].repeat(32));
        for (address, word) in [(0x5ec1, 0x03e0u16), (0x5f01, 0x7c00), (0x5f41, 0x7fff)] {
            banks.get_mut(&0x1a).unwrap()[address - 0x4000..address - 0x4000 + 64]
                .copy_from_slice(&word.to_le_bytes().repeat(32));
        }
        let mappings = game.assets.object_tables.player_mappings.clone();
        for (substate, color) in [(7, [255, 0, 0]), (8, [0, 255, 0]),
            (9, [0, 0, 255]), (10, [255; 3]), (19, [255; 3])] {
            let crate::session::SessionState::InLevel(ctx) = &mut game.state else { panic!("level") };
            ctx.ending_sequence.substate = substate;
            let ctx = game.level_ctx().unwrap();
            let first = render_scene(&game, ctx, &banks, &mappings, (160, 144), true).unwrap();
            let second = render_scene(&game, ctx, &banks, &mappings, (160, 144), true).unwrap();
            assert_eq!(first, vec![color; 160 * 144], "substate {substate}");
            assert_eq!(second, first);
            assert_eq!(game.level_ctx().unwrap().ending_sequence.substate, substate);
        }
        // The replacement extends through $9A9F; rows beyond the initial $240 survive.
        banks.get_mut(&0x1a).unwrap()[0x1fc1 + 0x240..0x1fc1 + 0x2a0].fill(7);
        let crate::session::SessionState::InLevel(ctx) = &mut game.state else { panic!("level") };
        ctx.ending_scene.scroll_y = 16;
        // Palette 7, color 0 changes the newly covered bottom two visible rows.
        banks.get_mut(&0x1a).unwrap()[0x1f41 + 56..0x1f41 + 58].fill(0);
        let pixels = render_scene(&game, game.level_ctx().unwrap(), &banks, &mappings, (160, 144), true).unwrap();
        assert_eq!(pixels[128 * 160], [0; 3]);
        assert_eq!(pixels[0], [255; 3]);
    }

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
                for id in [0x10, 0x11, 0x1a, 0x29] {
                    banks.insert(id, vec![0; 0x4000]);
                }
                let palette = banks.get_mut(&0x29).unwrap();
                palette[0x2769..0x27a9].copy_from_slice(&[0x1f, 0].repeat(32));
                let palette = banks.get_mut(&0x1a).unwrap();
                for address in [0x5ec1, 0x5f01, 0x5f41] {
                    palette[address - 0x4000..address - 0x4000 + 64]
                        .copy_from_slice(&[0x1f, 0].repeat(32));
                }
                let ctx = game.level_ctx().unwrap();
                let pixels = render_scene(
                    &game,
                    ctx,
                    &banks,
                    &tables.player_mappings,
                    (384, 216),
                    false,
                )
                .unwrap();
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
