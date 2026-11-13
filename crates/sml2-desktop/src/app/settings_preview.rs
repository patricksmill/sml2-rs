//! The settings detail pane's live sample: one opening-level frame with Mario,
//! an enemy and the HUD, so BG and both OBJ palettes all show. Frames are
//! rendered once per status bar, widescreen and camera zoom choice, then
//! recolored with the current filter.

use crate::menu::Preview;
use crate::settings::{CameraZoom, Filter, HudStyle, Settings, Widescreen, view_size};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions, Ui};
use sml2_core::video::PalettePixel;
use sml2_runtime::{FrameInput, Session, content::SessionAssets};

/// Ticks into the opening level: past the fade-in, with an enemy on screen.
const SAMPLE_TICKS: u32 = 120;
const HUDS: [HudStyle; 2] = [HudStyle::Classic, HudStyle::Modern];
/// Fixed shapes; "Window size" borrows the one closest to the window.
const SHAPES: [Widescreen; 3] = [Widescreen::Off, Widescreen::Wide16x9, Widescreen::Wide21x9];
const ZOOMS: [CameraZoom; 4] = [
    CameraZoom::Native,
    CameraZoom::Zoom125,
    CameraZoom::Zoom150,
    CameraZoom::Zoom200,
];
/// Native pixels magnified by the integer scaling sample, as x, y, width, height.
const CROP: [usize; 4] = [20, 84, 48, 36];
/// Physical pixels per game pixel in the integer scaling sample; Off adds half.
const MAGNIFY: usize = 4;

type Key = (HudStyle, Widescreen, CameraZoom);

struct Frame {
    key: Key,
    view: (usize, usize),
    pixels: Vec<PalettePixel>,
}

pub(super) struct SettingsPreview {
    frames: Vec<Frame>,
    header: [u8; 0x18],
    texture: Option<(TextureKey, TextureHandle)>,
}

#[derive(Clone, Copy, PartialEq)]
struct TextureKey {
    frame: Key,
    filter: Filter,
    /// The integer scaling sample's physical size, or `None` for the whole frame.
    pixels: Option<[usize; 2]>,
}

impl SettingsPreview {
    /// Run a scratch session to the sample frame. The assets come back
    /// unchanged for the real game; sessions never mutate them.
    pub(super) fn new(assets: SessionAssets) -> (SessionAssets, Result<Self, String>) {
        let header = assets.cartridge_header;
        let mut session = Session::new(assets);
        let frames = session.debug_warp_level(0).map(|()| {
            for _ in 0..SAMPLE_TICKS {
                session.advance(FrameInput::default());
            }
            let mut frames = Vec::new();
            for hud in HUDS {
                session.hud_style = hud;
                for shape in SHAPES {
                    for zoom in ZOOMS {
                        let view = view_size(shape, zoom, [0, 0]);
                        frames.push(Frame {
                            key: (hud, shape, zoom),
                            view,
                            pixels: session.render_palette_frame(view),
                        });
                    }
                }
            }
            frames
        });
        let preview = frames.map(|frames| Self {
            frames,
            header,
            texture: None,
        });
        (session.assets, preview)
    }

    fn frame(&self, key: Key) -> &Frame {
        self.frames
            .iter()
            .find(|frame| frame.key == key)
            .unwrap_or(&self.frames[0])
    }

    /// The sample for the current settings, fitted to the pane. `view` is the live
    /// game view; "Window size" shows the fixed shape closest to it.
    /// [`Preview::Pixels`] magnifies a corner of the native frame at the chosen scaling.
    pub(super) fn show(
        &mut self,
        ui: &mut Ui,
        kind: Preview,
        settings: &Settings,
        view: (usize, usize),
        caption: &str,
    ) {
        let ppp = ui.ctx().pixels_per_point();
        let (key, pixels) = match kind {
            Preview::Frame => (
                (
                    settings.hud_style,
                    closest_shape(settings.widescreen, view),
                    settings.camera_zoom,
                ),
                None,
            ),
            Preview::Pixels => {
                let width = CROP[2] * MAGNIFY
                    - if settings.integer_scaling {
                        0
                    } else {
                        CROP[2] / 2
                    };
                (
                    // Classic keeps Mario at the same native position as `CROP`.
                    (HudStyle::Classic, Widescreen::Off, CameraZoom::Native),
                    Some([width, width * CROP[3] / CROP[2]]),
                )
            }
        };
        let frame = self.frame(key);
        let size = match pixels {
            None => {
                let scale = (ui.available_width() / frame.view.0 as f32).min(2.0);
                egui::vec2(frame.view.0 as f32, frame.view.1 as f32) * scale
            }
            Some([w, h]) => egui::vec2(w as f32, h as f32) / ppp,
        };
        let texture_key = TextureKey {
            frame: frame.key,
            filter: settings.filter,
            pixels,
        };
        if self.texture.as_ref().is_none_or(|(k, _)| *k != texture_key) {
            let image = super::colorized(&frame.pixels, settings.filter, &self.header, frame.view);
            let image = match pixels {
                None => image,
                Some(size) => magnified(&image, frame.view.0, size),
            };
            let texture =
                ui.ctx()
                    .load_texture("sml2-settings-preview", image, TextureOptions::NEAREST);
            self.texture = Some((texture_key, texture));
        }
        let id = self.texture.as_ref().expect("texture just set").1.id();
        ui.vertical_centered(|ui| {
            ui.image((id, size));
            ui.weak(caption);
        });
    }
}

fn closest_shape(widescreen: Widescreen, view: (usize, usize)) -> Widescreen {
    if widescreen != Widescreen::WindowSize {
        return widescreen;
    }
    let aspect = view.0 as f32 / view.1 as f32;
    let distance = |shape| {
        let (w, h) = view_size(shape, CameraZoom::Native, [0, 0]);
        (w as f32 / h as f32 - aspect).abs()
    };
    SHAPES
        .into_iter()
        .min_by(|a, b| distance(*a).total_cmp(&distance(*b)))
        .unwrap_or(Widescreen::Off)
}

/// Nearest-neighbour enlargement of [`CROP`] to exactly `size` physical pixels,
/// so uneven source pixels show as they would on screen.
fn magnified(image: &ColorImage, width: usize, [w, h]: [usize; 2]) -> ColorImage {
    let [x0, y0, cw, ch] = CROP;
    let pixels = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| image.pixels[(y0 + y * ch / h) * width + x0 + x * cw / w])
        .collect::<Vec<Color32>>();
    ColorImage::new([w, h], pixels)
}
