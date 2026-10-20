//! Read-only session presentation; view dimensions never enter simulation state.

use super::render_view;
use super::{apply_tile_anim, hud_row_coins};
use crate::session::{LevelContext, Session, SessionState, MAP_FADE_STEP};
use std::error::Error;

/// `rBGP`/`rOBP` fade step: subtract `steps` from each 2-bit shade field,
/// clamped at 0. The console's map-to-level fade runs `$90` -> `$40` -> `$00`.
fn fade_palette(pal: u8, steps: u8) -> u8 {
    let mut out = 0u8;
    for i in 0..4 {
        let shift = i * 2;
        let field = (pal >> shift) & 0x03;
        out |= field.saturating_sub(steps) << shift;
    }
    out
}

impl LevelContext {
    /// Render one level view without advancing the session.
    pub fn render_frame(&self, vw: usize, vh: usize) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut bg = self.bg_live.clone();
        apply_tile_anim(
            &mut bg,
            &self.manifest.tile_anim,
            &self.anim_rows,
            self.anim_phase,
        );
        let hud_live = hud_row_coins(&self.hud, self.sim.coins_high, self.sim.coins_low);
        let flash = if self.sim.paused {
            Some(self.sim.pause_flash())
        } else {
            None
        };
        Ok(render_view(
            &self.manifest,
            &self.layout_live,
            &self.sim.layout_writes,
            &bg,
            self.cam.x,
            self.cam.y,
            vw,
            vh,
            Some((&hud_live, self.hud_y)),
            self.spr.as_ref(),
            flash,
        )?)
    }
}

impl Session {
    /// Render current view to an 8-bit grey buffer.
    pub fn render_frame(&self, view: (usize, usize)) -> Vec<u8> {
        let (vw, vh) = view;
        match &self.state {
            SessionState::Title(_) => {
                sml2_core::front_end::render_title(&self.assets.front_end, (vw, vh))
            }
            SessionState::FileSelect(file_select) => sml2_core::front_end::render_file_select(
                file_select,
                &self.slots,
                &self.assets.front_end,
                &self.assets.object_tables,
                (vw, vh),
            ),
            SessionState::Overworld(map_sim) => {
                let map_sim = self
                    .map_screen_transition
                    .as_ref()
                    .filter(|t| t.shows_source())
                    .map_or(map_sim, |t| &t.source);
                let mut vram = if map_sim.screen_id >= 13 && !self.assets.bell_room_tiles.is_empty()
                {
                    // Screens 13/14 copy bank 25 `$5000` to `$8800`; the zone
                    // tilesets at `map/tiles.bin` do not apply.
                    self.assets.bell_room_tiles.vram_tiles()
                } else {
                    self.assets
                        .overworld_tiles
                        .screen_vram_tiles(map_sim.screen_id)
                };
                // `$3F58` copies bank 6 `$4000` over `$8000` in both bonus
                // rooms, so the sprite pass sees those 128 tiles.
                if map_sim.screen_id >= 9 && !self.assets.room_common_tiles.is_empty() {
                    for (index, tile) in self
                        .assets
                        .room_common_tiles
                        .vram_tiles()
                        .iter()
                        .enumerate()
                    {
                        if index < vram.len() {
                            vram[index] = *tile;
                        }
                    }
                }
                if map_sim.screen_id == 0 {
                    if let Some(wave) = &self.map_wave {
                        wave.apply(&mut vram);
                    }
                }
                map_sim.apply_zone_tile_animation(&mut vram, &self.assets.zone_animation_tiles);
                if let Some(room) = map_sim.room.as_ref().filter(|_| (9..=12).contains(&map_sim.screen_id)) {
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
                let bg_pal = self
                    .assets
                    .overworld_manifest
                    .screens
                    .iter()
                    .find(|s| s.id == map_sim.screen_id)
                    .map(|s| s.bg_palette)
                    .unwrap_or(0xE4);
                // While a level entry runs, the console walks every palette
                // field toward shade 0 one step every MAP_FADE_STEP frames and
                // holds the white screen until the level goes live.
                let fade = match self.pending_level.as_ref() {
                    Some(t) if t.frame < MAP_FADE_STEP => 1,
                    Some(_) => 2,
                    None => 0,
                };
                let zone_palettes = map_sim.zone_palettes();
                let bg_pal = zone_palettes.map_or(bg_pal, |p| p.0);
                let (bg_pal, obp) = match &self.map_pan_return {
                    Some(pan) => pan.palettes(),
                    None => (fade_palette(bg_pal, fade), fade_palette(0xD0, fade)),
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
                    sml2_core::map::hub_decor_cells(
                        &map_sim.decor_slots,
                        map_sim.cam_x,
                        map_sim.cam_y,
                    )
                } else if let Some(room) = &map_sim.room {
                    room.oam_cells()
                } else {
                    sml2_core::map_zone_decor::zone_decor_oam(map_sim.zone_render_slots())
                };
                let zone_live = (1..=8).contains(&map_sim.screen_id);
                let tilemaps = self
                    .assets
                    .overworld_tilemaps
                    .with_screen_writes(map_sim.screen_id, &map_sim.zone_tile_writes());
                let mut manifest = self.assets.overworld_manifest.clone();
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
                    obj_palette1: transition_palettes.map_or_else(
                        || fade_palette(zone_palettes.map_or(0x39, |p| p.1), fade),
                        |p| p.2,
                    ),
                };
                sml2_core::map::render_overworld_view_with_video(
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
                    status_bar.as_ref(),
                    map_sim.scroll_only_latch != 0,
                    // `$3C9F4` reads `$FF97`, a frame counter that free-runs from
                    // boot; the session's own frame count is the same shape.
                    (self.frame_n & 0xFF) as u8,
                    &tilemaps,
                    &manifest,
                    &map_sim.level_progress,
                    &vram,
                    bg_pal,
                    obp,
                    vw,
                    vh,
                    Some(&video),
                )
            }
            SessionState::Demo(demo) => demo
                .ctx
                .render_frame(vw, vh)
                .unwrap_or_else(|_| vec![0u8; vw * vh]),
            SessionState::InLevel(ctx) => ctx
                .render_frame(vw, vh)
                .unwrap_or_else(|_| vec![0u8; vw * vh]),
            SessionState::Castle(seq) => sml2_core::map::render_castle_sequence(
                &self.assets.castle_reveal,
                &self.assets.castle_scripts,
                &self.assets.sprite_metas,
                seq,
                (vw, vh),
            ),
            SessionState::GameOver => vec![0u8; vw * vh],
        }
    }
}
