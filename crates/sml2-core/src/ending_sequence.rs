//! Native ending sequence controller for Super Mario Land 2 (State 19, Table $6801A).
//!
//! Orchestrates the 20 distinct narrative substates across scene loading, walking animations,
//! raster waves, castle tilemap transformation, credits text presentation, and terminal display.
//!
//! Ground truth references:
//! - Entry & dispatcher: ROM $6800F ($400F), Table $6801A ($401A)
//! - Substates 0..3: CPU $4520 ($68520)
//! - Substate 4: CPU $29DD ($029DD)
//! - Substates 5..19: CPU $42EE, $42BA, $43BD, $4149, $4139, $4398, $421E, $41D2,
//!   $41FD, $4063, $4042, $4241, $418B, $409E, $4267
//! - Ending VBlank service: ROM $68528 ($4528)

#[cfg(test)]
mod tests;

use crate::actor::FrameTick;
use crate::credits_text::CreditsTextState;
use crate::ending_scene::EndingScene;
use crate::ending_sprites::{
    append_burst, append_pair, append_secondary, append_walk, EndingSoundEffect,
};
use crate::object::ObjectTables;
use crate::vram_transfer::VramTransfer;

/// 2x3 ending logo tile IDs written by VBlank when ending_tile_flag is set.
pub const ENDING_LOGO_ROW1_OFFSET: usize = 328; // row 10, cols 8..=10 ($9948..$994A)
pub const ENDING_LOGO_ROW2_OFFSET: usize = 360; // row 11, cols 8..=10 ($9968..$996A)
pub const ENDING_LOGO_ROW1_TILES: [u8; 3] = [0x89, 0x8A, 0x8B];
pub const ENDING_LOGO_ROW2_TILES: [u8; 3] = [0x99, 0x9A, 0x9B];

/// Substate count in table $6801A.
pub const SUBSTATE_COUNT: usize = 20;

/// Visible sprite drawing mode requested by a substate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndingSpriteMode {
    None,
    Walk,
    Pair,
    Secondary,
    Burst,
}

/// Execution context for stepping one frame of the ending sequence.
pub struct EndingStepContext<'a> {
    pub scene: &'a mut EndingScene,
    pub rom: Option<&'a [u8]>,
    pub credits_text: Option<&'a [u8]>,
    pub object_tables: Option<&'a ObjectTables>,
    pub decrement_fade: bool,
}

/// Result of stepping one frame of the ending sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndingStepResult {
    pub substate: u8,
    pub frame: u8,
    pub scroll_y: u8,
    pub raster_y: u8,
    pub fade_timer: u16,
    pub sound_updated: bool,
    pub sfx_request_0: u8,
    pub sfx_request_1: u8,
    pub scanline_split_enabled: bool,
    pub terminal: bool,
}

/// State 19 ending sequence controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndingSequenceController {
    pub substate: u8,
    pub frame: u8,
    pub walk_timer: u8,
    /// Last submitted sprite composition, retained for alternate renderers.
    pub sprite_mode: EndingSpriteMode,
    pub sprite_frame: u8,
    pub sprite_timer: u8,
    pub sprite_y: u8,
    pub scroll_y: u8,
    pub raster_y: u8,
    pub fade_timer: u16,
    pub ending_tile_flag: u8,
    pub vram_transfer: Option<VramTransfer>,
    pub credits_text: Option<CreditsTextState>,
    pub sfx_request_0: u8,
    pub sfx_request_1: u8,
    pub sound_update_ticks: u32,
}

impl Default for EndingSequenceController {
    fn default() -> Self {
        Self {
            substate: 0,
            frame: 0,
            walk_timer: 0,
            sprite_mode: EndingSpriteMode::None,
            sprite_frame: 0,
            sprite_timer: 0,
            sprite_y: 0,
            scroll_y: 0,
            raster_y: 13,
            fade_timer: 0,
            ending_tile_flag: 0,
            vram_transfer: None,
            credits_text: None,
            sfx_request_0: 0,
            sfx_request_1: 0,
            sound_update_ticks: 0,
        }
    }
}

impl EndingSequenceController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decrements the fade countdown timer if non-zero.
    pub fn tick_fade_timer(&mut self) {
        if self.fade_timer > 0 {
            self.fade_timer -= 1;
        }
    }

    /// VBlank service at ROM $68528: processes transfers, credits text, and logo tiles.
    fn process_vblank(
        &mut self,
        scene: &mut EndingScene,
        rom: Option<&[u8]>,
        credits_text_slice: Option<&[u8]>,
    ) {
        let transfer_active = self
            .vram_transfer
            .as_ref()
            .is_some_and(|t| t.active && t.remaining_bytes > 0);

        if transfer_active {
            if let Some(t) = self.vram_transfer.as_mut() {
                let has_source = scene.assets().is_some_and(|a| {
                    a.transfer_source(t.source_bank, t.source_addr, t.chunk_len())
                        .is_some()
                });
                if let Some(rom_data) = rom {
                    let _ = t.step_scene(rom_data, scene);
                } else if has_source {
                    let _ = t.step_scene_from_assets(scene);
                } else {
                    // Headless dummy step for unit tests without full ROM
                    let chunk = t.chunk_len() as u16;
                    t.source_addr = t.source_addr.wrapping_add(chunk);
                    t.dest_addr = t.dest_addr.wrapping_add(chunk);
                    t.remaining_bytes = t.remaining_bytes.saturating_sub(chunk);
                    if t.remaining_bytes == 0 {
                        t.active = false;
                    }
                }
            }
        } else if let Some(ct) = self.credits_text.as_mut() {
            if ct.phase != 0 {
                if let Some(assets) = scene.assets_mut() {
                    if let Some(text) = credits_text_slice {
                        let _ = ct.step(text, assets.tilemap_mut());
                    } else {
                        // Without text slice, step phase down for testing
                        ct.phase = ct.phase.saturating_sub(1);
                    }
                }
            } else if self.ending_tile_flag != 0 {
                self.apply_ending_logo(scene);
            }
        } else if self.ending_tile_flag != 0 {
            self.apply_ending_logo(scene);
        }
    }

    fn apply_ending_logo(&mut self, scene: &mut EndingScene) {
        if let Some(assets) = scene.assets_mut() {
            let tm = assets.tilemap_mut();
            if tm.len() >= ENDING_LOGO_ROW2_OFFSET + 3 {
                tm[ENDING_LOGO_ROW1_OFFSET..ENDING_LOGO_ROW1_OFFSET + 3]
                    .copy_from_slice(&ENDING_LOGO_ROW1_TILES);
                tm[ENDING_LOGO_ROW2_OFFSET..ENDING_LOGO_ROW2_OFFSET + 3]
                    .copy_from_slice(&ENDING_LOGO_ROW2_TILES);
            }
        }
        self.ending_tile_flag = 0;
    }

    /// Reproject the last submitted pose with another revision's mappings.
    pub fn projected_cells(&self, tables: &ObjectTables) -> Vec<crate::player::Cell> {
        let mut cells = Vec::new();
        let mut timer = self.sprite_timer;
        let y = self.sprite_y;
        match self.sprite_mode {
            EndingSpriteMode::None => {}
            EndingSpriteMode::Walk => {
                let _ = append_walk(tables, &mut timer, y, &mut cells);
            }
            EndingSpriteMode::Pair => {
                let _ = append_pair(tables, &mut timer, y, &mut cells);
            }
            EndingSpriteMode::Secondary => {
                let _ = append_secondary(tables, y, false, &mut cells);
            }
            EndingSpriteMode::Burst => {
                let _ = append_burst(tables, FrameTick::new(self.sprite_frame as u64), &mut cells);
            }
        }
        cells
    }

    /// Renders sprites into the scene's SpriteBatch.
    fn draw_sprites(
        &mut self,
        mode: EndingSpriteMode,
        tables: Option<&ObjectTables>,
        scene: &mut EndingScene,
    ) {
        self.sprite_mode = mode;
        self.sprite_frame = self.frame;
        self.sprite_timer = self.walk_timer;
        self.sprite_y = scene.credits_sprite_y;
        let mut cells = Vec::new();
        if let Some(t) = tables {
            match mode {
                EndingSpriteMode::None => {}
                EndingSpriteMode::Walk => {
                    let _ =
                        append_walk(t, &mut self.walk_timer, scene.credits_sprite_y, &mut cells);
                }
                EndingSpriteMode::Pair => {
                    let _ =
                        append_pair(t, &mut self.walk_timer, scene.credits_sprite_y, &mut cells);
                }
                EndingSpriteMode::Secondary => {
                    let _ = append_secondary(t, scene.credits_sprite_y, false, &mut cells);
                }
                EndingSpriteMode::Burst => {
                    let comp = append_burst(t, FrameTick::new(self.frame as u64), &mut cells);
                    if let Some(EndingSoundEffect::RequestSound1(sfx)) = comp.sound_effect {
                        self.sfx_request_1 = sfx;
                    }
                }
            }
        } else {
            // Even without ObjectTables, advance walk_timer and sound per source contract
            if mode == EndingSpriteMode::Walk || mode == EndingSpriteMode::Pair {
                self.walk_timer = self.walk_timer.wrapping_add(1);
                if self.walk_timer >= 48 {
                    self.walk_timer = 0;
                }
            }
            if mode == EndingSpriteMode::Burst && (self.frame & 1 != 0) {
                self.sfx_request_1 = 24;
            }
        }
        let _ = scene.objects.write_frame(&cells);
    }

    /// Step the ending sequence by one frame.
    pub fn step(&mut self, context: &mut EndingStepContext<'_>) -> EndingStepResult {
        // Source $02FF: increment frame counter
        self.frame = self.frame.wrapping_add(1);

        // Optional fade countdown tick
        if context.decrement_fade {
            self.tick_fade_timer();
        }

        // VBlank service ($68528)
        self.process_vblank(context.scene, context.rom, context.credits_text);

        let mut sound_updated = false;

        // Substate dispatch (Table $6801A)
        match self.substate {
            0..=3 => {
                // Substates 0..3: advance once per frame
                self.substate += 1;
                self.sprite_mode = EndingSpriteMode::None;
                let _ = context.scene.objects.write_frame(&[]);
            }
            4 => {
                // Substate 4 ($29DD): Scene load
                if context.scene.assets().is_some() {
                    context.scene.scroll_x = 0;
                    context.scene.scroll_y = 0;
                    self.fade_timer = 0x0100;
                    context.scene.credits_sprite_y = 44;
                    self.substate = 5;
                }
                self.sprite_mode = EndingSpriteMode::None;
                let _ = context.scene.objects.write_frame(&[]);
            }
            5 => {
                // Substate 5 ($42EE): Walk draw, sound, wait fade
                self.draw_sprites(EndingSpriteMode::Walk, context.object_tables, context.scene);
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.fade_timer == 0 {
                    self.fade_timer = 0x0080;
                    self.substate = 6;
                }
            }
            6 => {
                // Substate 6 ($42BA): Move credits Y on fade & 3 == 0, walk draw, sound
                if (self.fade_timer as u8) & 0x03 == 0 {
                    context.scene.credits_sprite_y = context.scene.credits_sprite_y.wrapping_add(1);
                }
                self.draw_sprites(EndingSpriteMode::Walk, context.object_tables, context.scene);
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.fade_timer == 0 {
                    self.fade_timer = 0x0020;
                    self.raster_y = 13;
                    self.substate = 7;
                }
            }
            7 => {
                // Substate 7 ($43BD): Raster wobble, advance raster_y to 139
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.frame & 1 != 0 {
                    self.raster_y = self.raster_y.wrapping_add(1);
                }
                if self.raster_y == 139 {
                    self.vram_transfer = Some(VramTransfer::ending_request_2());
                    self.substate = 8;
                }
                self.sprite_mode = EndingSpriteMode::None;
                let _ = context.scene.objects.write_frame(&[]);
            }
            8 => {
                // Substate 8 ($4149): Wobble, wait transfer completion
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.vram_transfer.as_ref().is_none_or(|t| !t.active) {
                    self.vram_transfer = Some(VramTransfer::ending_background_request());
                    self.fade_timer = 0x0200;
                    self.substate = 9;
                }
                self.sprite_mode = EndingSpriteMode::None;
                let _ = context.scene.objects.write_frame(&[]);
            }
            9 => {
                // Substate 9 ($4139): wait for the background transfer ($A248).
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.vram_transfer.as_ref().is_none_or(|t| !t.active) {
                    self.substate = 10;
                }
                self.sprite_mode = EndingSpriteMode::None;
                let _ = context.scene.objects.write_frame(&[]);
            }
            10 => {
                // Substate 10 ($4398): Wobble, retract raster_y to 13
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.frame & 1 != 0 {
                    self.raster_y = self.raster_y.wrapping_sub(1);
                }
                if self.raster_y == 13 {
                    self.fade_timer = 0x00C0;
                    self.substate = 11;
                }
                self.sprite_mode = EndingSpriteMode::None;
                let _ = context.scene.objects.write_frame(&[]);
            }
            11 => {
                // Substate 11 ($421E): Sound, wait fade, trigger ending tile flag
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.fade_timer == 0 {
                    self.fade_timer = 0x0180;
                    self.ending_tile_flag = 0xFF;
                    self.substate = 12;
                }
                self.sprite_mode = EndingSpriteMode::None;
                let _ = context.scene.objects.write_frame(&[]);
            }
            12 => {
                // Substate 12 ($41D2): Sound, burst cycle, odd frame SFX1=24
                sound_updated = true;
                self.sound_update_ticks += 1;
                self.draw_sprites(
                    EndingSpriteMode::Burst,
                    context.object_tables,
                    context.scene,
                );
                if self.fade_timer == 0 {
                    self.fade_timer = 0x0040;
                    self.sfx_request_0 = 7;
                    context.scene.credits_sprite_y = 44;
                    self.substate = 13;
                }
            }
            13 => {
                // Substate 13 ($41FD): Sound, secondary sprite, wait fade
                sound_updated = true;
                self.sound_update_ticks += 1;
                self.draw_sprites(
                    EndingSpriteMode::Secondary,
                    context.object_tables,
                    context.scene,
                );
                if self.fade_timer == 0 {
                    self.fade_timer = 0x0200;
                    self.substate = 14;
                }
            }
            14 => {
                // Substate 14 ($4063): Secondary sprite, sound, wait idle -> transfer 1
                self.draw_sprites(
                    EndingSpriteMode::Secondary,
                    context.object_tables,
                    context.scene,
                );
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.vram_transfer.as_ref().is_none_or(|t| !t.active) {
                    self.vram_transfer = Some(VramTransfer::ending_request_1());
                    self.substate = 15;
                }
            }
            15 => {
                // Substate 15 ($4042): Secondary sprite, sound, wait idle -> SFX0=6, fade 64
                self.draw_sprites(
                    EndingSpriteMode::Secondary,
                    context.object_tables,
                    context.scene,
                );
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.vram_transfer.as_ref().is_none_or(|t| !t.active) {
                    self.sfx_request_0 = 6;
                    self.fade_timer = 0x0040;
                    self.substate = 16;
                }
            }
            16 => {
                // Substate 16 ($4241): Set credits_y = 44, walking pair draw, sound
                context.scene.credits_sprite_y = 44;
                self.draw_sprites(EndingSpriteMode::Pair, context.object_tables, context.scene);
                sound_updated = true;
                self.sound_update_ticks += 1;
                if self.fade_timer == 0 {
                    self.fade_timer = 0x0180;
                    self.substate = 17;
                }
            }
            17 => {
                // Substate 17 ($418B): Walking pair draw, sound, scroll background from timer
                self.draw_sprites(EndingSpriteMode::Pair, context.object_tables, context.scene);
                sound_updated = true;
                self.sound_update_ticks += 1;

                let t = (self.fade_timer >> 4) as u8;
                let scroll_y = (!t).wrapping_add(25);
                self.scroll_y = scroll_y;
                context.scene.scroll_y = scroll_y;
                context.scene.credits_sprite_y = 44u8.wrapping_sub(scroll_y);

                if scroll_y == 24 {
                    self.credits_text = Some(CreditsTextState {
                        ptr: 0,
                        phase: 4,
                        done: false,
                    });
                    self.substate = 18;
                }
            }
            18 => {
                // Substate 18 ($409E): Top/bottom LCD addressing split, wobble, text streaming
                self.draw_sprites(EndingSpriteMode::Pair, context.object_tables, context.scene);
                sound_updated = true;
                self.sound_update_ticks += 1;

                // Wobble condition: (fade_timer_low + 32) < 64
                let wobble_active = (self.fade_timer as u8).wrapping_add(32) < 64;
                self.raster_y = if wobble_active { 139 } else { 13 };

                if self.credits_text.as_ref().is_some_and(|ct| ct.done) {
                    self.substate = 19;
                } else if self.fade_timer == 0 {
                    if let Some(ct) = self.credits_text.as_mut() {
                        ct.start_page();
                    }
                    self.fade_timer = 255;
                }
            }
            19 => {
                // Substate 19 ($4267): Terminal stable display, walking pair draw, sound
                self.draw_sprites(EndingSpriteMode::Pair, context.object_tables, context.scene);
                sound_updated = true;
                self.sound_update_ticks += 1;
                // Terminal state does not advance
            }
            _ => {
                // Out-of-range protection: clamp to terminal
                self.substate = 19;
            }
        }

        EndingStepResult {
            substate: self.substate,
            frame: self.frame,
            scroll_y: self.scroll_y,
            raster_y: self.raster_y,
            fade_timer: self.fade_timer,
            sound_updated,
            sfx_request_0: self.sfx_request_0,
            sfx_request_1: self.sfx_request_1,
            scanline_split_enabled: self.substate >= 18,
            terminal: self.substate == 19,
        }
    }
}
