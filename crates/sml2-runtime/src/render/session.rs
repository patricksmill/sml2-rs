//! Read-only session presentation: rendering never writes simulation state
//! (the session's view feeds only the live level's spawn margin).

#[path = "hub.rs"]
mod hub;

use super::motion::{Offsets, Smooth};
use super::render_view_output_smooth;
use super::view;
use super::{hud_row_status, HudDigits, HudStyle};
use crate::session::{LevelContext, Session, SessionState, MAP_FADE_STEP};
use sml2_core::video::{OutputMode, PalettePixel};
use std::error::Error;

/// Bank 15 `$3E9FB`: twelve waits before each of four palette writes.
fn map_entry_palettes(frame: u16, initial: (u8, u8, u8)) -> (u8, u8, u8) {
    if frame < MAP_FADE_STEP {
        return initial;
    }
    let step = (frame / MAP_FADE_STEP - 1).min(3) as usize;
    let bg = [0xE4, 0x90, 0x40, 0x00][step];
    let obj = [0xD0, 0x80, 0x40, 0x00][step];
    (bg, obj, obj)
}

impl LevelContext {
    /// Render one level view without advancing the session.
    pub fn render_frame(&self, vw: usize, vh: usize) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(self
            .render_output(vw, vh, OutputMode::Grayscale, &Offsets::none(), HudStyle::Classic)?
            .0)
    }

    fn render_output(
        &self,
        vw: usize,
        vh: usize,
        output: OutputMode,
        offsets: &Offsets,
        hud_style: HudStyle,
    ) -> Result<(Vec<u8>, usize), Box<dyn Error>> {
        // State 19 draws the ending's own VRAM, not the level.
        if self.ending_entry.state() == sml2_core::ending_entry::EndingEntryState::Sequence {
            let frame = view::fixed_screen((vw, vh), output.blank(3), |size| {
                sml2_core::ending_render::render_ending_output(
                    &self.ending_scene,
                    &self.ending_sequence,
                    size,
                    output,
                )
            });
            return Ok((frame, 1));
        }
        let hud_live = hud_row_status(
            &self.hud,
            HudDigits {
                lives: self.sim.lives,
                coins_high: self.sim.coins_high,
                coins_low: self.sim.coins_low,
                money_bag: self.obj.money_bag,
                timer_high: self.sim.level_timer.high,
                timer_low: self.sim.level_timer.low,
            },
        );
        let flash = if self.scene_reload_blank {
            Some(0)
        } else if self.sim.paused {
            Some(self.sim.pause_flash())
        } else {
            None
        };
        let frame = render_view_output_smooth(
            &self.manifest,
            &self.layout_live,
            &self.sim.layout_writes,
            &self.bg_live,
            self.cam.x,
            self.cam.y,
            self.cam.room(),
            vw,
            vh,
            Some((&hud_live, self.hud_y)),
            hud_style,
            self.spr.as_ref(),
            flash,
            output,
            offsets,
            (!self.arena_loaded).then_some(&self.scenery),
        )?;
        Ok((frame, offsets.scale))
    }
}

impl Session {
    /// Render current view to an 8-bit grey buffer.
    pub fn render_frame(&self, view: (usize, usize)) -> Vec<u8> {
        self.render_output(view, OutputMode::Grayscale, Smooth::OFF)
            .0
    }

    /// Render winning palette sources and register-mapped hardware shades.
    pub fn render_palette_frame(&self, view: (usize, usize)) -> Vec<PalettePixel> {
        self.render_output(view, OutputMode::PaletteIndices, Smooth::OFF)
            .0
            .into_iter()
            .map(PalettePixel::from_token)
            .collect()
    }

    /// [`Session::render_palette_frame`] drawn `smooth.alpha` of the way from
    /// the previous tick, with the scale actually used (1 outside levels).
    pub fn render_palette_frame_smooth(
        &self,
        view: (usize, usize),
        smooth: Smooth,
    ) -> (Vec<PalettePixel>, usize) {
        let (frame, scale) = self.render_output(view, OutputMode::PaletteIndices, smooth);
        (
            frame.into_iter().map(PalettePixel::from_token).collect(),
            scale,
        )
    }

    fn render_output(
        &self,
        view: (usize, usize),
        output: OutputMode,
        smooth: Smooth,
    ) -> (Vec<u8>, usize) {
        let (mut frame, scale) = self.render_output_base(view, output, smooth);
        self.decorate_scene_native(&mut frame, view, scale, output);
        (frame, scale)
    }

    fn render_output_base(
        &self,
        view: (usize, usize),
        output: OutputMode,
        smooth: Smooth,
    ) -> (Vec<u8>, usize) {
        let (vw, vh) = view;
        let fill = output.blank(3);
        if self
            .pending_level
            .as_ref()
            .is_some_and(|t| t.easy_card_visible())
        {
            let font = self.assets.ending_scene.as_ref().and_then(|a| {
                a.transfer_source(0x1A, 0x4656, sml2_core::easy_mode_card::EASY_CARD_FONT_LEN)
            });
            let frame = view::fixed_screen(view, fill, |size| {
                sml2_core::easy_mode_card::render_easy_card_output(
                    font.unwrap_or(&[]),
                    &self.assets.sprite_metas,
                    size,
                    output,
                )
            });
            return (frame, 1);
        }
        let frame = match &self.state {
            SessionState::Title(_) => self.title_output(view, output),
            SessionState::FileSelectLoading(loading) => {
                // Pending `$19` retains title VRAM; active transfers have LCD off.
                if loading.lcd_disabled() {
                    vec![output.blank(0); vw * vh]
                } else {
                    self.title_output(view, output)
                }
            }
            SessionState::FileSelect(file_select) => {
                self.file_select_output(file_select, view, output)
            }
            SessionState::Overworld(map_sim) => {
                let map_sim = self.shown_map(map_sim);
                let surround = if (9..=14).contains(&map_sim.screen_id) {
                    output.blank(0)
                } else {
                    fill
                };
                self.overworld_view(map_sim, view, surround, |sim, (w, h), layer| {
                    let assets = MapAssets {
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
                    let frame = self.overworld_frame(sim, assets, (w, h), layer);
                    Ok(sml2_core::map::rasterize_overworld(
                        &frame.composition,
                        &frame.vram,
                        frame.bg_palette,
                        frame.obj_palette,
                        frame.obj_palette1,
                        w,
                        h,
                        output,
                    ))
                })
                .unwrap_or_else(|_| vec![fill; vw * vh])
            }
            SessionState::Demo(demo) => {
                return demo
                    .ctx
                    .render_output(vw, vh, output, &self.motion_offsets(smooth), self.hud_style)
                    .unwrap_or_else(|_| (vec![output.blank(3); vw * vh], 1));
            }
            SessionState::InLevel(ctx) => {
                return ctx
                    .render_output(vw, vh, output, &self.motion_offsets(smooth), self.hud_style)
                    .unwrap_or_else(|_| (vec![output.blank(3); vw * vh], 1));
            }
            SessionState::Castle(seq) => {
                let render = |size| {
                    sml2_core::map::render_castle_sequence_output(
                        &self.assets.castle_reveal,
                        &self.assets.castle_scripts,
                        &self.assets.sprite_metas,
                        seq,
                        size,
                        output,
                    )
                };
                if matches!(
                    seq.state,
                    sml2_core::castle_sequence::STATE_CONGRATS
                        | sml2_core::castle_sequence::STATE_WAIT_START
                ) {
                    view::fixed_screen(view, fill, render)
                } else {
                    render(view)
                }
            }
            SessionState::GameOver(screen) => sml2_core::map::render_game_over_output(
                &self.assets.castle_reveal,
                &self.assets.sprite_metas,
                sml2_core::map::GameOverView {
                    fade_timer: screen.sim.fade_timer,
                    frame_counter: screen.sim.frame_counter,
                    golden_coin_flags: screen.sim.golden_coin_flags,
                    obp: screen.obp,
                },
                view,
                output,
            ),
        };
        (frame, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_entry_palette_writes_follow_the_rom_wait_boundaries() {
        let initial = (0xE1, 0xD0, 0x39);
        for (frame, expected) in [
            (0, initial),
            (11, initial),
            (12, (0xE4, 0xD0, 0xD0)),
            (23, (0xE4, 0xD0, 0xD0)),
            (24, (0x90, 0x80, 0x80)),
            (35, (0x90, 0x80, 0x80)),
            (36, (0x40, 0x40, 0x40)),
            (47, (0x40, 0x40, 0x40)),
            (48, (0, 0, 0)),
            (73, (0, 0, 0)),
        ] {
            assert_eq!(
                map_entry_palettes(frame, initial),
                expected,
                "frame {frame}"
            );
        }
    }
}

/// The overworld data a frame draws from: the native set or DX's.
pub(crate) struct MapAssets<'a> {
    pub tilemaps: &'a sml2_core::map::OverworldTilemaps,
    pub manifest: &'a sml2_core::map::OverworldManifest,
    pub tiles: &'a sml2_core::map::OverworldTiles,
    pub zone_animation_tiles: &'a [u8],
    pub wave: Option<sml2_core::map::MapWave>,
    /// Another revision's sprite tables (`$4C4D`, `$62E6`, `$7251`, and the
    /// minigame metasprites); `None` keeps the native ones.
    pub sprites: Option<&'a sml2_dx::MapSprites>,
    /// DX bell-room tiles. `None` uses the session's native set.
    pub bell_tiles: Option<&'a sml2_core::map::BellRoomTiles>,
    /// DX copies of the 128 tiles both minigames put at `$8000`. `None`
    /// uses the session's native set.
    pub room_tiles: Option<&'a sml2_core::map::RoomCommonTiles>,
    /// DX player mappings for the minigame Mario sprite (`$5297`'s replacement).
    pub player_mappings: Option<&'a [Vec<sml2_core::object::MappingEntry>]>,
}

/// What an overworld frame draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapLayer {
    /// The screen. A [`HudStyle::Modern`] hub leaves out `$3F203`'s
    /// counters and their strip.
    Screen,
    /// Only the hub's counters, in place over the empty strip backing.
    Counters,
}

/// One overworld frame laid out, with the tiles and DMG palettes it draws with.
pub(crate) struct OverworldFrame {
    pub composition: sml2_core::map::OverworldComposition,
    pub vram: Vec<[u8; 64]>,
    pub bg_palette: u8,
    pub obj_palette: u8,
    pub obj_palette1: u8,
}

impl Session {
    /// The map screen on view: the transition's source until it hands over.
    pub(crate) fn shown_map<'a>(
        &'a self,
        map_sim: &'a sml2_core::map_sim::MapSim,
    ) -> &'a sml2_core::map_sim::MapSim {
        self.map_screen_transition
            .as_ref()
            .filter(|t| t.shows_source())
            .map_or(map_sim, |t| &t.source)
    }

    /// The title; an enlarged view continues the sign's backdrop.
    fn title_output(&self, view: (usize, usize), output: OutputMode) -> Vec<u8> {
        use sml2_core::front_end::render::{render_title_output, render_title_wide_output};
        if super::view::enlarged(view) {
            render_title_wide_output(
                &self.assets.front_end,
                view,
                super::view::margin(view),
                output,
            )
        } else {
            render_title_output(&self.assets.front_end, view, output)
        }
    }

    /// The file select; an enlarged view continues its block frame.
    fn file_select_output(
        &self,
        file_select: &sml2_core::front_end::FileSelect,
        view: (usize, usize),
        output: OutputMode,
    ) -> Vec<u8> {
        use sml2_core::front_end::render::{
            render_file_select_output, render_file_select_wide_output,
        };
        let (fe, tables) = (&self.assets.front_end, &self.assets.object_tables);
        if super::view::enlarged(view) {
            render_file_select_wide_output(
                file_select,
                &self.slots,
                fe,
                tables,
                view,
                super::view::margin(view),
                output,
            )
        } else {
            render_file_select_output(file_select, &self.slots, fe, tables, view, output)
        }
    }

    /// The overworld in any view. The hub is stitched from native frames
    /// (its sprites are 8-bit OAM positions relative to each frame's camera)
    /// during navigation and panning. Oversized views show the whole hub.
    /// Zones and minigames keep their native 160-wide picture
    /// centered in a solid room-color surround; unused tilemap columns are
    /// not scenery. Screen 8, the minigame doors, also stays native.
    pub(crate) fn overworld_view<P: Copy + PartialEq>(
        &self,
        map_sim: &sml2_core::map_sim::MapSim,
        view: (usize, usize),
        fill: P,
        mut render: impl FnMut(
            &sml2_core::map_sim::MapSim,
            (usize, usize),
            MapLayer,
        ) -> Result<Vec<P>, String>,
    ) -> Result<Vec<P>, String> {
        use super::view::{enlarged, letterbox, NATIVE};
        if !enlarged(view) {
            let mut pixels = render(map_sim, view, MapLayer::Screen)?;
            // Panning hides the counters, as `$3C9F4` does.
            if self.hud_style == HudStyle::Modern
                && map_sim.screen_id == 0
                && map_sim.scroll_only_latch == 0
            {
                self.corner_counters(map_sim, &mut pixels, view, &mut render)?;
            }
            return Ok(pixels);
        }
        match map_sim.screen_id {
            0 => {
                let size = (view.0.min(512), view.1.min(512));
                let pixels = self.stitched_hub(map_sim, size, fill, render)?;
                Ok(letterbox(&pixels, size, view, fill))
            }
            _ => Ok(letterbox(
                &render(map_sim, NATIVE, MapLayer::Screen)?,
                NATIVE,
                view,
                fill,
            )),
        }
    }

    /// Lay out the overworld frame from `assets`.
    pub(crate) fn overworld_frame(
        &self,
        map_sim: &sml2_core::map_sim::MapSim,
        assets: MapAssets<'_>,
        (vw, vh): (usize, usize),
        layer: MapLayer,
    ) -> OverworldFrame {
        let mut vram = if map_sim.screen_id >= 13 {
            // Screens 13/14 copy bank 25 `$5000` to `$8800`; the zone
            // tilesets at `map/tiles.bin` do not apply.
            match assets.bell_tiles {
                Some(tiles) => tiles.vram_tiles(),
                None if !self.assets.bell_room_tiles.is_empty() => {
                    self.assets.bell_room_tiles.vram_tiles()
                }
                None => assets.tiles.screen_vram_tiles(map_sim.screen_id),
            }
        } else {
            assets.tiles.screen_vram_tiles(map_sim.screen_id)
        };
        // `$3F58` copies bank 6 `$4000` over `$8000` in both bonus
        // rooms, so the sprite pass sees those 128 tiles.
        if map_sim.screen_id >= 9 {
            let common = match assets.room_tiles {
                Some(tiles) => tiles,
                None => &self.assets.room_common_tiles,
            };
            if !common.is_empty() {
                for (index, tile) in common.vram_tiles().iter().enumerate() {
                    if index < vram.len() {
                        vram[index] = *tile;
                    }
                }
            }
        }
        if map_sim.screen_id == 0 {
            if let Some(wave) = &assets.wave {
                wave.apply(&mut vram);
            }
        }
        map_sim.apply_zone_tile_animation(&mut vram, assets.zone_animation_tiles);
        if let Some(room) = map_sim
            .room
            .as_ref()
            .filter(|_| (9..=12).contains(&map_sim.screen_id))
        {
            room.apply_bulb_tiles(&mut vram);
        }
        if map_sim.screen_id == 1
            && map_sim.level_progress[7] & 1 != 0
            && map_sim.level_progress[38] & 0x80 != 0
        {
            // `$3D2FC`: bank 14 $4500->$8800, $220 bytes.
            let gate = vram[0x50..0x72].to_vec();
            vram[0x80..0xA2].copy_from_slice(&gate);
        }
        let bg_pal = assets
            .manifest
            .screens
            .iter()
            .find(|s| s.id == map_sim.screen_id)
            .map(|s| s.bg_palette)
            .unwrap_or(0xE4);
        let zone_palettes = map_sim.zone_palettes();
        let initial = (
            zone_palettes.map_or(bg_pal, |p| p.0),
            0xD0,
            zone_palettes.map_or(0x39, |p| p.1),
        );
        let (bg_pal, obp, obj_palette1) = match &self.map_pan_return {
            Some(pan) => {
                let p = pan.palettes();
                (p.0, p.1, initial.2)
            }
            None => self
                .pending_level
                .as_ref()
                .map_or(initial, |t| map_entry_palettes(t.frame, initial)),
        };
        // `$3F203` fills `sStatusBarOAM` from `$7251` on the hub; a zone
        // screen's `$3E277` clears those nine entries first, so its bar
        // has no counters.
        let status_bar = (map_sim.screen_id == 0).then_some(sml2_core::map::StatusBar {
            lives: self.lives,
            coins_low: map_sim.coins_low,
            coins_high: map_sim.coins_high,
        });
        // `$3CFC8` (hub, OAM 13+), `$3E277` (zones, OAM 4+) or the
        // room's own `$6109B` expansion: all run before the cursor,
        // which owns 0-3.
        let decor_cells = if map_sim.screen_id == 0 {
            match assets.sprites {
                Some(t) => sml2_core::map::hub_decor_cells_in(
                    &map_sim.decor_slots,
                    map_sim.cam_x,
                    map_sim.cam_y,
                    &t.hub_decor,
                ),
                None => sml2_core::map::hub_decor_cells(
                    &map_sim.decor_slots,
                    map_sim.cam_x,
                    map_sim.cam_y,
                ),
            }
        } else if let Some(room) = &map_sim.room {
            match assets.sprites {
                Some(tables) => {
                    room.oam_cells_from(&tables.room_metasprites, assets.player_mappings)
                }
                None => room.oam_cells(),
            }
        } else {
            let slots = map_sim.zone_render_slots();
            match assets.sprites {
                Some(t) => sml2_core::map_zone_decor::zone_decor_oam_in(slots, &t.zone_decor),
                None => sml2_core::map_zone_decor::zone_decor_oam(slots),
            }
        };
        let status_cells = status_bar.map(|bar| match assets.sprites {
            Some(t) => bar.cells_in(t.status_bar),
            None => bar.cells(),
        });
        // A Modern hub draws them from the counters layer instead.
        let screen_cells = status_cells.filter(|_| self.hud_style == HudStyle::Classic);
        let zone_live = (1..=8).contains(&map_sim.screen_id);
        let tilemaps = assets
            .tilemaps
            .with_screen_writes(map_sim.screen_id, &map_sim.zone_tile_writes());
        let mut manifest = assets.manifest.clone();
        if zone_live {
            manifest
                .dynamic_unlocks
                .retain(|r| r.screen_id != map_sim.screen_id);
        }
        let transition_palettes = self
            .map_screen_transition
            .as_ref()
            .map(|t| t.palettes(map_sim.screen_id));
        let (bg_pal, obp) = transition_palettes.map_or((bg_pal, obp), |p| (p.0, p.1));
        let video = sml2_core::map::MapVideo {
            scroll_rows: std::array::from_fn(|y| map_sim.zone_background_scroll(y).0),
            scroll_y: map_sim.zone_background_scroll(0).1,
            obj_palette1: transition_palettes.map_or_else(|| obj_palette1, |p| p.2),
        };
        let mut composition = sml2_core::map::compose_overworld_view(
            map_sim.screen_id,
            map_sim.cam_x,
            map_sim.cam_y,
            map_sim.cursor_x,
            map_sim.cursor_y,
            map_sim.cursor_move,
            map_sim.cursor_walking,
            map_sim.walk_frame,
            map_sim.cursor_pose,
            &decor_cells,
            screen_cells.as_ref().map(|c| &c[..]),
            map_sim.scroll_only_latch != 0,
            // `$3C9F4` reads `$FF97`, a frame counter that free-runs from
            // boot; the session's own frame count is the same shape.
            (self.frame_n & 0xFF) as u8,
            &tilemaps,
            &manifest,
            &map_sim.level_progress,
            vw,
            vh,
            Some(&video),
        );
        match layer {
            MapLayer::Counters => {
                composition.bg.fill(None);
                composition.oam = status_cells.map_or_else(Vec::new, |c| c.to_vec());
            }
            // The counters' strip goes with them: the map shows through.
            MapLayer::Screen if status_cells.is_some() && screen_cells.is_none() => {
                let (x0, y0, x1, y1) = sml2_core::map::status_bar_strip_rect(vw, vh);
                for py in y0..y1 {
                    for px in x0..x1 {
                        composition.bg[py * vw + px] = Some(sml2_core::map::hub_bg_sample(
                            &tilemaps,
                            (map_sim.cam_x, map_sim.cam_y),
                            (px, py),
                        ));
                    }
                }
            }
            MapLayer::Screen => {}
        }
        OverworldFrame {
            composition,
            vram,
            bg_palette: bg_pal,
            obj_palette: obp,
            obj_palette1: video.obj_palette1,
        }
    }
}
