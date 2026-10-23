//! One cached atlas serves the tile tray, canvas, and gesture previews.

use super::{DesktopApp, Editor};
use crate::settings::Filter;
use eframe::egui::{self, Color32, ColorImage, Context, Rect, TextureHandle, TextureOptions, Ui};
use sml2_core::level::Metatile;
use sml2_core::video::{self, cgb, PalettePixel, PaletteSource};
use sml2_dx::LevelGraphics;

pub(super) struct Atlas {
    pub texture: TextureHandle,
    pub rows: usize,
    pub layout: Vec<u8>,
    pub key: String,
    pub warning: Option<String>,
}

impl Atlas {
    pub fn uv(&self, id: u8) -> Rect {
        let id = usize::from(id);
        Rect::from_min_max(
            egui::pos2((id % 16) as f32 / 16.0, (id / 16) as f32 / self.rows as f32),
            egui::pos2(
                (id % 16 + 1) as f32 / 16.0,
                (id / 16 + 1) as f32 / self.rows as f32,
            ),
        )
    }
}

pub(super) fn native_tile(
    bg: &[u8],
    bgp: u8,
    metatile: &Metatile,
    filter: Filter,
    header: &[u8; 0x18],
) -> Result<Vec<Color32>, String> {
    let palette = filter.palette(header);
    let mut pixels = vec![Color32::TRANSPARENT; 256];
    for (quad, &tile) in metatile.tiles.iter().enumerate() {
        let offset = video::bg_tile_offset(tile);
        let bytes: [u8; 16] = bg
            .get(offset..offset + 16)
            .ok_or("Scenery tile exceeds the background graphics")?
            .try_into()
            .unwrap();
        let colors = video::tile_colors(&bytes);
        for y in 0..8 {
            for x in 0..8 {
                let shade = (bgp >> (2 * colors[y * 8 + x])) & 3;
                let [r, g, b] = palette.rgb(PalettePixel {
                    source: PaletteSource::Bg,
                    shade,
                });
                pixels[(quad / 2 * 8 + y) * 16 + quad % 2 * 8 + x] = Color32::from_rgb(r, g, b);
            }
        }
    }
    Ok(pixels)
}

pub(super) fn dx_tile(level: &LevelGraphics, id: u8) -> Result<Vec<Color32>, String> {
    let metatile = level
        .metatiles
        .get(id)
        .ok_or("Scenery tile missing from DX graphics")?;
    let palettes = cgb::CgbPalettes::from_bytes(&level.bg_palettes, &level.obj_palettes)
        .map_err(|e| e.to_string())?;
    let banks = [&level.background[0][..], &level.background[1][..]];
    let mut pixels = Vec::with_capacity(256);
    for y in 0..16 {
        for x in 0..16 {
            let tile = metatile.tiles[(y / 8) * 2 + x / 8];
            let attr = cgb::CgbAttributes::from_byte(
                *level
                    .tile_attributes
                    .get(tile as usize)
                    .ok_or("Scenery tile missing DX attributes")?,
            );
            let index = cgb::sample_tile(
                banks,
                video::bg_tile_offset(tile),
                attr,
                (x % 8) as u8,
                (y % 8) as u8,
            )
            .map_err(|e| e.to_string())?;
            let [r, g, b] = cgb::compose_pixel(
                cgb::BackgroundPixel {
                    index,
                    priority: attr.priority(),
                },
                None,
                true,
                &palettes,
            );
            pixels.push(Color32::from_rgb(r, g, b));
        }
    }
    Ok(pixels)
}

impl DesktopApp {
    pub(super) fn editor_atlas(&mut self, ctx: &Context) {
        let key = format!(
            "{}:{:?}:{}:{:?}:{}",
            self.editor.level,
            self.settings.filter,
            self.settings.dx_mode,
            self.settings.dx_installation,
            self.dx_bundle.is_some()
        );
        if self
            .editor
            .atlas
            .as_ref()
            .is_some_and(|atlas| atlas.key == key)
        {
            return;
        }
        let Some(game) = &self.game else {
            return;
        };
        let Ok(content) = game.session.assets.level(self.editor.level) else {
            return;
        };
        let count = content.manifest.metatiles.metatiles.len().min(256);
        let native = || -> Result<Vec<Vec<Color32>>, String> {
            content
                .manifest
                .metatiles
                .metatiles
                .iter()
                .take(count)
                .map(|tile| {
                    native_tile(
                        &content.bg_base,
                        content.manifest.header.palettes[0],
                        tile,
                        self.settings.filter,
                        &game.session.assets.cartridge_header,
                    )
                })
                .collect()
        };
        let dx = self
            .dx_bundle
            .as_ref()
            .filter(|_| self.settings.dx_mode)
            .and_then(|bundle| bundle.level(self.editor.level));
        let mut warning = None;
        let (tiles, dx) = if let Some(level) = dx {
            match (0..count)
                .map(|id| dx_tile(level, id as u8))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(tiles) => (Ok(tiles), Some(level)),
                Err(error) => {
                    warning = Some(format!("DX preview unavailable: {error}"));
                    (native(), None)
                }
            }
        } else {
            (native(), None)
        };
        let tiles = match tiles {
            Ok(tiles) => tiles,
            Err(error) => {
                self.status = Some(error);
                return;
            }
        };
        let rows = count.div_ceil(16).max(1);
        let mut pixels = vec![Color32::TRANSPARENT; 256 * rows * 16];
        for (id, tile) in tiles.iter().enumerate() {
            for y in 0..16 {
                let at = (id / 16 * 16 + y) * 256 + id % 16 * 16;
                pixels[at..at + 16].copy_from_slice(&tile[y * 16..y * 16 + 16]);
            }
        }
        // Match the runtime's DX layout choice: decoration may change only
        // when the original collision class is preserved.
        let layout = content
            .layout
            .cells
            .iter()
            .enumerate()
            .map(|(at, &native)| {
                dx.and_then(|level| level.layout.get(at))
                    .copied()
                    .filter(|&id| {
                        let classes = &content.manifest.metatile_classes;
                        classes.get(id as usize).is_some()
                            && classes.get(id as usize) == classes.get(native as usize)
                    })
                    .unwrap_or(native)
            })
            .collect();
        self.editor.atlas = Some(Atlas {
            texture: ctx.load_texture(
                "scenery-atlas",
                ColorImage::new([256, rows * 16], pixels),
                TextureOptions::NEAREST,
            ),
            rows,
            layout,
            key,
            warning,
        });
    }
}

impl Editor {
    pub(super) fn tile_tray(&mut self, ui: &mut Ui) {
        let Some(work) = &mut self.work else {
            return;
        };
        let Some(atlas) = &self.atlas else {
            return;
        };
        egui::CollapsingHeader::new("Tile tray")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.all_tiles, false, "Used in level");
                    ui.selectable_value(&mut self.all_tiles, true, "All tiles");
                    ui.separator();
                    ui.add(
                        egui::Image::new((atlas.texture.id(), egui::vec2(32.0, 32.0)))
                            .uv(atlas.uv(work.metatile)),
                    );
                    ui.label(format!("Selected tile {}", work.metatile));
                });
                egui::ScrollArea::vertical()
                    .id_salt("scenery-tray")
                    .max_height(82.0)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for id in 0..work.count.min(256) {
                                let id = id as u8;
                                if !self.all_tiles && !work.used.contains(&id) {
                                    continue;
                                }
                                let image =
                                    egui::Image::new((atlas.texture.id(), egui::vec2(32.0, 32.0)))
                                        .uv(atlas.uv(id));
                                if ui
                                    .add(egui::Button::image(image).selected(work.metatile == id))
                                    .on_hover_text(format!("Tile {id}"))
                                    .clicked()
                                {
                                    work.metatile = id;
                                }
                            }
                        });
                    });
            });
    }
}
