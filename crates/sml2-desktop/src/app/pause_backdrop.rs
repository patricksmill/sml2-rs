//! Preserve the actual submitted picture, including its layout and surroundings,
//! while a game menu is open. Texture uploads resume with gameplay.
use super::DesktopApp;
use eframe::egui::{self, Color32, Rect, Ui};
use sml2_runtime::render::motion::Smooth;

pub(super) struct GameView {
    bounds: Rect,
    shapes: Vec<egui::epaint::ClippedShape>,
}

impl DesktopApp {
    pub(super) fn remember_game_view(&mut self, ui: &mut Ui) {
        let layer = ui.layer_id();
        let start = ui
            .ctx()
            .graphics(|graphics| graphics.get(layer).map_or(0, |list| list.next_idx().0));
        self.game_view(ui);
        let shapes = ui.ctx().graphics(|graphics| {
            graphics
                .get(layer)
                .unwrap()
                .all_entries()
                .skip(start)
                .cloned()
                .collect()
        });
        self.last_game_view = Some(GameView {
            bounds: ui.available_rect_before_wrap(),
            shapes,
        });
    }

    pub(super) fn paused_game_view(&mut self, ui: &mut Ui) {
        let available = ui.available_rect_before_wrap();
        if self.last_game_view.is_none() {
            // A menu can be opened before the first game frame is presented.
            if let Some(image) = self.frame_image(Smooth::OFF) {
                self.texture.set(image, egui::TextureOptions::NEAREST);
            }
            self.remember_game_view(ui);
        } else {
            let still = self.last_game_view.as_ref().unwrap();
            let offset = available.center() - still.bounds.center();
            for clipped in &still.shapes {
                let mut shape = clipped.shape.clone();
                shape.translate(offset);
                ui.painter()
                    .with_clip_rect(clipped.clip_rect.translate(offset).intersect(available))
                    .add(shape);
            }
        }
        ui.painter()
            .rect_filled(available, 0, Color32::from_black_alpha(150));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{
        tests::{app, frame},
        Screen,
    };
    use crate::settings::{Filter, Widescreen, Zoom};

    #[test]
    fn menus_dim_the_last_presented_game_view_and_keep_it_until_resume() {
        for (from_launcher, menu) in [
            (false, Screen::Home),
            (false, Screen::Settings),
            (true, Screen::Home),
            (true, Screen::Settings),
        ] {
            let (mut app, ctx, path) = app();
            // A playtest can leave the launcher available while the game runs.
            app.options.launch_library = from_launcher;
            if from_launcher {
                app.load_launcher();
            }
            app.apply_debug(crate::app::debug::DebugAction::WarpLevel(4));
            frame(&mut app, &ctx, vec![]);
            let displayed = app.last_game_view.as_ref().unwrap().shapes.clone();
            let bounds = app.last_game_view.as_ref().unwrap().bounds;
            let game_rect = app.game_rect;
            let tick = app.game.as_ref().unwrap().session.frame_n;
            app.return_screen = Screen::Game;
            app.navigate(&ctx, menu);
            app.settings.filter = Filter::Amber;
            app.settings.widescreen = Widescreen::Wide16x9;
            app.settings.zoom = Zoom::Integer(1);
            for _ in 0..2 {
                let output = frame(&mut app, &ctx, vec![]);
                assert!(app.menu_from_game);
                assert_eq!(app.game.as_ref().unwrap().session.frame_n, tick);
                assert_eq!(
                    app.game_rect, game_rect,
                    "settings must not change the paused picture's layout"
                );
                assert!(
                    output
                        .textures_delta
                        .set
                        .iter()
                        .all(|(id, _)| *id != app.texture.id()),
                    "the displayed game texture stays frozen"
                );
                for shape in &displayed {
                    assert!(
                        output.shapes.iter().any(|painted| painted == shape),
                        "reuse the submitted scene, including its overlays and surroundings"
                    );
                }
                assert!(output.shapes.iter().any(|painted| matches!(&painted.shape,
                    egui::Shape::Rect(rect) if rect.rect == bounds && rect.fill == Color32::from_black_alpha(150)
                )), "a dark veil covers the scene behind the menu");
                assert!(
                    !output.shapes.iter().any(|painted| matches!(&painted.shape,
                        egui::Shape::Text(text) if text.galley.text() == "Scenery Editor"
                    )),
                    "the launcher bar must not appear over a running game's menu"
                );
            }
            // Escape from either popup resumes; it must not quit the launcher.
            app.input.event(crate::input::Button::key("Escape"), true);
            let resumed = frame(&mut app, &ctx, vec![]);
            assert_eq!(app.screen, Screen::Game);
            assert!(!app.menu_from_game);
            assert!(resumed
                .textures_delta
                .set
                .iter()
                .any(|(id, _)| *id == app.texture.id()));
            assert!(!resumed.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .any(|command| matches!(command, egui::ViewportCommand::Close)));
            let _ = std::fs::remove_dir_all(path);
        }
    }
}
