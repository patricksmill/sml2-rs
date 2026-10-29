//! Compose fixed-scene borders after the original frame has been drawn.
use super::view;
use crate::{
    scene_scenery::SceneTarget,
    session::{Session, SessionState},
};
use sml2_core::{
    scenery::SceneryOverlay,
    video::{OutputMode, PalettePixel, PaletteSource},
};

/// Swatches are the original scene's background, without actors. They retain
/// the same 10×9 positions in native and DX presentation.
pub(crate) fn paint_border<P: Copy>(
    pixels: &mut [P],
    view: (usize, usize),
    scale: usize,
    scenery: &SceneryOverlay,
    background: &[P],
    protected: &[(i32, i32, i32, i32)],
) {
    if scenery.is_empty() || !view::enlarged(view) || background.len() != 160 * 144 {
        return;
    }
    let (mx, my) = view::margin(view);
    let width = view.0 * scale;
    if pixels.len() != width * view.1 * scale {
        return;
    }
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let x = (index % width / scale) as i32 - mx;
        let y = (index / width / scale) as i32 - my;
        if (0..160).contains(&x) && (0..144).contains(&y) {
            continue;
        }
        if protected.iter().any(|&(left, top, width, height)| {
            (left..left + width).contains(&x) && (top..top + height).contains(&y)
        }) {
            continue;
        }
        if let Some(tile) = scenery.get(x.div_euclid(16), y.div_euclid(16)) {
            let sx = usize::from(tile) % 10 * 16 + x.rem_euclid(16) as usize;
            let sy = usize::from(tile) / 10 * 16 + y.rem_euclid(16) as usize;
            if let Some(&source) = background.get(sy * 160 + sx) {
                *pixel = source;
            }
        }
    }
}

impl Session {
    /// Protected pixels in scene-local coordinates, in addition to the
    /// original screen. Arena cameras retain the level renderer's framing
    /// and HUD, so those live pixels also remain untouched.
    pub fn scene_border_protected_rects(&self, size: (usize, usize)) -> Vec<(i32, i32, i32, i32)> {
        let mut rectangles = vec![(0, 0, 160, 144)];
        if matches!(self.scene_scenery_target(), Some(SceneTarget::Arena { .. })) {
            if let Some(ctx) = self.level_ctx() {
                let camera = (ctx.cam.x, ctx.cam.y);
                let pan = ctx.spr.as_ref().map_or(0, |s| i32::from(s.scroll_pan));
                let room = ctx.cam.room();
                let native = view::level_origin(view::NATIVE, camera, pan, room);
                let enlarged = view::level_origin(size, camera, pan, room);
                let (mx, my) = view::margin(size);
                let lift = self.hud_style.lift();
                rectangles.push((
                    native.0 - enlarged.0 - mx,
                    native.1 - enlarged.1 - my + lift,
                    160,
                    144,
                ));
                match self.hud_style {
                    super::HudStyle::Classic => rectangles.push((
                        0,
                        i32::from(ctx.hud_y) + size.1 as i32 - 144 - my,
                        160,
                        8,
                    )),
                    super::HudStyle::Modern => rectangles.extend(
                        super::corner_hud_rects(size)
                            .map(|(x, y, w, h)| (x - mx, y - my, w, h)),
                    ),
                }
            }
        }
        rectangles
    }

    pub fn scene_scenery_target(&self) -> Option<SceneTarget> {
        if self
            .pending_level
            .as_ref()
            .is_some_and(|t| t.easy_card_visible())
        {
            return None;
        }
        match &self.state {
            SessionState::Title(_) => Some(SceneTarget::Title),
            SessionState::FileSelectLoading(load) if !load.lcd_disabled() => {
                Some(SceneTarget::Title)
            }
            SessionState::FileSelect(_) => Some(SceneTarget::FileSelect),
            SessionState::GameOver(_) => Some(SceneTarget::GameOver),
            SessionState::Castle(_) => Some(SceneTarget::Castle),
            SessionState::Overworld(map) => Some(SceneTarget::Map(self.shown_map(map).screen_id)),
            _ => self.level_ctx().and_then(|ctx| {
                if ctx.ending_entry.state() == sml2_core::ending_entry::EndingEntryState::Sequence {
                    Some(SceneTarget::Ending)
                } else if ctx.arena_loaded {
                    Some(SceneTarget::Arena {
                        level: ctx.manifest.level_id,
                        gfx: ctx.sim.level_gfx_index,
                    })
                } else {
                    None
                }
            }),
        }
    }

    pub fn scene_scenery(&self, target: SceneTarget) -> &SceneryOverlay {
        self.assets.scene_scenery.get(target)
    }

    pub fn set_scene_scenery(
        &mut self,
        target: SceneTarget,
        overlay: SceneryOverlay,
    ) -> Result<(), String> {
        self.assets.scene_scenery.set(target, overlay)
    }

    pub fn scene_palette_frame(&self) -> Result<Vec<PalettePixel>, String> {
        Ok(self
            .scene_background(OutputMode::PaletteIndices)?
            .into_iter()
            .map(PalettePixel::from_token)
            .collect())
    }

    pub(crate) fn scene_background(&self, output: OutputMode) -> Result<Vec<u8>, String> {
        let size = view::NATIVE;
        let target = self
            .scene_scenery_target()
            .ok_or("no fixed scenery scene is showing")?;
        match target {
            SceneTarget::Title => Ok(sml2_core::front_end::render::render_title_output(
                &self.assets.front_end,
                size,
                output,
            )),
            SceneTarget::FileSelect => sml2_core::front_end::screen::render_bg_output(
                self.assets.front_end.file_select_vram(),
                self.assets.front_end.file_select_tilemap(),
                sml2_core::front_end::file_select::FILE_SELECT_BGP,
                (0, 0),
                size,
                sml2_core::front_end::screen::TileData::Signed,
                output,
            ),
            SceneTarget::Map(_) => {
                let SessionState::Overworld(map) = &self.state else {
                    return Err("map scene required".into());
                };
                let map = self.shown_map(map);
                let assets = super::session::MapAssets {
                    tilemaps: &self.assets.overworld_tilemaps,
                    manifest: &self.assets.overworld_manifest,
                    tiles: &self.assets.overworld_tiles,
                    zone_animation_tiles: &self.assets.zone_animation_tiles,
                    wave: self.map_wave.clone(),
                    sprites: None,
                    bell_tiles: None,
                    room_tiles: None,
                    player_mappings: None,
                };
                let mut frame =
                    self.overworld_frame(map, assets, size, super::session::MapLayer::Screen);
                frame.composition.oam.clear();
                Ok(sml2_core::map::rasterize_overworld(
                    &frame.composition,
                    &frame.vram,
                    frame.bg_palette,
                    frame.obj_palette,
                    frame.obj_palette1,
                    160,
                    144,
                    output,
                ))
            }
            SceneTarget::Castle | SceneTarget::GameOver => {
                let (tiles, map, palette) = if let SessionState::Castle(sequence) = &self.state {
                    let (tiles, map, _) = sml2_core::map::castle_sequence_projection(
                        &self.assets.castle_reveal,
                        &self.assets.castle_scripts,
                        sequence,
                    );
                    (tiles, map, sequence.bgp)
                } else {
                    {
                        let (tiles, map, _) = sml2_core::map::castle_sequence_projection(
                            &self.assets.castle_reveal,
                            &self.assets.castle_scripts,
                            &Default::default(),
                        );
                        (tiles, map, 0xe4)
                    }
                };
                let shades = output.palette(palette, PaletteSource::Bg);
                Ok((0..160 * 144)
                    .map(|i| {
                        let (x, y) = (i % 160, i / 160);
                        let tile = map.get(y / 8 * 32 + x / 8).copied().unwrap_or(0);
                        let tile = if tile < 128 {
                            usize::from(tile) + 256
                        } else {
                            usize::from(tile)
                        };
                        let shade = tiles.get(tile).map_or(0, |t| t[y % 8 * 8 + x % 8]);
                        shades[usize::from(shade)]
                    })
                    .collect())
            }
            SceneTarget::Arena { .. } => {
                let ctx = self.level_ctx().ok_or("arena scene required")?;
                super::render_view_output_smooth(
                    &ctx.manifest,
                    &ctx.layout_live,
                    &ctx.sim.layout_writes,
                    &ctx.bg_live,
                    ctx.cam.x,
                    ctx.cam.y,
                    ctx.cam.room(),
                    160,
                    144,
                    None,
                    super::HudStyle::Classic,
                    None,
                    None,
                    output,
                    &super::motion::Offsets::none(),
                    None,
                )
            }
            SceneTarget::Ending => {
                let ctx = self.level_ctx().ok_or("ending scene required")?;
                let mut scene = ctx.ending_scene.clone();
                scene.objects.clear_cells();
                scene.display.objects_enabled = false;
                Ok(sml2_core::ending_render::render_ending_output(
                    &scene,
                    &ctx.ending_sequence,
                    size,
                    output,
                ))
            }
        }
    }

    pub(crate) fn decorate_scene_native(
        &self,
        frame: &mut [u8],
        view: (usize, usize),
        scale: usize,
        output: OutputMode,
    ) {
        let Some(target) = self.scene_scenery_target() else {
            return;
        };
        let overlay = self.scene_scenery(target);
        if !overlay.is_empty() && view::enlarged(view) {
            if let Ok(background) = self.scene_background(output) {
                paint_border(
                    frame,
                    view,
                    scale,
                    overlay,
                    &background,
                    &self.scene_border_protected_rects(view),
                );
            }
        }
    }

    pub(crate) fn decorate_scene_dx(
        &self,
        frame: &mut [[u8; 3]],
        view: (usize, usize),
        scale: usize,
        dx: &sml2_dx::Presentation,
        character: sml2_core::character::Character,
    ) -> Result<(), String> {
        let Some(target) = self.scene_scenery_target() else {
            return Ok(());
        };
        let overlay = self.scene_scenery(target);
        if !overlay.is_empty() && view::enlarged(view) {
            let background = self.scene_palette_dx(dx, character)?;
            paint_border(
                frame,
                view,
                scale,
                overlay,
                &background,
                &self.scene_border_protected_rects(view),
            );
        }
        Ok(())
    }
}
