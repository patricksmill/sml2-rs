//! Read-only CPU composition of the native game view. No window or filesystem access.

use crate::{SCREEN_H, SCREEN_W};
use serde::Deserialize;
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest, LAYOUT_HEIGHT, LAYOUT_WIDTH};
use sml2_core::object::ObjectTables;
use sml2_core::video;

/// One live object instance fed to the renderer: world position, sprite id and
/// the object's `sSpriteFlipFlags` byte (bit4 = OBP select, bit5 = X flip,
/// bit6 = Y flip, bit7 = BG-over-OBJ priority).
#[derive(Debug, Clone, Deserialize)]
pub struct SpriteInst {
    pub sprite: u8,
    pub x: u16,
    pub y: u16,
    #[serde(default)]
    pub flip: u8,
    /// The object slot, so a smoothed frame matches objects across ticks.
    #[serde(default)]
    pub slot: u8,
}

mod dx;
mod layers;
mod margin;
pub use margin::paintable_cell;
pub mod motion;
pub mod overlay;
mod pipe_background;
pub mod view;
pub use pipe_background::PipeBackground;

/// Everything needed for the OBJ pass.
pub struct SpriteCtx {
    /// Retained source-room columns during a horizontal scene warp.
    pub pipe_background: Option<PipeBackground>,
    pub insts: Vec<SpriteInst>,
    pub objblock: Vec<u8>,
    pub tables: ObjectTables,
    pub level_id: u8,
    pub obp0: u8,
    pub obp1: u8,
    /// Optional player (Mario): world position + `$FFC6` frame.
    pub player: Option<(u16, u16, u8)>,
    /// Optional 3-up moon shield ($54FB): world position + frame 220/221.
    pub player_shield: Option<(u16, u16, u8)>,
    /// Star defeat streak counter ($551E): `(timer, chain)` where timer is post-decrement (31..0).
    pub streak_counter: Option<(u8, u8)>,
    /// `$530D`'s `$FFBB` in draw order: true while pipe travel has set it to 0,
    /// which makes the `$5297` expander set attr bit 7 (Mario behind BG).
    pub player_behind_bg: bool,
    /// `$54C8`'s `$FFC7`: the star countdown's bit 2, which makes `$5297` set
    /// attr bit 4 on every one of Mario's cells (OBP1 — the star flash).
    pub player_star_flash: bool,
    /// Brick-bump sprites for this frame (`PlayerSim::block_draw`): the engine
    /// stages `$FFC4`/`$FFC5`/`$FFC6` and enqueues them (`$5297`), so they are
    /// extra OAM sprites drawn at the block — through the same player-sprite
    /// path, after Mario (his entries come first, so he wins overlaps).
    pub bumpers: Vec<(u16, u16, u8)>,
    /// The live fireballs' sprites (`$3261`'s `$2CF4` staging, mapping `$B0`/`$B4` +
    /// the rDiv flicker), `(x, y, mapping)` in the same convention as `bumpers`.
    pub fireballs: Vec<(u16, u16, u8)>,
    /// During a boss fight, the boss path's own draw output (`$3A14` -> bank 25
    /// `$66000`, [`sml2_core::objects::ObjectSim::boss_oam`]): the entries are
    /// already in OAM byte space, so they are blitted at `(x - 8, y - 16)` and
    /// their attr byte is used as-is (the pass XORed `sSpriteFlipFlags` in).
    /// Drawn after Mario, like the level objects — and `insts` is empty on
    /// these frames, because the boss slots' state byte carries HP, not a slot
    /// state, and the engine's own draw never runs in the boss path.
    pub boss_oam: Vec<sml2_core::objects::BossOamCell>,
    /// Distinct phases: Mario subtracts the decremented phase; BG/objects use latched scroll.
    pub player_pan: u8,
    pub scroll_pan: u8,
}

/// The BG is a 32x32-tile tilemap = 256 px. The engine loads it with the level's rows and
/// columns around the camera — clamped into the level, because the window has to hold real
/// level rows — and never clamps the camera or the view.
pub(crate) fn window_origin(view: i32, world: i32, span: i32) -> i32 {
    view.clamp(0, world - span)
}

/// A screen row/column seen through that window: itself inside the level, the window's own
/// rows again outside it. A span is a whole number of 16-px metatiles and of 8-px
/// tiles, so the wrap keeps both grids and the fine scroll intact.
pub(crate) fn window_coord(v: i32, origin: i32, span: i32) -> i32 {
    origin + (v - origin).rem_euclid(span)
}

/// Render a `vw`x`vh` view centred on the camera as an 8-bit grey frame. The
/// native 160x144 rect stays pixel-exact inside it; `hud` draws the
/// status-bar row in the native rect at 160x144 and bottom-centred in larger
/// views.
///
/// `overlay` is the sim's sparse level-map rewrite list (`layout_writes`: keys
/// `row * 256 + col`, the engine's `$B000` addressing). The engine edits the
/// same WRAM map its probes read, so a cell the sim rewrote — a collected
/// coin's `$60`, a bump's `$18`/`$07`/… — must draw its updated background.
/// The transient bump tile `$18` uses the ROM's separate blank BG writes.
#[allow(clippy::too_many_arguments)]
pub fn render_view(
    manifest: &LevelManifest,
    layout: &LevelLayout,
    overlay: &[(u32, u8)],
    bg: &[u8],
    cam_x: u16,
    cam_y: u16,
    vw: usize,
    vh: usize,
    hud: Option<(&[u8], u8)>,
    spr: Option<&SpriteCtx>,
    // `$2756`'s pause flash: the byte the ROM writes to `sBGPalette` (and to
    // `sOAMPalette1`, which the caller applies to `spr.obp0`) while paused,
    // standing in for the level's BG palette. `None` on every other frame —
    // which is every frame any replay sees, so the gate is unaffected.
    pal_override: Option<u8>,
) -> Result<Vec<u8>, String> {
    render_view_output(
        manifest,
        layout,
        overlay,
        bg,
        cam_x,
        cam_y,
        vw,
        vh,
        hud,
        spr,
        pal_override,
        video::OutputMode::Grayscale,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn render_view_output(
    manifest: &LevelManifest,
    layout: &LevelLayout,
    overlay: &[(u32, u8)],
    bg: &[u8],
    cam_x: u16,
    cam_y: u16,
    vw: usize,
    vh: usize,
    hud: Option<(&[u8], u8)>,
    spr: Option<&SpriteCtx>,
    // `$2756`'s pause flash: the byte the ROM writes to `sBGPalette` (and to
    // `sOAMPalette1`, which the caller applies to `spr.obp0`) while paused,
    // standing in for the level's BG palette. `None` on every other frame —
    // which is every frame any replay sees, so the gate is unaffected.
    pal_override: Option<u8>,
    output: video::OutputMode,
) -> Result<Vec<u8>, String> {
    render_view_output_smooth(
        manifest,
        layout,
        overlay,
        bg,
        cam_x,
        cam_y,
        vw,
        vh,
        hud,
        spr,
        pal_override,
        output,
        &motion::Offsets::none(),
        None,
    )
}

/// [`render_view_output`] at `offsets.scale` output pixels per game pixel,
/// each layer shifted by `offsets`.
#[allow(clippy::too_many_arguments)]
pub fn render_view_output_smooth(
    manifest: &LevelManifest,
    layout: &LevelLayout,
    overlay: &[(u32, u8)],
    bg: &[u8],
    cam_x: u16,
    cam_y: u16,
    vw: usize,
    vh: usize,
    hud: Option<(&[u8], u8)>,
    spr: Option<&SpriteCtx>,
    pal_override: Option<u8>,
    output: video::OutputMode,
    offsets: &motion::Offsets,
    scenery: Option<&sml2_core::scenery::SceneryOverlay>,
) -> Result<Vec<u8>, String> {
    // Decode every tile in the composed block once.
    let tiles: Vec<[u8; 64]> = (0..256u16)
        .map(|t| {
            let off = video::bg_tile_offset(t as u8);
            let bytes: [u8; 16] = bg[off..off + 16].try_into().expect("block is $1000 bytes");
            video::tile_colors(&bytes)
        })
        .collect();
    let shades = output.palette(
        pal_override.unwrap_or(manifest.header.palettes[0]),
        video::PaletteSource::Bg,
    );
    let world_w = (LAYOUT_WIDTH * 16) as i32;
    let world_h = (LAYOUT_HEIGHT * 16) as i32;
    // The engine never clamps the view to the level: it leaves the camera where it is and
    // slides a 256-px tilemap window under it (an enlarged view widens the window to the
    // view, `view::window_span`), so with the camera near an edge the view
    // runs past the level and shows the window's rows again. Clamping the view here (which
    // this used to do) instead shifted the whole background against Mario's OAM — on level
    // 5, whose start camera sits at the bottom of a 768-px world (cam_y 704, so the view
    // top 632 is past 768 - 144), every tile drew 8 px low and Mario looked like he floated
    // above the block row the console puts him on.
    let sx = cam_x as i32 - vw as i32 / 2;
    let sy = cam_y as i32
        - vh as i32 / 2
        - view::level_bias((vw, vh))
        - spr.map_or(0, |s| s.scroll_pan as i32);
    let (span_x, span_y) = (view::window_span(vw), view::window_span(vh));
    let (ox, oy) = (
        window_origin(sx, world_w, span_x),
        window_origin(sy, world_h, span_y),
    );
    let border = offsets.border() as i32;
    let mut layers = layers::Layers::new(
        (vw, vh),
        offsets.border(),
        NativeBg {
            shade: shades[0],
            index: 0,
        },
    );
    for py in -border..vh as i32 + border {
        let wy = window_coord(sy + py, oy, span_y);

        for px in -border..vw as i32 + border {
            let wx = window_coord(sx + px, ox, span_x);
            let (wx, wy) = spr
                .and_then(|s| s.pipe_background)
                .map_or((wx, wy), |pipe| {
                    let (x, y) = pipe.sample(sx + px, sy + py);
                    let (dx, dy) = pipe.source_offset();
                    let source = pipe.retains_source(sx + px);
                    let (vx, vy) = if source { (sx + dx, sy + dy) } else { (sx, sy) };
                    (
                        window_coord(x, window_origin(vx, world_w, span_x), span_x),
                        window_coord(y, window_origin(vy, world_h, span_y), span_y),
                    )
                });
            let (cx, tx) = ((wx as usize) >> 4, (wx as usize) & 15);
            let (cy, ty) = ((wy as usize) >> 4, (wy as usize) & 15);
            let key = (cy * 256 + cx) as u32;
            let rewritten = overlay.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
            let Some(id) = rewritten.or_else(|| layout.get(cx, cy)) else {
                continue;
            };
            let mt = manifest
                .metatiles
                .get(id)
                .ok_or_else(|| format!("metatile id {id} out of table"))?;
            let q = (ty >> 3) * 2 + (tx >> 3);
            // $5BFC keeps collision tile 24 during a block bump, while $5C01
            // clears all four BG tiles to $FF. The moving block is an OBJ.
            // Only rewrites carry this transient state; an original layout
            // cell numbered 24 still uses the level's metatile artwork.
            let tile = if rewritten == Some(24) {
                0xFF
            } else {
                mt.tiles[q] as usize
            };
            let col = tiles[tile][(ty & 7) * 8 + (tx & 7)];
            *layers.bg_mut(px, py) = NativeBg {
                shade: shades[col as usize],
                index: col,
            };
        }
    }
    if let Some(scenery) = scenery.filter(|_| spr.and_then(|s| s.pipe_background).is_none()) {
        let pan = spr.map_or(0, |s| s.scroll_pan as i32);
        for py in -border..vh as i32 + border {
            for px in -border..vw as i32 + border {
                match margin::margin_sample((vw, vh), px, py, (cam_x, cam_y), pan, scenery) {
                    margin::MarginSample::Native => {}
                    margin::MarginSample::Blank => {
                        *layers.bg_mut(px, py) = NativeBg {
                            shade: shades[0],
                            index: 0,
                        };
                    }
                    margin::MarginSample::Level { wx, wy } => {
                        let (cx, tx) = ((wx as usize) >> 4, (wx as usize) & 15);
                        let (cy, ty) = ((wy as usize) >> 4, (wy as usize) & 15);
                        let key = (cy * 256 + cx) as u32;
                        let rewritten = overlay.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
                        let Some(id) = rewritten.or_else(|| layout.get(cx, cy)) else {
                            continue;
                        };
                        let mt = manifest
                            .metatiles
                            .get(id)
                            .ok_or_else(|| format!("metatile id {id} out of table"))?;
                        let tile = if rewritten == Some(24) {
                            0xFF
                        } else {
                            mt.tiles[(ty >> 3) * 2 + (tx >> 3)] as usize
                        };
                        let col = tiles[tile][(ty & 7) * 8 + (tx & 7)];
                        *layers.bg_mut(px, py) = NativeBg {
                            shade: shades[col as usize],
                            index: col,
                        };
                    }
                    margin::MarginSample::Paint { metatile, tx, ty } => {
                        let mt = manifest.metatiles.get(metatile).ok_or_else(|| {
                            format!("scenery metatile {metatile} is not in the level")
                        })?;
                        let tile = mt.tiles[(ty >> 3) * 2 + (tx >> 3)] as usize;
                        let col = tiles[tile][(ty & 7) * 8 + (tx & 7)];
                        *layers.bg_mut(px, py) = NativeBg {
                            shade: shades[col as usize],
                            index: col,
                        };
                    }
                }
            }
        }
    }
    // Window layer: the status-bar strip (no scroll; window at WX=7 -> px 0).
    // It stays horizontally centred and on the bottom rows of a larger viewport.
    if let Some((row, wy)) = hud {
        let hx0 = (vw as i32 - SCREEN_W as i32) / 2;
        let hy0 = vh as i32 - SCREEN_H as i32 + wy as i32;
        for dy in 0..8usize {
            let py = hy0 + dy as i32;
            if py < 0 || py >= vh as i32 {
                continue;
            }
            for col in 0..SCREEN_W {
                let Some(&tile) = row.get(col) else { break };
                for dx in 0..8usize {
                    let px = hx0 + (col * 8 + dx) as i32;
                    if px < 0 || px >= vw as i32 {
                        continue;
                    }
                    let c = tiles[tile as usize][dy * 8 + dx];
                    let idx = py as usize * vw + px as usize;
                    layers.window[idx] = Some(NativeBg {
                        shade: shades[c as usize],
                        index: c,
                    });
                }
            }
        }
    }
    // OBJ sprites: the player first (its cells are OAM entries 0.., highest
    // priority), then objects — mapping entries expanded like $C000/$5297
    // (attr ^= the object's flip flags). GB priority: smaller X first, then
    // earlier instance; BG wins where attr bit 7 is set and the BG pixel is
    // non-transparent.
    if let Some(s) = spr {
        let resized = (vw, vh) != view::NATIVE;
        let (mx, my) = view::level_margin((vw, vh));
        let place = |x, y| view::sprite_anchor((vw, vh), (cam_x, cam_y), (x, y));
        let pal0 = output.palette(s.obp0, video::PaletteSource::Obj0);
        let pal1 = output.palette(s.obp1, video::PaletteSource::Obj1);
        if let Some((px, py, frame)) = s.player {
            let ((ax, ay), (ox, oy)) = place(px, py);
            let cells = sml2_core::player::draw_cells_styled(
                &s.tables,
                frame,
                px,
                py,
                ax,
                ay,
                sml2_core::player::CellStyle {
                    force_palette_one: s.player_star_flash,
                    behind_background: s.player_behind_bg,
                },
            )
            .ok_or_else(|| format!("player frame {frame} has no mapping"))?;
            let cells = in_obj_priority_order(cells);
            // The player base already contains the camera (+96/+98 is world -
            // cam), so these cells are in final screen space: no `sx/sy`.
            for c in cells {
                layers.cells.push(native_cell(
                    &s.objblock,
                    &pal0,
                    &pal1,
                    c.x as i32 - 8 + ox,
                    c.y.wrapping_sub(s.player_pan) as i32 - 16 + oy,
                    c.tile,
                    c.attr,
                    offsets.player,
                ));
            }
        }
        if let Some((shx, shy, sframe)) = s.player_shield {
            let ((ax, ay), (ox, oy)) = place(shx, shy);
            let cells = sml2_core::player::draw_cells(&s.tables, sframe, shx, shy, ax, ay, false)
                .ok_or_else(|| format!("player shield frame {sframe} has no mapping"))?;
            for c in cells {
                layers.cells.push(native_cell(
                    &s.objblock,
                    &pal0,
                    &pal1,
                    c.x as i32 - 8 + ox,
                    c.y.wrapping_sub(s.player_pan) as i32 - 16 + oy,
                    c.tile,
                    c.attr,
                    offsets.shield,
                ));
            }
        }
        if let (Some((px, py, _)), Some((timer, chain))) = (s.player, s.streak_counter) {
            let ((ax, ay), (ox, oy)) = place(px, py);
            let cells =
                sml2_core::player::draw_streak_counter(&s.tables, timer, chain, px, py, ax, ay);
            for c in cells {
                layers.cells.push(native_cell(
                    &s.objblock,
                    &pal0,
                    &pal1,
                    c.x as i32 - 8 + ox,
                    c.y.wrapping_sub(s.player_pan) as i32 - 16 + oy,
                    c.tile,
                    c.attr,
                    offsets.player,
                ));
            }
        }
        // The live fireballs ($3261's $2CF4 staging, mapping $B0/$B4 + the rDiv
        // flicker), drawn with the bumpers. `$3261` stages them inside `$0F71`,
        // i.e. before the player's own draw call, so their OAM index — and thus
        // the overlap priority against Mario for the frame or two a fresh ball
        // sits inside him — may want to precede his cells; not verified against
        // a console frame yet, and it can only show while they overlap.
        for (i, (fx, fy, ff)) in s.fireballs.iter().copied().enumerate() {
            let ((ax, ay), (ox, oy)) = place(fx, fy);
            let cells = sml2_core::player::draw_cells(&s.tables, ff, fx, fy, ax, ay, false)
                .ok_or_else(|| format!("fireball mapping {ff} has no entry"))?;
            for c in cells {
                layers.cells.push(native_cell(
                    &s.objblock,
                    &pal0,
                    &pal1,
                    c.x as i32 - 8 + ox,
                    c.y as i32 - 16 + oy,
                    c.tile,
                    c.attr,
                    offsets.fireball(i),
                ));
            }
        }
        // The brick-bump sprites ($2C82's $5297 enqueues), drawn after Mario's
        // cells and before the objects — the engine's OAM order.
        for (i, (bx, by, bf)) in s.bumpers.iter().copied().enumerate() {
            let ((ax, ay), (ox, oy)) = place(bx, by);
            let cells = sml2_core::player::draw_cells(&s.tables, bf, bx, by, ax, ay, false)
                .ok_or_else(|| format!("bump frame {bf} has no mapping"))?;
            for c in cells {
                layers.cells.push(native_cell(
                    &s.objblock,
                    &pal0,
                    &pal1,
                    c.x as i32 - 8 + ox,
                    c.y as i32 - 16 + oy,
                    c.tile,
                    c.attr,
                    offsets.bumper(i),
                ));
            }
        }
        let mut order: Vec<usize> = (0..s.insts.len()).collect();
        order.sort_by_key(|&i| (s.insts[i].x, i));
        for &i in &order {
            let inst = &s.insts[i];
            let Some(mapping) = s.tables.mapping_for(s.level_id, inst.sprite) else {
                continue;
            };
            let flip_x = inst.flip & 0x20 != 0;
            let flip_y = inst.flip & 0x40 != 0;
            for e in mapping {
                let attr = e.attr ^ inst.flip;
                let ex = if flip_x {
                    (!e.x).wrapping_sub(7) as i32
                } else {
                    e.x as i32
                };
                let ey = if flip_y {
                    (!e.y).wrapping_sub(7) as i32
                } else {
                    e.y as i32
                };
                layers.cells.push(native_cell(
                    &s.objblock,
                    &pal0,
                    &pal1,
                    inst.x as i32 - sx + ex,
                    inst.y as i32 - sy + ey,
                    e.tile,
                    attr,
                    offsets.object(inst.slot),
                ));
            }
        }
        // The boss path's own draw (`$3A14` -> bank 25 `$66000`), which replaces
        // the level's `$C000` pass while the fight runs: those entries are
        // already OAM bytes, so the screen pixel is `(x - 8, y - 16)` and the
        // attr byte stands (the pass XORed the flip flags in itself). Drawn last
        // so the fight's parts layer over the level's objects, and after Mario,
        // who keeps OAM's first entries.
        let boss_shift = if resized { (mx, my) } else { (0, 0) };
        for c in &s.boss_oam {
            layers.cells.push(native_cell(
                &s.objblock,
                &pal0,
                &pal1,
                c.x as i32 - 8 + boss_shift.0,
                c.y as i32 - 16 + boss_shift.1,
                c.tile,
                c.attr,
                (0, 0),
            ));
        }
    }
    let (bg, obj) = layers.compose(offsets.scale, offsets.bg, |o, b| o.behind && b.index != 0);
    Ok(bg
        .iter()
        .zip(&obj)
        .map(|(b, o)| o.map_or(b.shade, |o| o.shade))
        .collect())
}

/// DMG OBJ overlap priority: the smaller OAM x wins, then the earlier entry.
/// Mario's own cells overlap (the bunny ears over his head).
fn in_obj_priority_order(mut cells: Vec<sml2_core::player::Cell>) -> Vec<sml2_core::player::Cell> {
    cells.sort_by_key(|c| c.x);
    cells
}

/// A native BG/window pixel: its mapped shade and colour index (priority).
#[derive(Clone, Copy)]
struct NativeBg {
    shade: u8,
    index: u8,
}

/// A native OBJ pixel: its mapped shade and attr bit 7.
#[derive(Clone, Copy)]
struct NativeObj {
    shade: u8,
    behind: bool,
}

/// One 8x8 OAM cell at screen `(x, y)`: colour 0 is transparent, attr bit5 =
/// X flip / bit6 = Y flip / bit4 = OBP1 / bit7 = BG priority, earlier cells
/// win overlaps (push order), and OBJ tiles use the `$8000` mode
/// (`$8000 + t*16`).
#[allow(clippy::too_many_arguments)]
fn native_cell(
    objblock: &[u8],
    pal0: &[u8],
    pal1: &[u8],
    x: i32,
    y: i32,
    tile: u8,
    attr: u8,
    shift: (i32, i32),
) -> layers::Cell<NativeObj> {
    let (xf, yf) = (attr & 0x20 != 0, attr & 0x40 != 0);
    let pal = if attr & 0x10 != 0 { pal1 } else { pal0 };
    let t = tile as usize;
    let off = if t < 128 {
        t * 16
    } else {
        0x800 + (t - 128) * 16
    };
    let bytes: [u8; 16] = objblock[off..off + 16]
        .try_into()
        .expect("objblock is $1000 bytes");
    let colors = video::tile_colors(&bytes);
    let pixels = std::array::from_fn(|i| {
        let (dx, dy) = (i % 8, i / 8);
        let c = colors[(if yf { 7 - dy } else { dy }) * 8 + if xf { 7 - dx } else { dx }];
        (c != 0).then(|| NativeObj {
            shade: pal[c as usize],
            behind: attr & 0x80 != 0,
        })
    });
    layers::Cell {
        x,
        y,
        shift,
        pixels,
    }
}

mod session;

/// Build the 20-tile HUD row: ROM template + optional digit substitution
/// (ten digits in column order: lives, coins, money, time; tile = $80 + digit).
pub(crate) fn hud_row(hud: &HudStrip, digits: Option<&str>) -> Vec<u8> {
    let mut row = hud.template.clone();
    if let Some(d) = digits {
        for (i, col) in hud.digit_columns.iter().enumerate() {
            if let Some(v) = d.chars().nth(i).and_then(|c| c.to_digit(10)) {
                if let Some(slot) = row.get_mut(*col as usize) {
                    *slot = 0x80 + v as u8;
                }
            }
        }
    }
    row
}

/// Live status-bar digits. `$076B` writes these while the game state is `$04`.
pub(crate) struct HudDigits {
    /// `sLives`, BCD.
    pub lives: u8,
    /// `sCoinHigh`. Only the low nibble is drawn.
    pub coins_high: u8,
    /// `sCoinLow`, BCD.
    pub coins_low: u8,
    /// `sMoneyBagCount`, BCD. Defeating an enemy increments it (`$5B31D`).
    pub money_bag: u8,
    /// `sTimerHigh`. Drawn as the whole byte, not a nibble.
    pub timer_high: u8,
    /// `sTimerLow`, BCD.
    pub timer_low: u8,
}

/// `$076B`: patch the window map's digit tiles (`$80 + value`) from the live
/// counters. Column order is lives, coins, money bags, time.
pub(crate) fn hud_row_status(hud: &HudStrip, digits: HudDigits) -> Vec<u8> {
    let mut row = hud.template.clone();
    let values = [
        digits.lives >> 4,
        digits.lives & 0x0F,
        digits.coins_high & 0x0F,
        digits.coins_low >> 4,
        digits.coins_low & 0x0F,
        digits.money_bag >> 4,
        digits.money_bag & 0x0F,
        digits.timer_high,
        digits.timer_low >> 4,
        digits.timer_low & 0x0F,
    ];
    for (i, value) in values.into_iter().enumerate() {
        if let Some(&col) = hud.digit_columns.get(i) {
            if let Some(slot) = row.get_mut(col as usize) {
                *slot = value.wrapping_add(0x80);
            }
        }
    }
    row
}

/// Overlay the level's animated tiles (VRAM `$9200`..`$923F`) at the given
/// phase onto the composed BG block (offset `$A00` = tile 32). No-op when the
/// level's animation is disabled.
pub(crate) fn apply_tile_anim(
    bg: &mut [u8],
    anim: &sml2_core::level::TileAnim,
    rows: &[u8],
    phase: u8,
) {
    if !anim.enabled {
        return;
    }
    let off = anim.row as usize * 0x100 + (phase & 3) as usize * 0x40;
    bg[0xA00..0xA40].copy_from_slice(&rows[off..off + 0x40]);
}

#[cfg(test)]
mod tests {
    use super::{hud_row_status, in_obj_priority_order, native_cell, HudDigits, NativeObj};
    use sml2_core::level::HudStrip;
    use sml2_core::player::Cell;

    /// A later OAM entry with a smaller x wins the overlap.
    #[test]
    fn the_smaller_oam_x_wins_overlapping_obj_pixels() {
        // Tile 0 is solid colour 1, tile 1 solid colour 3.
        let mut objblock = vec![0u8; 0x1000];
        objblock[..16].copy_from_slice(&[0xFF, 0x00].repeat(8));
        objblock[16..32].fill(0xFF);
        let cells = vec![
            Cell {
                y: 16,
                x: 9,
                tile: 0,
                attr: 0,
            },
            Cell {
                y: 16,
                x: 8,
                tile: 1,
                attr: 0,
            },
        ];
        let pal = [0u8, 1, 2, 3];
        let mut layers: super::layers::Layers<u8, NativeObj> =
            super::layers::Layers::new((16, 8), 0, 0);
        for c in in_obj_priority_order(cells) {
            layers.cells.push(native_cell(
                &objblock,
                &pal,
                &pal,
                c.x as i32 - 8,
                c.y as i32 - 16,
                c.tile,
                c.attr,
                (0, 0),
            ));
        }
        let (bg, obj) = layers.compose(1, (0, 0), |_, _| false);
        let img: Vec<u8> = bg
            .iter()
            .zip(&obj)
            .map(|(b, o)| o.map_or(*b, |o| o.shade))
            .collect();
        assert_eq!(&img[..9], &[3, 3, 3, 3, 3, 3, 3, 3, 1]);
    }

    /// `$076B` writes every counter. The timer's high byte is not masked.
    #[test]
    fn the_status_bar_patches_lives_coins_money_and_time() {
        let hud = HudStrip {
            template: vec![0x80; 20],
            window_x: 7,
            window_y: 136,
            digit_columns: vec![2, 3, 7, 8, 9, 13, 14, 17, 18, 19],
        };
        let row = hud_row_status(
            &hud,
            HudDigits {
                lives: 0x15,
                coins_high: 0x12,
                coins_low: 0x34,
                money_bag: 0x07,
                timer_high: 0x14,
                timer_low: 0x59,
            },
        );
        assert_eq!(&row[2..4], &[0x81, 0x85], "lives");
        assert_eq!(&row[7..10], &[0x82, 0x83, 0x84], "coins, high nibble only");
        assert_eq!(&row[13..15], &[0x80, 0x87], "money bags");
        assert_eq!(
            &row[17..20],
            &[0x94, 0x85, 0x89],
            "timer high is the whole byte"
        );
    }
}

#[cfg(test)]
mod palette_tests;
