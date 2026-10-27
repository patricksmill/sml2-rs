//! DX presentation of any level. Rendering never advances native gameplay.
use super::{draw_corner_hud, hud_row_status, window_coord, window_origin, HudDigits, HudStyle};
use crate::session::{LevelContext, Session, SessionState};
use sml2_core::actor::ScreenPosition;
use sml2_core::front_end::screen::{
    castle_backdrop, file_select_backdrop, title_backdrop, Backdrop,
};
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
use sml2_dx::{LevelGraphics, Presentation, Screen};

impl Session {
    /// The fixed scene's 160×144 background, without actor sprites or borders.
    pub fn scene_palette_dx(
        &self,
        dx: &Presentation,
        character: Character,
    ) -> Result<Vec<[u8; 3]>, String> {
        use crate::scene_scenery::SceneTarget;
        let target = self
            .scene_scenery_target()
            .ok_or("no fixed scenery scene is showing")?;
        let size = super::view::NATIVE;
        match target {
            SceneTarget::Title => screen(&dx.title, &[], None, size, title_backdrop, 0),
            SceneTarget::FileSelect => {
                screen(&dx.file_select, &[], None, size, file_select_backdrop, 0)
            }
            SceneTarget::Castle | SceneTarget::GameOver => {
                self.render_dx_castle_scene(dx, character, size, true)
            }
            SceneTarget::Map(_) => {
                let SessionState::Overworld(map) = &self.state else {
                    unreachable!()
                };
                self.render_dx_map(map, dx, character, size, true)
            }
            SceneTarget::Arena { .. } => render(
                self.level_ctx().ok_or("arena scene required")?,
                dx,
                character,
                size,
                &super::motion::Offsets::none(),
                true,
                HudStyle::Classic,
            ),
            SceneTarget::Ending => super::dx_ending::render_background(
                self,
                self.level_ctx().ok_or("ending scene required")?,
                dx,
                character,
            ),
        }
    }

    /// The level a DX frame would draw: live play or an attract demo,
    /// arenas and boss fights included.
    fn dx_level(&self) -> Option<&LevelContext> {
        self.level_ctx()
    }

    /// Whether DX presentation covers the current screen: the title, the
    /// file select, the overworld (hub, zones and both minigames) and every
    /// level, live or in a demo, boss fights included. The other screens keep
    /// their native picture.
    pub fn dx_supported(&self) -> bool {
        match &self.state {
            SessionState::Title(_)
            | SessionState::FileSelectLoading(_)
            | SessionState::FileSelect(_)
            | SessionState::GameOver(_)
            | SessionState::Castle(_) => true,
            // The hub, the eight zones, and both minigames (screens 9–14).
            SessionState::Overworld(map_sim) => self.shown_map(map_sim).screen_id <= 14,
            _ => self.dx_level().is_some(),
        }
    }

    /// Render the current level in direct CGB RGB888 color, drawing the player
    /// with `character`'s graphics.
    pub fn render_dx_frame(
        &self,
        dx: &Presentation,
        character: Character,
        view: (usize, usize),
    ) -> Result<Vec<[u8; 3]>, String> {
        self.render_dx_frame_smooth(dx, character, view, super::motion::Smooth::OFF)
            .map(|(frame, _)| frame)
    }

    /// [`Self::render_dx_frame`] with level frames smoothed by `smooth`,
    /// returning the frame and its output pixels per game pixel.
    pub fn render_dx_frame_smooth(
        &self,
        dx: &Presentation,
        character: Character,
        view: (usize, usize),
        smooth: super::motion::Smooth,
    ) -> Result<(Vec<[u8; 3]>, usize), String> {
        // Attract demos always show Mario, whatever the file select chose.
        let character = if matches!(self.state, SessionState::Demo(_)) { Character::Mario } else { character };
        let (mut frame, scale) = self.render_dx_frame_smooth_base(dx, character, view, smooth)?;
        self.decorate_scene_dx(&mut frame, view, scale, dx, character)?;
        Ok((frame, scale))
    }

    fn render_dx_frame_smooth_base(
        &self,
        dx: &Presentation,
        character: Character,
        view: (usize, usize),
        smooth: super::motion::Smooth,
    ) -> Result<(Vec<[u8; 3]>, usize), String> {
        if let Some(ctx) = self.level_ctx().filter(|ctx| {
            ctx.ending_entry.state() == sml2_core::ending_entry::EndingEntryState::Sequence
        }) {
            return super::dx_ending::render(self, ctx, dx, character, view).map(|p| (p, 1));
        }
        match &self.state {
            SessionState::Title(_) => {
                return Ok((screen(&dx.title, &[], None, view, title_backdrop, 0)?, 1));
            }
            SessionState::FileSelectLoading(loading) => {
                // `$19` keeps the title until its transfers turn the LCD off.
                if loading.lcd_disabled() {
                    return Ok((vec![[255; 3]; view.0 * view.1], 1));
                }
                return Ok((screen(&dx.title, &[], None, view, title_backdrop, 0)?, 1));
            }
            SessionState::FileSelect(file_select) => {
                let frame = self.render_dx_file_select(file_select, dx, character, view)?;
                return Ok((frame, 1));
            }
            SessionState::GameOver(_) | SessionState::Castle(_) => {
                return self
                    .render_dx_castle_scene(dx, character, view, false)
                    .map(|p| (p, 1));
            }
            SessionState::Overworld(map_sim) => {
                return Ok((self.render_dx_map(map_sim, dx, character, view, false)?, 1))
            }
            _ => {}
        }
        let ctx = self
            .dx_level()
            .ok_or("DX presentation requires the title, file select or a level")?;
        let offsets = self.motion_offsets(smooth);
        Ok((
            render(ctx, dx, character, view, &offsets, false, self.hud_style)?,
            offsets.scale,
        ))
    }

    fn render_dx_castle_scene(
        &self,
        dx: &Presentation,
        character: Character,
        view: (usize, usize),
        background_only: bool,
    ) -> Result<Vec<[u8; 3]>, String> {
        let bank = |id: u8, address: usize, len: usize| -> Result<Vec<u8>, String> {
            dx.banks
                .get(&id)
                .and_then(|b| b.get(address - 0x4000..address - 0x4000 + len))
                .map(|b| b.to_vec())
                .ok_or_else(|| "DX castle assets require re-import".into())
        };
        let mut vram = bank(0x0c, 0x4c92, 0x1800)?;
        let mut tilemap = bank(0x0c, 0x6492, 0x300)?;
        let (sprites, attr_addr, bg_addr, obj_addr) = match &self.state {
            SessionState::GameOver(over) => (
                sml2_core::map::game_over_sprites(sml2_core::map::GameOverView {
                    fade_timer: over.sim.fade_timer,
                    frame_counter: over.sim.frame_counter,
                    golden_coin_flags: over.sim.golden_coin_flags,
                    obp: over.obp,
                }),
                0x4000,
                0x7c39,
                0x7c51,
            ),
            SessionState::Castle(seq) => {
                let reveal = sml2_core::map::CastleRevealTiles::from_bytes(&vram, &tilemap)
                    .ok_or("invalid DX castle tiles")?;
                let mut scripts = self.assets.castle_scripts.clone();
                for name in [
                    "castle_row_windows",
                    "castle_congrats_tilemap",
                    "castle_congrats_tiles",
                ] {
                    let meta = scripts.meta(name).ok_or("missing castle metadata")?.clone();
                    let address = u16::from_str_radix(meta.addr.trim_start_matches('$'), 16)
                        .map_err(|e| e.to_string())?;
                    scripts
                        .replace_blob(name, &bank(meta.bank, usize::from(address), meta.len)?)?;
                }
                let (tiles, map, sprites) =
                    sml2_core::map::castle_sequence_projection(&reveal, &scripts, seq);
                for (tile, pixels) in vram.as_chunks_mut::<16>().0.iter_mut().zip(tiles) {
                    for y in 0..8 {
                        for x in 0..8 {
                            tile[y * 2] = (tile[y * 2] << 1) | (pixels[y * 8 + x] & 1);
                            tile[y * 2 + 1] =
                                (tile[y * 2 + 1] << 1) | ((pixels[y * 8 + x] >> 1) & 1);
                        }
                    }
                }
                tilemap = map;
                (
                    sprites,
                    0x4240,
                    0x7042,
                    if character == Character::Luigi {
                        0x709a
                    } else {
                        0x705a
                    },
                )
            }
            _ => return Err("castle scene required".into()),
        };
        let mut attributes = bank(0x27, attr_addr, 0x240)?;
        attributes.resize(0x400, 0);
        let mut bg_palettes = bank(0x26, bg_addr, 24)?;
        bg_palettes.resize(64, 0);
        tilemap.resize(0x400, 0);
        let mut obj_palettes = bank(0x26, obj_addr, 64)?;
        if let SessionState::Castle(seq) = &self.state {
            if seq.state == sml2_core::castle_sequence::STATE_REVEAL_B && seq.menu == 3 {
                bg_palettes = crate::session::dx_effects::castle_entry(&bg_palettes, seq.fade_low);
                obj_palettes = crate::session::dx_effects::castle_entry(&obj_palettes, seq.fade_low);
            } else if seq.state == sml2_core::castle_sequence::STATE_MENU && seq.menu == 1 {
                crate::session::dx_effects::castle_coin(&mut obj_palettes, self.ff97());
            }
        }
        let scene = Screen {
            vram: [vram, vec![0; 0x1800]],
            tilemap,
            attributes,
            bg_palettes,
            obj_palettes,
        };
        let mut tables = self.assets.object_tables.clone();
        tables.player_mappings = dx.player(character, 0).mappings.clone();
        let mut cells = Vec::new();
        for (x, y, frame) in sprites.into_iter().filter(|_| !background_only) {
            player::append_cells(
                &tables,
                frame,
                ScreenPosition { x, y },
                CellStyle::default(),
                &mut cells,
            );
        }
        let pixels = screen(
            &scene,
            &cells,
            None,
            super::view::NATIVE,
            file_select_backdrop,
            0,
        )?;
        if matches!(&self.state, SessionState::Castle(seq) if matches!(seq.state,
            sml2_core::castle_sequence::STATE_CONGRATS | sml2_core::castle_sequence::STATE_WAIT_START))
            || !super::view::enlarged(view)
        {
            return Ok(super::view::letterbox(
                &pixels,
                super::view::NATIVE,
                view,
                [0; 3],
            ));
        }
        let mut extended = screen(&scene, &[], None, view, castle_backdrop, 0)?;
        super::view::copy_rect(
            &mut extended,
            view,
            &pixels,
            160,
            (0, 0, 160, 144),
            super::view::margin(view),
        );
        Ok(extended)
    }

    /// `$30000`'s screen with its OAM: the pipe tallies, the Easy banner, the
    /// erase hop and the cursor, through DX's player mappings and graphics.
    fn render_dx_file_select(
        &self,
        fs: &sml2_core::front_end::FileSelect,
        dx: &Presentation,
        character: Character,
        view: (usize, usize),
    ) -> Result<Vec<[u8; 3]>, String> {
        let player = dx.player(character, 0);
        let mut tables = self.assets.object_tables.clone();
        tables.player_mappings = player.mappings.clone();
        let mut cells = Vec::new();
        // `$300FE`: high nibble then low nibble, tile + $80, X +8 between digits.
        for slot in 0..3u8 {
            let tally = self.slots.slot(usize::from(slot)).map_or(0, |r| r[0x42]);
            let x = 32 + 32 * slot;
            for (dx, digit) in [(0, tally >> 4), (8, tally & 0x0F)] {
                cells.push(Cell {
                    y: 88,
                    x: x + dx,
                    tile: 0x80 | digit,
                    attr: 0,
                });
            }
        }
        let mut sprite = |frame: u8, (x, y): (u8, u8), behind: bool| {
            let style = CellStyle {
                force_palette_one: false,
                behind_background: behind,
            };
            player::append_cells(&tables, frame, ScreenPosition { x, y }, style, &mut cells);
        };
        if fs.easy_mode() {
            sprite(241, (68, 48), false);
        }
        if let Some((y, frame)) = fs.erase_hop() {
            sprite(frame, (fs.cursor().0, y), true);
        }
        let cursor = fs.cursor_presentation();
        sprite(cursor.sprite, cursor.position, cursor.behind_background);
        // The character door ends at x $C8, past the 160-wide map. Follow the
        // cursor so the walk stays on screen instead of leaving through the edge.
        // Clamp before widening: signed saturation would permit negative scroll.
        let scroll_x = i32::from(fs.cursor().0.saturating_sub(152));
        screen(
            &dx.file_select,
            &cells,
            Some(player),
            view,
            file_select_backdrop,
            scroll_x,
        )
    }
}

impl Session {
    /// The overworld through DX's tiles, maps and patches, colored by the
    /// screen's palettes and either its attribute map (zones) or the hub's
    /// per-tile table (`$29:$6542`, which `$29:$6440` also uses for tiles the
    /// map rewrites).
    fn render_dx_map(
        &self,
        map_sim: &sml2_core::map_sim::MapSim,
        dx: &Presentation,
        character: Character,
        view: (usize, usize),
        background_only: bool,
    ) -> Result<Vec<[u8; 3]>, String> {
        let map_sim = self.shown_map(map_sim);
        let colors = dx
            .map
            .screens
            .get(usize::from(map_sim.screen_id))
            .ok_or("DX colors the hub, the zones and both minigames")?;
        // `$26:$77F1`: the hub's eastern half has its own background set.
        let bg_palettes = match &colors.alternate_bg_palettes {
            Some(east) if map_sim.cam_x >= 0xE7 => east,
            _ => &colors.bg_palettes,
        };
        let player = usize::from(character == Character::Luigi);
        let mut bg_palettes = bg_palettes.to_vec();
        let mut obj_palettes = colors.obj_palettes[player].clone();
        if let Some(transition) = &self.pending_level {
            bg_palettes = crate::session::dx_effects::map_exit(&bg_palettes, transition.frame, 9);
            obj_palettes = crate::session::dx_effects::map_exit(&obj_palettes, transition.frame, 9);
        } else if let Some(transition) = &self.map_screen_transition {
            if transition.entering && transition.shows_source() {
                bg_palettes = crate::session::dx_effects::map_exit(&bg_palettes, transition.frame, 9);
                obj_palettes = crate::session::dx_effects::map_exit(&obj_palettes, transition.frame, 9);
            } else if !transition.entering {
                let waits = transition.frame.saturating_sub(80);
                bg_palettes = crate::session::dx_effects::map_exit(&bg_palettes, waits, 5);
                obj_palettes = crate::session::dx_effects::map_exit(&obj_palettes, waits, 5);
            }
        } else if let Some(transition) = &self.map_pan_return {
            let waits = transition.fade_waits();
            bg_palettes = crate::session::dx_effects::map_return(&bg_palettes, waits);
            obj_palettes = crate::session::dx_effects::map_return(&obj_palettes, waits);
        } else if self.coin_room_blank {
            bg_palettes = crate::session::dx_effects::white(bg_palettes.len());
            obj_palettes = crate::session::dx_effects::white(obj_palettes.len());
        }
        let palettes = CgbPalettes::from_bytes(&bg_palettes, &obj_palettes)
            .map_err(|e| e.to_string())?;
        let space_path =
            map_sim.level_progress[sml2_core::map::progress_offset::SPACE_PATH_0] & 1 != 0;
        let attributes = match (&colors.alternate_attributes, space_path) {
            (Some(alternate), true) => Some(alternate),
            _ => colors.attributes.as_ref(),
        };
        // The only pixels a hub frame leaves empty are the status bar's
        // `$3F203` strip, which DX draws in color 3 (black in every hub set);
        // a zone's are past its 18 rows.
        let blank = IndexedPixel::new(if map_sim.screen_id == 0 { 3 } else { 0 }, 0).unwrap();
        let player = dx.player(character, 0);
        let mario = &player.mappings;
        // `$24:$78B0` loads the rooms' `$8000` sprites from `$2A`/`$2B` by
        // `$FFF6`; DX's bank-6 copy is only Mario's.
        let room_tiles = sml2_core::map::RoomCommonTiles::from_bytes(&player.normal)
            .ok_or("invalid DX player graphics")?;
        // VRAM1 on every map screen: the player's `$8000` overlay over what
        // the title and file select left at `$8800`. Room Mario and the coin
        // rooms' status box (attribute bit 3) draw from it.
        let mut bank1 = dx.file_select.vram[1].clone();
        bank1[..0x800].copy_from_slice(&player.bank1);
        let vram1: Vec<[u8; 64]> = bank1
            .as_chunks::<16>()
            .0
            .iter()
            .map(video::tile_colors)
            .collect();
        // DX's eye whites on color 0 belong to the map cursor; the rooms'
        // Mario has none.
        let cursor = map_sim.screen_id < 9;
        // The room's unused map columns contain unrelated graphics. Extend
        // its clear background color instead, using the interior cell's palette.
        let surround = if (9..=14).contains(&map_sim.screen_id) {
            let palette = attributes
                .and_then(|a| a.get(8 * 32 + 10))
                .copied()
                .unwrap_or(0)
                & 7;
            let offset = usize::from(palette) * 8;
            sml2_core::hardware_palettes::rgb555(u16::from_le_bytes([
                bg_palettes[offset],
                bg_palettes[offset + 1],
            ]))
        } else {
            [0, 0, 0]
        };
        self.overworld_view(map_sim, view, surround, |sim, size| {
            let wave = self
                .map_wave
                .as_ref()
                .and_then(|live| sml2_core::map::MapWave::follow(dx.map.tiles.screen_raw(0), live));
            let assets = crate::render::session::MapAssets {
                tilemaps: &dx.map.tilemaps,
                manifest: &dx.map.manifest,
                tiles: &dx.map.tiles,
                zone_animation_tiles: &dx.map.zone_animation_tiles,
                wave,
                sprites: Some(&dx.map.sprites),
                bell_tiles: Some(&dx.map.bell_tiles),
                room_tiles: Some(&room_tiles),
                player_mappings: Some(mario),
            };
            let mut frame = self.overworld_frame(sim, assets, size);
            if background_only {
                frame.composition.oam.clear();
            }
            dx_map_pixels(
                &frame,
                &vram1,
                size,
                attributes,
                &dx.map.hub_attributes,
                blank,
                &palettes,
                cursor,
            )
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn dx_map_pixels(
    frame: &crate::render::session::OverworldFrame,
    vram1: &[[u8; 64]],
    (w, h): (usize, usize),
    attributes: Option<&Vec<u8>>,
    hub_attributes: &[u8],
    blank: IndexedPixel,
    palettes: &CgbPalettes,
    cursor: bool,
) -> Result<Vec<[u8; 3]>, String> {
    // Attribute bit 3 picks VRAM bank 1.
    let bank = |attr: u8| {
        if attr & 0x08 != 0 {
            vram1
        } else {
            &frame.vram[..]
        }
    };
    let mut background = Vec::with_capacity(w * h);
    for sample in &frame.composition.bg {
        let pixel = match sample {
            Some(s) => {
                let attr = match attributes {
                    Some(map) => {
                        let (x, y) = (usize::from(s.cell.0), usize::from(s.cell.1));
                        map.get(y * 32 + x).copied().unwrap_or(0)
                    }
                    None => hub_attributes[usize::from(s.tile_byte)],
                };
                let (mut px, mut py) = (s.pixel % 8, s.pixel / 8);
                if attr & 0x20 != 0 {
                    px = 7 - px;
                }
                if attr & 0x40 != 0 {
                    py = 7 - py;
                }
                let color = bank(attr)
                    .get(s.tile)
                    .map_or(0, |t| t[usize::from(py * 8 + px)]);
                BackgroundPixel {
                    index: IndexedPixel::new(color, attr & 7).map_err(|e| e.to_string())?,
                    priority: attr & 0x80 != 0,
                }
            }
            None => BackgroundPixel {
                index: blank,
                priority: false,
            },
        };
        background.push(pixel);
    }
    // DX runs in CGB mode, where the earlier OAM entry wins regardless of X.
    // Luigi's mappings layer VRAM1 face cells over his head in that order.
    let mut objects = vec![None; w * h];
    for &(oy, ox, tile, attr) in &frame.composition.oam {
        let Some(data) = bank(attr).get(usize::from(tile)) else {
            continue;
        };
        for dy in 0..8i32 {
            for dx in 0..8i32 {
                let (x, y) = (i32::from(ox) - 8 + dx, i32::from(oy) - 16 + dy);
                if !(0..w as i32).contains(&x) || !(0..h as i32).contains(&y) {
                    continue;
                }
                let sx = if attr & 0x20 != 0 { 7 - dx } else { dx };
                let sy = if attr & 0x40 != 0 { 7 - dy } else { dy };
                let color = data[(sy * 8 + sx) as usize];
                let target = &mut objects[y as usize * w + x as usize];
                if color == 0 || target.is_some() {
                    continue;
                }
                *target = Some(ObjectPixel {
                    index: IndexedPixel::new(color, attr & 7).map_err(|e| e.to_string())?,
                    behind_background: attr & 0x80 != 0,
                });
            }
        }
    }
    let mut pixels =
        cgb::compose_frame(&background, &objects, true, palettes).map_err(|e| e.to_string())?;
    if cursor {
        fill_cursor_eye_holes(
            &mut pixels,
            &frame.composition.oam,
            &frame.vram,
            (w, h),
            palettes,
        );
    }
    Ok(pixels)
}

/// DX's cursor tiles put the eye whites on sprite color 0. That index is
/// transparent, so the map shows through the eyes. Color 0 that is enclosed
/// by the cursor (the shared tiles `0..32`) is the authored eye color, OBJ
/// palette color 0. Color 0 on the silhouette stays transparent.
fn fill_cursor_eye_holes(
    pixels: &mut [[u8; 3]],
    oam: &[(u8, u8, u8, u8)],
    vram: &[[u8; 64]],
    (w, h): (usize, usize),
    palettes: &CgbPalettes,
) {
    let mut cover = vec![None; w * h];
    for &(oy, ox, tile, attr) in oam {
        if tile >= 32 {
            continue;
        }
        let Some(data) = vram.get(usize::from(tile)) else {
            continue;
        };
        for dy in 0..8i32 {
            for dx in 0..8i32 {
                let (x, y) = (i32::from(ox) - 8 + dx, i32::from(oy) - 16 + dy);
                if !(0..w as i32).contains(&x) || !(0..h as i32).contains(&y) {
                    continue;
                }
                let sx = if attr & 0x20 != 0 { 7 - dx } else { dx };
                let sy = if attr & 0x40 != 0 { 7 - dy } else { dy };
                let i = y as usize * w + x as usize;
                if cover[i].is_some() {
                    continue;
                }
                cover[i] = Some((data[(sy * 8 + sx) as usize], attr & 7));
            }
        }
    }
    let mut exterior = vec![false; w * h];
    let mut queue = Vec::new();
    let leaks = |cover: &[Option<(u8, u8)>], x: usize, y: usize| {
        [
            (x as i32 - 1, y as i32),
            (x as i32 + 1, y as i32),
            (x as i32, y as i32 - 1),
            (x as i32, y as i32 + 1),
        ]
        .into_iter()
        .any(|(nx, ny)| {
            nx < 0
                || ny < 0
                || nx >= w as i32
                || ny >= h as i32
                || cover[ny as usize * w + nx as usize].is_none()
        })
    };
    for i in 0..w * h {
        if !matches!(cover[i], Some((0, _))) {
            continue;
        }
        if leaks(&cover, i % w, i / w) {
            exterior[i] = true;
            queue.push(i);
        }
    }
    while let Some(i) = queue.pop() {
        let (x, y) = (i % w, i / w);
        for (nx, ny) in [
            (x as i32 - 1, y as i32),
            (x as i32 + 1, y as i32),
            (x as i32, y as i32 - 1),
            (x as i32, y as i32 + 1),
        ] {
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let j = ny as usize * w + nx as usize;
            if exterior[j] || !matches!(cover[j], Some((0, _))) {
                continue;
            }
            exterior[j] = true;
            queue.push(j);
        }
    }
    for (i, sample) in cover.iter().enumerate() {
        let Some((0, pal)) = *sample else {
            continue;
        };
        if exterior[i] {
            continue;
        }
        pixels[i] = sml2_core::hardware_palettes::rgb555(palettes.objects()[usize::from(pal)][0]);
    }
}

/// A 20×18 front-end screen at scroll 0, with OAM cells in priority order.
/// Native when `view` is not enlarged; otherwise centred, with `backdrop`
/// past the map. The signed `$8800` tile mode serves both the title and the
/// file select, and `player`, when given, replaces the `$8000` graphics in
/// both banks.
fn screen(
    screen: &Screen,
    cells: &[Cell],
    player: Option<&sml2_dx::PlayerGraphics>,
    view: (usize, usize),
    backdrop: Backdrop,
    scroll_x: i32,
) -> Result<Vec<[u8; 3]>, String> {
    use super::view::{enlarged, margin, NATIVE};
    let (view, at) = if enlarged(view) {
        (view, margin(view))
    } else {
        (NATIVE, (0, 0))
    };
    let palettes = CgbPalettes::from_bytes(&screen.bg_palettes, &screen.obj_palettes)
        .map_err(|e| e.to_string())?;
    let mut vram = screen.vram.clone();
    if let Some(player) = player {
        vram[0][..0x800].copy_from_slice(&player.normal);
        vram[1][..0x800].copy_from_slice(&player.bank1);
    }
    let banks = [&vram[0][..], &vram[1][..]];
    let mut background = Vec::with_capacity(view.0 * view.1);
    for y in 0..view.1 as i32 {
        let ny = y - at.1;
        for x in 0..view.0 as i32 {
            let nx = x - at.0;
            let world_x = nx + scroll_x;
            let on_map = (0..160).contains(&world_x) && (0..144).contains(&ny);
            let ((col, row), line) = if on_map {
                (
                    ((world_x / 8) as usize, (ny / 8) as usize),
                    (ny & 7) as usize,
                )
            } else {
                backdrop(
                    &screen.tilemap,
                    world_x.div_euclid(8),
                    ny.div_euclid(8),
                    ny.rem_euclid(8) as usize,
                )
            };
            let cell = row * 32 + col;
            let attr = CgbAttributes::from_byte(screen.attributes[cell]);
            background.push(BackgroundPixel {
                index: cgb::sample_tile(
                    banks,
                    0x800 + video::bg_tile_offset(screen.tilemap[cell]),
                    attr,
                    world_x.rem_euclid(8) as u8,
                    line as u8,
                )
                .map_err(|e| e.to_string())?,
                priority: attr.priority(),
            });
        }
    }
    let mut layers = super::layers::Layers::new(view, 0, background[0]);
    let cell_scroll = scroll_x;
    push_cells(
        &mut layers.cells,
        banks,
        cells.to_vec(),
        0,
        (at.0 - cell_scroll, at.1),
        (0, 0),
    )?;
    let (_, objects) = layers.compose(1, (0, 0), |_, _| false);
    cgb::compose_frame(&background, &objects, true, &palettes).map_err(|e| e.to_string())
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
        .as_chunks::<2>()
        .0
        .iter()
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
    view: (usize, usize),
    offsets: &super::motion::Offsets,
    background_only: bool,
    hud_style: HudStyle,
) -> Result<Vec<[u8; 3]>, String> {
    let (vw, vh) = view;
    let (mx, _) = super::view::margin(view);
    let level_id = ctx.manifest.level_id;
    let level = dx
        .level(level_id)
        .ok_or_else(|| format!("no DX graphics for level {level_id}"))?;
    let player = dx.player(character, level.theme);
    // `$02:$7500` on every `$25AF` reload; `$36:$7306` once a fight loads.
    let arena = dx.arena(level_id).filter(|_| ctx.arena_loaded);
    let fight = arena
        .zip(ctx.boss_definition)
        .and_then(|(arena, definition)| arena.fight(definition));
    // The reload's `$A21A` bump moves `$2C:$5D62` to the other mapping table.
    let mappings = &dx
        .player(character, level.theme + arena.map_or(0, |a| a.theme))
        .mappings;
    let level_source = (&level.bg_palettes[..], &level.obj_palettes[..]);
    let (bg_source, obj_source) = fight.map_or(level_source, |f| (&f.bg_palettes, &f.obj_palettes));
    // A fight load clears earlier overrides; later regional triggers can
    // replace its colors, while `$FFAD` survives the load.
    let (bg_source, obj_source) = ctx
        .dx_regional
        .resolve(bg_source, obj_source, |at| dx.rom(0x26, at, 64))?;
    // `$31:$4000`'s pause dims whatever `$D200` holds, regional sets included.
    let bg_palettes = if ctx.sim.paused {
        pause_palettes(&bg_source, ctx.sim.frame_counter & 16 != 0)
    } else {
        bg_source
    };
    let palettes = CgbPalettes::from_bytes(&bg_palettes, &obj_source).map_err(|e| e.to_string())?;
    let background = [live_background(ctx, dx, level), level.background[1].clone()];
    // Each assembled bank starts at $8000. Signed BG tile addressing adds $800.
    let mut banks: [Vec<u8>; 2] = std::array::from_fn(|bank| {
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
    if let Some(arena) = arena {
        copy_vram(&mut banks[1], &arena.vram1);
    }
    for copy in fight.iter().flat_map(|f| &f.vram0) {
        copy_vram(&mut banks[0], copy);
    }
    let banks = [&banks[0][..], &banks[1][..]];
    // A fight's load writes its own VRAM1 attribute rows; tiles drawn since
    // (layout rewrites) take the per-tile table.
    let tile_attributes = fight
        .and_then(|f| f.tile_attributes.as_deref())
        .unwrap_or(&level.tile_attributes);
    // `$25AF` swaps in an arena's layout and its `$26B5` metatile table,
    // which DX edits in place.
    let arena_metatiles = ctx
        .arena_loaded
        .then(|| dx.arena_metatiles.get(usize::from(ctx.sim.level_gfx_index)))
        .flatten();
    let metatiles = arena_metatiles.unwrap_or(&level.metatiles);
    // DX sessions simulate DX's layout; a level entered before DX was
    // enabled keeps its native cells until the next entry.
    let cell_at = |cx: usize, cy: usize| ctx.layout_live.get(cx, cy);
    let bg_attr = |tile: u8, (wx, wy): (usize, usize), rewritten: bool| {
        let cell = fight
            .filter(|_| !rewritten)
            .and_then(|f| f.cell_attribute(wx / 8 % 32, wy / 8 % 32));
        CgbAttributes::from_byte(cell.unwrap_or(tile_attributes[usize::from(tile)]))
    };
    let border = offsets.border() as i32;
    let blank = BackgroundPixel {
        index: IndexedPixel::new(0, 0).unwrap(),
        priority: false,
    };
    let mut layers = super::layers::Layers::new(view, offsets.border(), blank);
    let pan = ctx.spr.as_ref().map_or(0, |s| s.scroll_pan as i32);
    let camera = (ctx.cam.x, ctx.cam.y);
    let room = ctx.cam.room();
    let viewport = super::view::LevelViewport::new(view, camera, pan, room, offsets.scale, offsets.bg);
    let lift = if background_only { 0 } else { hud_style.lift() };
    let (sx, sy) = (viewport.origin.0, viewport.origin.1 - lift);
    let raw_origin = super::view::raw_level_origin(view, camera, pan);
    let world_w = (LAYOUT_WIDTH * 16) as i32;
    let world_h = (LAYOUT_HEIGHT * 16) as i32;
    let (span_x, span_y) = (super::view::window_span(vw), super::view::window_span(vh));
    let (ox, oy) = (
        window_origin(sx, world_w, span_x),
        window_origin(sy, world_h, span_y),
    );
    for py in -border..vh as i32 + border {
        for px in -border..vw as i32 + border {
            let (mut wx, mut wy) = (
                window_coord(sx + px, ox, span_x),
                window_coord(sy + py, oy, span_y),
            );
            if let Some(pipe) = ctx.spr.as_ref().and_then(|s| s.pipe_background) {
                let (x, y) = pipe.sample(sx + px, sy + py);
                let (dx, dy) = pipe.source_offset();
                let (vx, vy) = if pipe.retains_source(sx + px) {
                    (sx + dx, sy + dy)
                } else {
                    (sx, sy)
                };
                wx = window_coord(x, window_origin(vx, world_w, span_x), span_x);
                wy = window_coord(y, window_origin(vy, world_h, span_y), span_y);
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
            let Some(id) = rewritten.or_else(|| cell_at(cx, cy)) else {
                continue;
            };
            let metatile = metatiles
                .get(id)
                .ok_or_else(|| format!("invalid DX metatile {id}"))?;
            let tile = if rewritten == Some(24) {
                0xff
            } else {
                metatile.tiles[(ty / 8) * 2 + tx / 8]
            };
            let attr = bg_attr(tile, (wx as usize, wy as usize), rewritten.is_some());
            *layers.bg_mut(px, py) = BackgroundPixel {
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
    let pan = ctx.spr.as_ref().map_or(0, |s| s.scroll_pan as i32);
    let scenery_window =
        super::margin::SceneryWindow::new(view, (ctx.cam.x, ctx.cam.y), pan, room, border + lift, &ctx.scenery)
            .with_layout_writes(&ctx.sim.layout_writes);
    let past_level = ctx.spr.as_ref().and_then(|s| s.pipe_background).is_none();
    for py in -border..vh as i32 + border {
        if !past_level {
            break;
        }
        for px in -border..vw as i32 + border {
            match scenery_window.sample(px, py - lift) {
                super::margin::MarginSample::Native => {}
                super::margin::MarginSample::Blank => {
                    *layers.bg_mut(px, py) = blank;
                }
                super::margin::MarginSample::Level { wx, wy } => {
                    let (cx, cy) = (wx as usize / 16, wy as usize / 16);
                    let (tx, ty) = (wx as usize & 15, wy as usize & 15);
                    let rewritten = scenery_window.rewrite(px, py - lift);
                    let Some(id) = rewritten.or_else(|| cell_at(cx, cy)) else {
                        *layers.bg_mut(px, py) = blank;
                        continue;
                    };
                    let metatile = metatiles
                        .get(id)
                        .ok_or_else(|| format!("invalid DX metatile {id}"))?;
                    let tile = if rewritten == Some(24) {
                        0xff
                    } else {
                        metatile.tiles[(ty / 8) * 2 + tx / 8]
                    };
                    let attr = bg_attr(tile, (wx as usize, wy as usize), rewritten.is_some());
                    *layers.bg_mut(px, py) = BackgroundPixel {
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
                super::margin::MarginSample::Paint { metatile, tx, ty } => {
                    let metatile = level.metatiles.get(metatile).ok_or_else(|| {
                        format!("scenery metatile {metatile} is not in the level")
                    })?;
                    let tile = metatile.tiles[(ty >> 3) * 2 + (tx >> 3)];
                    let attr = CgbAttributes::from_byte(level.tile_attributes[tile as usize]);
                    *layers.bg_mut(px, py) = BackgroundPixel {
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
    let sample_hud = |col: usize, tile: u8, x: u8, y: u8| {
        let attr = CgbAttributes::from_byte(level.hud_attributes[col]);
        cgb::sample_tile(banks, 0x800 + video::bg_tile_offset(tile), attr, x, y)
            .map(|index| BackgroundPixel {
                index,
                priority: attr.priority(),
            })
            .map_err(|e| e.to_string())
    };
    if hud_style == HudStyle::Modern && !background_only {
        // DX's own HUD tiles paint a dark backing into every cell, so the
        // corners draw the original glyphs in DX's HUD palettes instead.
        draw_corner_hud(
            &row,
            view,
            &mut layers.window,
            |col, tile, x, y| {
                let off = video::bg_tile_offset(tile);
                let bytes: &[u8; 16] = ctx.bg_live[off..off + 16]
                    .try_into()
                    .map_err(|_| format!("HUD tile {tile:#04X} out of range"))?;
                let color = video::tile_colors(bytes)[usize::from(y) * 8 + usize::from(x)];
                let attr = CgbAttributes::from_byte(level.hud_attributes[col]);
                let index =
                    IndexedPixel::new(color, attr.palette()).map_err(|e| e.to_string())?;
                Ok((
                    color,
                    BackgroundPixel {
                        index,
                        priority: attr.priority(),
                    },
                ))
            },
            |ink| BackgroundPixel {
                index: IndexedPixel::new(0, ink.index.palette()).expect("a sampled palette"),
                ..ink
            },
        )?;
    }
    let classic_rows = if background_only || hud_style != HudStyle::Classic { 0 } else { 8 };
    for dy in 0..classic_rows {
        let py = ctx.hud_y as i32 + vh as i32 - 144 + dy;
        if !(0..vh as i32).contains(&py) {
            continue;
        }
        for (col, &tile) in row.iter().take(20).enumerate() {
            for dx in 0..8 {
                let x = mx + (col * 8 + dx) as i32;
                if !(0..vw as i32).contains(&x) {
                    continue;
                }
                layers.window[py as usize * vw + x as usize] =
                    Some(sample_hud(col, tile, dx as u8, dy as u8)?);
            }
        }
    }
    if let Some(s) = ctx.spr.as_ref().filter(|_| !background_only) {
        let mut tables = s.tables.clone();
        tables.player_mappings = mappings.clone();
        // `mapping_for` picks mode A or B by level; both hold this level's table.
        tables.sprite_mappings_a = dx.objects(level).to_vec();
        tables.sprite_mappings_b = tables.sprite_mappings_a.clone();
        let place = |x, y| super::view::sprite_anchor(view, (ctx.cam.x, ctx.cam.y), (x, y));
        // Fireballs are staged by $0F71 before Mario's $2D0A draw.
        for (i, &(x, y, frame)) in s.fireballs.iter().enumerate() {
            let ((ax, ay), offset) = place(x, y);
            let cells = player::draw_cells(&tables, frame, x, y, ax, ay, false)
                .ok_or_else(|| format!("missing DX fireball frame {frame}"))?;
            push_cells(
                &mut layers.cells,
                banks,
                cells,
                0,
                offset,
                offsets.fireball(i),
            )?;
        }
        if let Some((x, y, frame)) = s.player {
            let ((ax, ay), offset) = place(x, y);
            let mut cells = player::draw_cells_styled(
                &tables,
                frame,
                x,
                y,
                ax,
                ay,
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
            push_cells(
                &mut layers.cells,
                banks,
                cells,
                s.player_pan,
                offset,
                offsets.player,
            )?;
        }
        if let Some((x, y, frame)) = s.player_shield {
            let ((ax, ay), offset) = place(x, y);
            if let Some(cells) = player::draw_cells(&tables, frame, x, y, ax, ay, false) {
                push_cells(
                    &mut layers.cells,
                    banks,
                    cells,
                    s.player_pan,
                    offset,
                    offsets.shield,
                )?;
            }
        }
        if let (Some((x, y, _)), Some((timer, chain))) = (s.player, s.streak_counter) {
            let ((ax, ay), offset) = place(x, y);
            push_cells(
                &mut layers.cells,
                banks,
                player::draw_streak_counter(&tables, timer, chain, x, y, ax, ay),
                s.player_pan,
                offset,
                offsets.player,
            )?;
        }
        for (i, &(x, y, frame)) in s.bumpers.iter().enumerate() {
            let ((ax, ay), offset) = place(x, y);
            let cells = player::draw_cells(&tables, frame, x, y, ax, ay, false)
                .ok_or_else(|| format!("missing DX bump frame {frame}"))?;
            push_cells(
                &mut layers.cells,
                banks,
                cells,
                0,
                offset,
                offsets.bumper(i),
            )?;
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
                layers.cells.push(obj_cell(
                    banks,
                    inst.x as i32 - raw_origin.0 + x as i32,
                    inst.y as i32 - raw_origin.1 + y as i32,
                    entry.tile,
                    entry.attr ^ inst.flip,
                    offsets.object(inst.slot),
                )?);
            }
        }
        // The boss pass's records replayed through DX's lists, which move
        // and recolor some. These are already OAM bytes, so an enlarged view
        // offsets them by the same level margin as the native blit.
        let (bx, by) = super::view::level_margin(view);
        let mut boss = Vec::new();
        for &draw in &s.boss_draws {
            if let Some(list) = dx.boss_oam_lists.get(usize::from(draw.sprite)) {
                sml2_core::objects::expand_boss_list(list, draw, &mut boss);
            }
        }
        for c in boss {
            layers.cells.push(obj_cell(
                banks,
                i32::from(c.x) - 8 + bx,
                i32::from(c.y) - 16 + by,
                c.tile,
                c.attr,
                (0, 0),
            )?);
        }
    }
    // `$25AF` uploads `$36:$7746`'s white; the door scroll shows it until
    // `$0EC0` runs `$7306`, the same window native play blanks.
    if ctx.scene_reload_blank {
        return Ok(vec![[255; 3]; vw * vh * offsets.scale * offsets.scale]);
    }
    for cell in &mut layers.cells {
        cell.y += lift;
        cell.shift.0 += viewport.sprite_shift.0;
        cell.shift.1 += viewport.sprite_shift.1;
    }
    let (background, objects) = layers.compose(offsets.scale, viewport.bg_shift, |_, _| false);
    cgb::compose_frame(&background, &objects, true, &palettes).map_err(|e| e.to_string())
}

/// Lay a DX VRAM copy over an assembled bank.
fn copy_vram(bank: &mut [u8], copy: &sml2_dx::VramCopy) {
    if let Some(target) = bank.get_mut(copy.offset..copy.offset + copy.bytes.len()) {
        target.copy_from_slice(&copy.bytes);
    }
}

/// One 8x8 CGB OBJ cell at screen `(x, y)`; colour 0 is transparent.
fn obj_cell(
    banks: [&[u8]; 2],
    x: i32,
    y: i32,
    tile: u8,
    attr: u8,
    shift: (i32, i32),
) -> Result<Cell8, String> {
    let attr = CgbAttributes::from_byte(attr);
    let mut pixels = [None; 64];
    for (i, pixel) in pixels.iter_mut().enumerate() {
        let index = cgb::sample_tile(
            banks,
            tile as usize * 16,
            attr,
            (i % 8) as u8,
            (i / 8) as u8,
        )
        .map_err(|e| e.to_string())?;
        if index.color() != 0 {
            *pixel = Some(ObjectPixel {
                index,
                behind_background: attr.priority(),
            });
        }
    }
    Ok(Cell8 {
        x,
        y,
        shift,
        pixels,
    })
}

type Cell8 = super::layers::Cell<ObjectPixel>;

fn push_cells(
    out: &mut Vec<Cell8>,
    banks: [&[u8]; 2],
    cells: Vec<Cell>,
    pan: u8,
    offset: (i32, i32),
    shift: (i32, i32),
) -> Result<(), String> {
    for cell in cells {
        out.push(obj_cell(
            banks,
            cell.x as i32 - 8 + offset.0,
            cell.y.wrapping_sub(pan) as i32 - 16 + offset.1,
            cell.tile,
            cell.attr,
            shift,
        )?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dx_map_cursor_fills_an_enclosed_color_zero_and_keeps_the_outline_clear() {
        use crate::render::session::OverworldFrame;
        use sml2_core::hardware_palettes::rgb555;
        use sml2_core::map::OverworldComposition;
        let mut tile = [3u8; 64];
        tile[0] = 0;
        tile[3 * 8 + 3] = 0;
        let mut bg = [0u8; 64];
        bg[4..6].copy_from_slice(&0x001fu16.to_le_bytes());
        let mut obj = [0u8; 64];
        obj[0..2].copy_from_slice(&0x03e0u16.to_le_bytes());
        let palettes = CgbPalettes::from_bytes(&bg, &obj).unwrap();
        let frame = OverworldFrame {
            composition: OverworldComposition {
                bg: vec![None; 64],
                oam: vec![(16, 8, 0, 0)],
            },
            vram: vec![tile],
            bg_palette: 0xE4,
            obj_palette: 0xD0,
            obj_palette1: 0x39,
        };
        let out = dx_map_pixels(
            &frame,
            &[],
            (8, 8),
            None,
            &[],
            IndexedPixel::new(2, 0).unwrap(),
            &palettes,
            true,
        )
        .unwrap();
        assert_eq!(out[0], rgb555(0x001f), "a corner stays the background");
        assert_eq!(
            out[3 * 8 + 3],
            rgb555(0x03e0),
            "an enclosed eye uses OBJ color 0"
        );
        assert_eq!(out[1], [0, 0, 0], "the outline stays opaque");
    }

    #[test]
    fn dx_map_attribute_bit_3_draws_from_vram_bank_1() {
        use crate::render::session::OverworldFrame;
        use sml2_core::hardware_palettes::rgb555;
        use sml2_core::map::{BgSample, OverworldComposition};
        let mut bg = [0u8; 64];
        bg[2..8].copy_from_slice(&[0x1f, 0, 0xe0, 0x03, 0x00, 0x7c]);
        let mut obj = [0u8; 64];
        obj[4..6].copy_from_slice(&0x7fffu16.to_le_bytes());
        let palettes = CgbPalettes::from_bytes(&bg, &obj).unwrap();
        // Bank 0's tile is color 1, bank 1's color 2 on its top row only.
        let mut bank1 = [0u8; 64];
        bank1[..8].fill(2);
        let sample = |pixel| {
            Some(BgSample {
                tile: 0,
                tile_byte: 0,
                cell: (0, 0),
                pixel,
            })
        };
        let frame = OverworldFrame {
            composition: OverworldComposition {
                bg: vec![sample(0), sample(1)],
                oam: vec![(16, 9, 0, 0x08)],
            },
            vram: vec![[1; 64]],
            bg_palette: 0xE4,
            obj_palette: 0xD0,
            obj_palette1: 0x39,
        };
        // `$48`: bank 1, flipped vertically, so the top row reads row 7.
        let out = dx_map_pixels(
            &frame,
            &[bank1],
            (2, 1),
            Some(&vec![0x48]),
            &[],
            IndexedPixel::new(0, 0).unwrap(),
            &palettes,
            false,
        )
        .unwrap();
        assert_eq!(out[0], [0, 0, 0], "flipped: bank 1's row 7 is color 0");
        assert_eq!(out[1], rgb555(0x7fff), "the sprite reads bank 1's color 2");
    }

    #[test]
    fn file_select_scroll_keeps_the_character_door_on_screen() {
        use sml2_core::player::Cell;
        let mut vram0 = vec![0u8; 0x2000];
        for y in 0..8 {
            vram0[y * 2] = 0xFF;
            vram0[y * 2 + 1] = 0xFF;
        }
        let mut obj = vec![0u8; 64];
        obj[6..8].copy_from_slice(&0x7fffu16.to_le_bytes());
        let picture = Screen {
            vram: [vram0, vec![0u8; 0x2000]],
            tilemap: vec![0; 0x400],
            attributes: vec![0; 0x400],
            bg_palettes: vec![0; 64],
            obj_palettes: obj,
        };
        let cells = vec![Cell {
            y: 16,
            x: 200,
            tile: 0,
            attr: 0,
        }];
        let frame = screen(&picture, &cells, None, (160, 144), file_select_backdrop, 48).unwrap();
        assert_eq!(
            frame[144],
            [255, 255, 255],
            "door sprite stays on the right"
        );
        assert_eq!(
            frame[0],
            [0, 0, 0],
            "the left edge is the shifted background"
        );
    }
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
        let blank = BackgroundPixel {
            index: IndexedPixel::new(0, 0).unwrap(),
            priority: false,
        };
        let mut layers = crate::render::layers::Layers::new((160, 144), 0, blank);
        layers
            .cells
            .push(obj_cell([&bank0, &bank1], 0, 0, 0, 0x8c, (0, 0)).unwrap());
        layers
            .cells
            .push(obj_cell([&bank0, &bank1], 0, 0, 0, 0x0e, (0, 0)).unwrap());
        let (_, objects) = layers.compose(1, (0, 0), |_, _| false);
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
