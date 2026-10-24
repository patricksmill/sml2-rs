//! A still title scene and floating actions over its grass. The preview owns no save
//! or audio output and never advances the title's attract-mode countdown.
use super::{colorized, filter_preview::FilterPreview, DesktopApp, Screen};
use crate::settings::{Filter, Zoom};
use eframe::egui::{self, Color32, ColorImage, Context, Rect, TextureHandle, TextureOptions, Ui};
use sml2_runtime::{
    content::{ExtractedContent, SessionAssets},
    Session,
};

#[derive(PartialEq, Eq)]
struct Key {
    view: (usize, usize),
    filter: Filter,
    dx: bool,
    installation: Option<sml2_dx::cache::CacheKey>,
}

pub(super) struct Launcher {
    session: Session,
    texture: Option<(Key, TextureHandle)>,
    ink: Color32,
}

impl DesktopApp {
    pub(super) fn load_launcher(&mut self) {
        if self.library.vanilla_ready {
            match crate::vanilla_import::vanilla_cache() {
                Ok(path) => self.options.assets = path,
                Err(error) => {
                    self.status = Some(error);
                    return;
                }
            }
        }
        match SessionAssets::load_from(&ExtractedContent::new(&self.options.assets)) {
            Ok(assets) => {
                let (assets, preview) = FilterPreview::new(assets);
                self.filter_preview = preview.ok();
                let mut session = Session::new(assets);
                session.enter_title();
                self.launcher = Some(Launcher {
                    session,
                    texture: None,
                    ink: Color32::WHITE,
                });
            }
            Err(error) => self.status = Some(format!("Title preview could not be loaded: {error}")),
        }
    }

    fn launcher_bar(&mut self, ctx: &Context, grass: Rect) {
        let ink = self
            .launcher
            .as_ref()
            .map_or(Color32::WHITE, |launcher| launcher.ink);
        egui::Area::new(egui::Id::new("launcher-bar"))
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(grass.center())
            .constrain_to(grass)
            .movable(false)
            .fade_in(false)
            .layout(egui::Layout::left_to_right(egui::Align::Center))
            .show(ctx, |ui| {
                ui.spacing_mut().interact_size.y = 36.0;
                ui.spacing_mut().button_padding = egui::vec2(12.0, 8.0);
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(16.0));
                let visuals = ui.visuals_mut();
                visuals.override_text_color = Some(ink);
                visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.hovered.weak_bg_fill = Color32::from_white_alpha(60);
                visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.active.weak_bg_fill = Color32::from_white_alpha(100);
                visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                visuals.selection.bg_fill = if ink == Color32::BLACK {
                    Color32::from_white_alpha(90)
                } else {
                    Color32::from_black_alpha(90)
                };
                visuals.selection.stroke = egui::Stroke::new(1.0_f32, ink);
                if self.screen == Screen::Home {
                    self.library_panel(ctx, ui);
                } else {
                    for label in ["Play", "Scenery Editor", "Settings", "Quit"] {
                        ui.add_enabled(false, egui::Button::new(label));
                    }
                }
                if self.library.checking {
                    ui.weak("Checking installed game…");
                }
            });
    }

    pub(super) fn launcher_view(&mut self, ui: &mut Ui) {
        let available = ui.available_rect_before_wrap();
        let Some(launcher) = &mut self.launcher else {
            ui.centered_and_justified(|ui| {
                ui.weak("Import your ROM in Settings → Game to show the title scene.");
            });
            self.launcher_bar(
                ui.ctx(),
                Rect::from_min_max(
                    egui::pos2(available.left(), available.bottom() - 80.0),
                    available.max,
                ),
            );
            return;
        };
        let ppp = ui.ctx().pixels_per_point();
        let physical = available.size() * ppp;
        let fit = (physical.x / 256.0).min(physical.y / 192.0).max(1.0);
        let scale = match self.settings.zoom {
            Zoom::Auto if self.settings.integer_scaling => fit.floor(),
            Zoom::Auto => fit,
            Zoom::Integer(scale) => f32::from(scale).min(fit).max(1.0),
        };
        let view = (
            (physical.x / scale).ceil().max(160.0) as usize,
            (physical.y / scale).ceil().max(144.0) as usize,
        );
        let dx = self.dx_bundle.as_ref().filter(|_| self.settings.dx_mode);
        let key = Key {
            view,
            filter: self.settings.filter,
            dx: dx.is_some(),
            installation: self.settings.dx_installation.clone(),
        };
        if launcher.texture.as_ref().is_none_or(|(old, _)| *old != key) {
            let image = dx
                .and_then(|dx| {
                    match launcher
                        .session
                        .render_dx_frame(dx, self.settings.character, view)
                    {
                        Ok(rgb) => Some(ColorImage::new(
                            [view.0, view.1],
                            rgb.into_iter()
                                .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                                .collect(),
                        )),
                        Err(error) => {
                            self.dx_message = error;
                            None
                        }
                    }
                })
                .unwrap_or_else(|| {
                    colorized(
                        &launcher.session.render_palette_frame(view),
                        self.settings.filter,
                        &launcher.session.assets.cartridge_header,
                        view,
                    )
                });
            // Keep transparent controls legible with both DX grass and filters.
            let grass = image.pixels.last().copied().unwrap_or(Color32::BLACK);
            let brightness = (u32::from(grass.r()) * 299
                + u32::from(grass.g()) * 587
                + u32::from(grass.b()) * 114)
                / 1000;
            launcher.ink = if brightness > 128 {
                Color32::BLACK
            } else {
                Color32::WHITE
            };
            if let Some((old, texture)) = &mut launcher.texture {
                texture.set(image, TextureOptions::NEAREST);
                *old = key;
            } else {
                launcher.texture = Some((
                    key,
                    ui.ctx()
                        .load_texture("sml2-launcher", image, TextureOptions::NEAREST),
                ));
            }
        }
        let texture = &launcher.texture.as_ref().unwrap().1;
        let size = egui::vec2(view.0 as f32, view.1 as f32) * scale / ppp;
        let destination = Rect::from_center_size(available.center(), size);
        ui.painter().with_clip_rect(available).image(
            texture.id(),
            destination,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        // Row 18 starts the solid grass below the sign and mushrooms. Use the
        // same native origin as both title renderers so the bar follows resizing.
        let native_y = sml2_runtime::render::view::margin(view).1 as f32;
        let grass_top = destination.top() + (native_y + 144.0) * scale / ppp;
        self.launcher_bar(
            ui.ctx(),
            Rect::from_min_max(egui::pos2(available.left(), grass_top), available.max),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Button;
    use sml2_runtime::session::SessionState;

    #[test]
    fn startup_shows_a_still_title_with_floating_actions_in_the_grass_at_every_window_size() {
        let (app, ctx, path) = crate::app::tests::app();
        let mut options = app.options;
        options.launch_library = true;
        let mut app = DesktopApp::new(&ctx, options, Default::default(), None);
        assert!(
            app.game.is_none(),
            "the preview must not create a playable game or touch saves"
        );
        assert!(app.launcher.is_some());
        for size in [
            egui::vec2(480.0, 400.0),
            egui::vec2(800.0, 720.0),
            egui::vec2(1688.0, 984.0),
        ] {
            // The floating area settles its measured size on the first frame.
            for pass in 0..3 {
                let output = ctx.run(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| app.ui(ctx),
                );
                let launcher = app.launcher.as_ref().unwrap();
                let id = launcher.texture.as_ref().unwrap().1.id();
                let scene = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Mesh(mesh) if mesh.texture_id == id => {
                            Some(Rect::from_points(
                                &mesh.vertices.iter().map(|v| v.pos).collect::<Vec<_>>(),
                            ))
                        }
                        _ => None,
                    })
                    .expect("the title is drawn on startup");
                assert!(scene.left() <= 0.0 && scene.right() >= size.x);
                assert!(scene.top() <= 0.0 && scene.bottom() >= size.y);
                if pass == 0 {
                    continue;
                }
                let actions: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => {
                            Some((text.galley.text(), text.visual_bounding_rect()))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    actions.iter().map(|(text, _)| *text).collect::<Vec<_>>(),
                    ["Play", "Scenery Editor", "Settings", "Quit"]
                );
                let view = launcher.texture.as_ref().unwrap().0.view;
                let native_y = sml2_runtime::render::view::margin(view).1 as f32;
                let grass_top = scene.top() + (native_y + 144.0) * scene.height() / view.1 as f32;
                assert!(actions.iter().all(|(_, rect)| rect.min.x >= 0.0
                    && rect.max.x <= size.x
                    && rect.top() >= grass_top
                    && rect.bottom() < size.y));
                let action_rect = actions
                    .iter()
                    .fold(Rect::NOTHING, |rect, (_, action)| rect.union(*action));
                assert!(
                    (action_rect.center().x - size.x / 2.0).abs() < 2.0,
                    "the actions are centered below the sign: {action_rect:?}"
                );
            }
            let launcher = app.launcher.as_ref().unwrap();
            assert_eq!(launcher.session.frame_n, 0);
            let SessionState::Title(title) = &launcher.session.state else {
                panic!("title")
            };
            assert_eq!(
                title.frame(),
                0,
                "idle launcher frames never start attract mode"
            );
        }
        for (key, expected) in [("ArrowRight", 1), ("ArrowLeft", 0), ("ArrowLeft", 3)] {
            app.input.event(Button::key(key), true);
            crate::app::tests::frame(&mut app, &ctx, vec![]);
            assert_eq!(app.menu.selected, expected);
            app.input.event(Button::key(key), false);
        }
        assert!(!path.join("save.sav").exists());
        let _ = std::fs::remove_dir_all(path);
    }
}
