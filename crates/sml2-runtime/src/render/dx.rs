//! DX presentation of any level. Rendering never advances native gameplay.
use super::{hud_row_status, window_coord, window_origin, HudDigits};
use crate::session::{LevelContext, Session, SessionState};
use sml2_core::actor::ScreenPosition;
use sml2_core::front_end::screen::{file_select_backdrop, title_backdrop, Backdrop};
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
    /// The level a DX frame would draw: live play or an attract demo, outside
    /// boss arenas and fights. DX's `$36:$7306` gives each arena its own
    /// palettes, VRAM1 graphics and attribute map, and its boss sprites go
    /// through their own draw path; neither is imported yet.
    fn dx_level(&self) -> Option<&LevelContext> {
        let ctx = self.level_ctx()?;
        let boss = ctx.cam.boss_fight_phase != 0
            || ctx.spr.as_ref().is_some_and(|s| !s.boss_oam.is_empty());
        (!ctx.arena_loaded && !boss).then_some(ctx)
    }

    /// Whether DX presentation covers the current screen: the title, the
    /// file select and every level, live or in a demo, except boss arenas.
    /// The map, arenas and other screens keep their native picture.
    pub fn dx_supported(&self) -> bool {
        match &self.state {
            SessionState::Title(_)
            | SessionState::FileSelectLoading(_)
            | SessionState::FileSelect(_) => true,
            // The hub and the eight zones; the bonus rooms stay native.
            SessionState::Overworld(map_sim) => self.shown_map(map_sim).screen_id <= 8,
            _ => self.dx_level().is_some(),
        }
    }

    /// Render the current level in direct CGB RGB888 color, drawing the player
    /// with `character`'s graphics. Physics stay native either way.
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
        match &self.state {
            SessionState::Title(_) => {
                return Ok((screen(&dx.title, &[], None, view, title_backdrop)?, 1));
            }
            SessionState::FileSelectLoading(loading) => {
                // `$19` keeps the title until its transfers turn the LCD off.
                if loading.lcd_disabled() {
                    return Ok((vec![[255; 3]; view.0 * view.1], 1));
                }
                return Ok((screen(&dx.title, &[], None, view, title_backdrop)?, 1));
            }
            SessionState::FileSelect(file_select) => {
                let frame = self.render_dx_file_select(file_select, dx, character, view)?;
                return Ok((frame, 1));
            }
            SessionState::Overworld(map_sim) => {
                return Ok((self.render_dx_map(map_sim, dx, character, view)?, 1))
            }
            _ => {}
        }
        let ctx = self
            .dx_level()
            .ok_or("DX presentation requires the title, file select or a level")?;
        let offsets = self.motion_offsets(smooth);
        Ok((render(ctx, dx, character, view, &offsets)?, offsets.scale))
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
        // `$30064`: Easy mode draws the small-Mario frames, 32 ids later.
        let frame = if fs.easy_mode() {
            fs.cursor_sprite().wrapping_add(32)
        } else {
            fs.cursor_sprite()
        };
        sprite(frame, fs.cursor(), fs.cursor_behind_bg());
        screen(
            &dx.file_select,
            &cells,
            Some(player),
            view,
            file_select_backdrop,
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
    ) -> Result<Vec<[u8; 3]>, String> {
        let map_sim = self.shown_map(map_sim);
        let colors = dx
            .map
            .screens
            .get(usize::from(map_sim.screen_id))
            .ok_or("DX colors only the hub and the zones")?;
        // `$26:$77F1`: the hub's eastern half has its own background set.
        let bg_palettes = match &colors.alternate_bg_palettes {
            Some(east) if map_sim.cam_x >= 0xE7 => east,
            _ => &colors.bg_palettes,
        };
        let player = usize::from(character == Character::Luigi);
        let palettes = CgbPalettes::from_bytes(bg_palettes, &colors.obj_palettes[player])
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
        self.overworld_view(map_sim, view, [0, 0, 0], |sim, size| {
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
            };
            let frame = self.overworld_frame(sim, assets, size);
            dx_map_pixels(
                &frame,
                size,
                attributes,
                &dx.map.hub_attributes,
                blank,
                &palettes,
            )
        })
    }
}

fn dx_map_pixels(
    frame: &crate::render::session::OverworldFrame,
    (w, h): (usize, usize),
    attributes: Option<&Vec<u8>>,
    hub_attributes: &[u8],
    blank: IndexedPixel,
    palettes: &CgbPalettes,
) -> Result<Vec<[u8; 3]>, String> {
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
                let color = frame
                    .vram
                    .get(s.tile)
                    .map_or(0, |t| t[usize::from(s.pixel)]);
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
    // `draw_map_objects`: smaller X wins, then the earlier entry.
    let mut objects = vec![None; w * h];
    let mut order: Vec<_> = frame.composition.oam.iter().enumerate().collect();
    order.sort_by_key(|&(i, &(_, x, _, _))| (x, i));
    for (_, &(oy, ox, tile, attr)) in order {
        let Some(data) = frame.vram.get(usize::from(tile)) else {
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
    cgb::compose_frame(&background, &objects, true, palettes).map_err(|e| e.to_string())
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
            let ((col, row), line) = if (0..160).contains(&nx) && (0..144).contains(&ny) {
                (((nx / 8) as usize, (ny / 8) as usize), (ny & 7) as usize)
            } else {
                backdrop(
                    &screen.tilemap,
                    nx.div_euclid(8),
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
                    nx.rem_euclid(8) as u8,
                    line as u8,
                )
                .map_err(|e| e.to_string())?,
                priority: attr.priority(),
            });
        }
    }
    let mut layers = super::layers::Layers::new(view, 0, background[0]);
    push_cells(&mut layers.cells, banks, cells.to_vec(), 0, at, (0, 0))?;
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

/// The metatile DX draws for an unrewritten cell. DX re-laid out many levels,
/// mostly decoration; its cell is drawn where it keeps the native cell's
/// collision class, and the native cell elsewhere, so the picture never shows
/// ground the native physics lacks. Coin and block rewrites are native state.
fn dx_cell(ctx: &LevelContext, level: &LevelGraphics, cx: usize, cy: usize) -> Option<u8> {
    let native = ctx.layout_live.get(cx, cy)?;
    let classes = &ctx.manifest.metatile_classes;
    let class = |id: u8| classes.get(usize::from(id));
    match level.layout.get(cy * LAYOUT_WIDTH + cx) {
        Some(&dx) if class(dx).is_some() && class(dx) == class(native) => Some(dx),
        _ => Some(native),
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
    dx: &Presentation,
    character: Character,
    view: (usize, usize),
    offsets: &super::motion::Offsets,
) -> Result<Vec<[u8; 3]>, String> {
    let (vw, vh) = view;
    let (mx, _) = super::view::margin(view);
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
    let border = offsets.border() as i32;
    let blank = BackgroundPixel {
        index: IndexedPixel::new(0, 0).unwrap(),
        priority: false,
    };
    let mut layers = super::layers::Layers::new(view, offsets.border(), blank);
    let sx = ctx.cam.x as i32 - vw as i32 / 2;
    let sy = ctx.cam.y as i32
        - vh as i32 / 2
        - super::view::level_bias(view)
        - ctx.spr.as_ref().map_or(0, |s| s.scroll_pan as i32);
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
            let Some(id) = rewritten.or_else(|| dx_cell(ctx, level, cx, cy)) else {
                continue;
            };
            let metatile = level
                .metatiles
                .get(id)
                .ok_or_else(|| format!("invalid DX metatile {id}"))?;
            let tile = if rewritten == Some(24) {
                0xff
            } else {
                metatile.tiles[(ty / 8) * 2 + tx / 8]
            };
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
    let pan = ctx.spr.as_ref().map_or(0, |s| s.scroll_pan as i32);
    let scenery_window =
        super::margin::SceneryWindow::new(view, (ctx.cam.x, ctx.cam.y), pan, border, &ctx.scenery);
    let past_level = ctx.spr.as_ref().and_then(|s| s.pipe_background).is_none();
    for py in -border..vh as i32 + border {
        if !past_level {
            break;
        }
        for px in -border..vw as i32 + border {
            match scenery_window.sample(px, py) {
                super::margin::MarginSample::Native => {}
                super::margin::MarginSample::Blank => {
                    *layers.bg_mut(px, py) = blank;
                }
                super::margin::MarginSample::Level { wx, wy } => {
                    let (cx, cy) = (wx as usize / 16, wy as usize / 16);
                    let (tx, ty) = (wx as usize & 15, wy as usize & 15);
                    let Some(id) = dx_cell(ctx, level, cx, cy) else {
                        *layers.bg_mut(px, py) = blank;
                        continue;
                    };
                    let metatile = level
                        .metatiles
                        .get(id)
                        .ok_or_else(|| format!("invalid DX metatile {id}"))?;
                    let tile = metatile.tiles[(ty / 8) * 2 + tx / 8];
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
    for dy in 0..8 {
        let py = ctx.hud_y as i32 + vh as i32 - 144 + dy;
        if !(0..vh as i32).contains(&py) {
            continue;
        }
        for (col, &tile) in row.iter().take(20).enumerate() {
            let attr = CgbAttributes::from_byte(level.hud_attributes[col]);
            for dx in 0..8 {
                let x = mx + (col * 8 + dx) as i32;
                if !(0..vw as i32).contains(&x) {
                    continue;
                }
                layers.window[py as usize * vw + x as usize] = Some(BackgroundPixel {
                    index: cgb::sample_tile(
                        banks,
                        0x800 + video::bg_tile_offset(tile),
                        attr,
                        dx as u8,
                        dy as u8,
                    )
                    .map_err(|e| e.to_string())?,
                    priority: attr.priority(),
                });
            }
        }
    }
    if let Some(s) = &ctx.spr {
        let mut tables = s.tables.clone();
        tables.player_mappings = player.mappings.clone();
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
                    inst.x as i32 - sx + x as i32,
                    inst.y as i32 - sy + y as i32,
                    entry.tile,
                    entry.attr ^ inst.flip,
                    offsets.object(inst.slot),
                )?);
            }
        }
    }
    if ctx.scene_reload_blank {
        return Ok(vec![[255; 3]; vw * vh * offsets.scale * offsets.scale]);
    }
    let (background, objects) = layers.compose(offsets.scale, offsets.bg, |_, _| false);
    cgb::compose_frame(&background, &objects, true, &palettes).map_err(|e| e.to_string())
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
