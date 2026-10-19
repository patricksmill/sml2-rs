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
            SessionState::Overworld(map_sim) => {
                let mut vram = self
                    .assets
                    .overworld_tiles
                    .screen_vram_tiles(map_sim.screen_id);
                if map_sim.screen_id == 0 {
                    if let Some(wave) = &self.map_wave {
                        wave.apply(&mut vram);
                    }
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
                let bg_pal = fade_palette(bg_pal, fade);
                let obp = fade_palette(0xD0, fade);
                // `$3F203` fills `sStatusBarOAM` from `$7251` on the hub; a zone
                // screen's `$3E277` clears those nine entries first, so its bar
                // has no counters.
                let status_bar = (map_sim.screen_id == 0).then_some(sml2_core::map::StatusBar {
                    lives: self.lives,
                    coins_low: map_sim.coins_low,
                    coins_high: map_sim.coins_high,
                });
                sml2_core::map::render_overworld_view(
                    map_sim.screen_id,
                    map_sim.cam_x,
                    map_sim.cam_y,
                    map_sim.cursor_x,
                    map_sim.cursor_y,
                    map_sim.cursor_move,
                    map_sim.cursor_walking,
                    map_sim.walk_frame,
                    map_sim.cursor_pose,
                    // `$3CFC8` (hub, OAM 13+) or `$3E277` (zones, OAM 4+): both
                    // are expanded here, before the cursor, which owns 0-3.
                    &if map_sim.screen_id == 0 {
                        sml2_core::map::hub_decor_cells(
                            &map_sim.decor_slots,
                            map_sim.cam_x,
                            map_sim.cam_y,
                        )
                    } else {
                        sml2_core::map_zone_decor::zone_decor_oam(&map_sim.zone_slots)
                    },
                    status_bar.as_ref(),
                    map_sim.scroll_only_latch != 0,
                    // `$3C9F4` reads `$FF97`, a frame counter that free-runs from
                    // boot; the session's own frame count is the same shape.
                    (self.frame_n & 0xFF) as u8,
                    &self.assets.overworld_tilemaps,
                    &self.assets.overworld_manifest,
                    &map_sim.level_progress,
                    &vram,
                    bg_pal,
                    obp,
                    vw,
                    vh,
                )
            }
            SessionState::InLevel(ctx) => ctx
                .render_frame(vw, vh)
                .unwrap_or_else(|_| vec![0u8; vw * vh]),
            SessionState::GameOver => vec![0u8; vw * vh],
        }
    }
}
