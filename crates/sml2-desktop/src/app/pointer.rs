//! Mouse picks on the title and the file select. The ROM step still owns the motion.

use eframe::egui::{self, Rect};
use sml2_core::front_end::keys::{DOWN, LEFT, RIGHT};
use sml2_core::front_end::{FileSelect, SaveSlots};
use sml2_core::player_sim::PlayerKeys;
use sml2_runtime::render::view::{margin, NATIVE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Aim {
    Pipe(u8),
    Clear,
    Door,
}

pub(super) struct AimPress {
    pub keys: PlayerKeys,
    pub done: bool,
}

pub(super) fn column(native_x: u8, door: bool) -> Aim {
    if native_x < 0x38 {
        Aim::Pipe(0x28)
    } else if native_x < 0x58 {
        Aim::Pipe(0x48)
    } else if native_x < 0x78 {
        Aim::Pipe(0x68)
    } else if !door || native_x < 0xA8 {
        Aim::Clear
    } else {
        Aim::Door
    }
}

pub(super) fn native_point(view: (usize, usize), pixel: (i32, i32)) -> Option<(u8, u8)> {
    let (mx, my) = margin(view);
    let x = pixel.0 - mx;
    let y = pixel.1 - my;
    if (0..160).contains(&x) && (0..144).contains(&y) {
        Some((x as u8, y as u8))
    } else {
        None
    }
}

pub(super) fn view_pixel(
    rect: Rect,
    view: (usize, usize),
    pointer: egui::Pos2,
) -> Option<(i32, i32)> {
    if !rect.contains(pointer) || rect.width() < 1.0 || rect.height() < 1.0 {
        return None;
    }
    let x = ((pointer.x - rect.min.x) / rect.width() * view.0 as f32).floor() as i32;
    let y = ((pointer.y - rect.min.y) / rect.height() * view.1 as f32).floor() as i32;
    Some((x, y))
}

pub(super) fn title_accepts_start(timer: u16) -> bool {
    timer >> 8 < 5
}

pub(super) fn aim_keys(aim: Aim, menu_state: u8, cursor_x: u8) -> Option<AimPress> {
    // States other than 1 are a walk, a hop, or a pipe drop already in progress.
    if menu_state != 1 {
        return None;
    }
    let mut keys = PlayerKeys::default();
    let (target, arrive): (u8, fn(&mut PlayerKeys)) = match aim {
        Aim::Pipe(x) => (x, |keys| keys.down = true),
        Aim::Clear => (0x88, |keys| keys.a = true),
        Aim::Door => {
            keys.right = true;
            return Some(AimPress {
                keys,
                done: cursor_x >= 0x88,
            });
        }
    };
    if cursor_x == target {
        arrive(&mut keys);
        return Some(AimPress { keys, done: true });
    }
    if cursor_x < target {
        keys.right = true;
    } else {
        keys.left = true;
    }
    Some(AimPress { keys, done: false })
}

#[derive(Clone, Copy)]
pub(super) struct FrontSnap {
    kind: FrontKind,
    menu_state: u8,
    cursor_x: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FrontKind {
    Title,
    FileSelect,
    Other,
}

pub(super) struct PointerMerge {
    pub keys: PlayerKeys,
    pub clear_aim: bool,
    pub clear_start: bool,
}

impl super::DesktopApp {
    pub(super) fn front_snap(&self) -> FrontSnap {
        use sml2_runtime::session::SessionState;
        let Some(game) = &self.game else {
            return FrontSnap {
                kind: FrontKind::Other,
                menu_state: 0,
                cursor_x: 0,
            };
        };
        match &game.session.state {
            SessionState::Title(_) => FrontSnap {
                kind: FrontKind::Title,
                menu_state: 0,
                cursor_x: 0,
            },
            SessionState::FileSelect(file_select) => FrontSnap {
                kind: FrontKind::FileSelect,
                menu_state: file_select.menu_state(),
                cursor_x: file_select.cursor().0,
            },
            _ => FrontSnap {
                kind: FrontKind::Other,
                menu_state: 0,
                cursor_x: 0,
            },
        }
    }

    pub(super) fn on_picture_click(&mut self, pos: egui::Pos2, rect: egui::Rect) {
        let Some(game) = &self.game else {
            return;
        };
        let view = self.settings.view();
        let Some(pixel) = view_pixel(rect, view, pos) else {
            return;
        };
        if let sml2_runtime::session::SessionState::FileSelect(file_select) = &game.session.state {
            if file_select.menu_state() == 1 && file_select.cursor().0 == 40 {
                let mut hall = super::hall::Hall::enter();
                if let Some(index) =
                    super::hall::pedestal_at(pixel, &hall, &self.settings, self.debug)
                {
                    hall.goal = Some(index);
                    self.hall = Some(hall);
                    self.aim = None;
                    self.start_click = false;
                    return;
                }
            }
        }
        let Some((x, _)) = native_point(view, pixel) else {
            return;
        };
        use sml2_runtime::session::SessionState;
        match &game.session.state {
            SessionState::Title(title) if title_accepts_start(title.timer()) => {
                self.start_click = true;
                self.aim = None;
            }
            SessionState::FileSelect(file_select) => {
                let door = self.settings.dx_mode && self.dx_bundle.is_some();
                let scroll = if door {
                    file_select.cursor().0.saturating_sub(152)
                } else {
                    0
                };
                self.aim = Some(column(x.saturating_add(scroll), door));
                self.start_click = false;
            }
            _ => {}
        }
    }
}

pub(super) fn merge_pointer(
    snap: &FrontSnap,
    keys: &PlayerKeys,
    aim: Option<Aim>,
    start_click: bool,
) -> PointerMerge {
    let mut keys = *keys;
    let mut clear_aim = false;
    let mut clear_start = false;
    let pad = keys.right
        || keys.left
        || keys.up
        || keys.down
        || keys.a
        || keys.b
        || keys.start
        || keys.select;
    if snap.kind == FrontKind::Title && start_click {
        if !pad {
            keys.start = true;
        }
        clear_start = true;
    }
    if snap.kind == FrontKind::FileSelect {
        if let Some(aim) = aim {
            if pad {
                clear_aim = true;
            } else if let Some(press) = aim_keys(aim, snap.menu_state, snap.cursor_x) {
                keys.right = press.keys.right;
                keys.left = press.keys.left;
                keys.down = press.keys.down;
                keys.a = press.keys.a;
                clear_aim = press.done;
            }
        }
    }
    PointerMerge {
        keys,
        clear_aim,
        clear_start,
    }
}

fn drop_in(file_select: &mut FileSelect, slots: &mut SaveSlots) {
    for _ in 0..45 {
        file_select.step(slots, 0, 0);
    }
    assert_eq!(file_select.menu_state(), 1);
    assert_eq!(file_select.cursor(), (0x28, 0x7C));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_follow_the_rom_cursor_stops() {
        assert_eq!(column(0x20, false), Aim::Pipe(0x28));
        assert_eq!(column(0x40, false), Aim::Pipe(0x48));
        assert_eq!(column(0x60, false), Aim::Pipe(0x68));
        assert_eq!(column(0x90, false), Aim::Clear);
        assert_eq!(column(0xB0, false), Aim::Clear);
        assert_eq!(column(0xB0, true), Aim::Door);
    }

    #[test]
    fn file_select_clicks_follow_the_camera_at_the_character_door() {
        use sml2_runtime::session::SessionState;

        let (mut app, _, path) = super::super::tests::app();
        app.dx_bundle = Some(super::super::dx_tests::red_bundle());
        app.settings.dx_mode = true;
        let session = &mut app.game.as_mut().unwrap().session;
        let mut file_select = FileSelect::new(&mut session.slots);
        for _ in 0..64 {
            file_select.step(&mut session.slots, 0, 0);
        }
        for _ in 0..3 {
            file_select.step(&mut session.slots, RIGHT, 0);
            for _ in 0..32 {
                file_select.step(&mut session.slots, 0, 0);
            }
        }
        session.state = SessionState::FileSelect(file_select);
        super::super::dx::prepare_tick(session, &app.settings, true);
        let SessionState::FileSelect(file_select) = &mut session.state else {
            unreachable!()
        };
        file_select.step(&mut session.slots, RIGHT, 0);
        for _ in 0..65 {
            file_select.step(&mut session.slots, 0, 0);
        }
        assert_eq!(file_select.cursor().0, 200);

        let view = app.settings.view();
        let (mx, my) = margin(view);
        let rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(view.0 as f32, view.1 as f32));
        for (screen_x, aim) in [
            (24, Aim::Pipe(72)),
            (56, Aim::Pipe(104)),
            (88, Aim::Clear),
            (152, Aim::Door),
        ] {
            app.on_picture_click(egui::pos2((mx + screen_x) as f32, (my + 70) as f32), rect);
            assert_eq!(app.aim, Some(aim));
        }
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn clicking_a_visible_hall_sign_from_file_select_starts_the_walk() {
        let (mut app, _, path) = super::super::tests::app();
        app.settings.widescreen = crate::settings::Widescreen::Wide16x9;
        app.settings.camera_zoom = crate::settings::CameraZoom::Zoom200;
        app.game.as_mut().unwrap().session.enter_file_select();
        for _ in 0..64 {
            app.game
                .as_mut()
                .unwrap()
                .session
                .advance(Default::default());
        }
        let view = app.settings.view();
        let (mx, my) = margin(view);
        let rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(view.0 as f32, view.1 as f32));
        app.on_picture_click(egui::pos2((mx - 24) as f32, (my + 72) as f32), rect);
        assert_eq!(app.hall.as_ref().unwrap().goal, Some(1));
        assert_eq!(app.aim, None);
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn the_door_click_walks_until_clear_then_leaves() {
        let walk = aim_keys(Aim::Door, 1, 0x28).unwrap();
        assert!(walk.keys.right);
        assert!(!walk.done);
        let leave = aim_keys(Aim::Door, 1, 0x88).unwrap();
        assert!(leave.keys.right);
        assert!(leave.done);
        assert!(aim_keys(Aim::Door, 2, 0x88).is_none());
    }

    #[test]
    fn letterbox_bars_are_not_buttons() {
        assert_eq!(native_point(NATIVE, (10, 10)), Some((10, 10)));
        assert_eq!(native_point((320, 180), (0, 0)), None);
        let (mx, my) = margin((320, 180));
        assert_eq!(native_point((320, 180), (mx, my)), Some((0, 0)));
    }

    #[test]
    fn the_title_ignores_a_click_during_the_opening_hold() {
        assert!(!title_accepts_start(0x0540));
        assert!(title_accepts_start(0x04FF));
    }

    #[test]
    fn clicking_the_middle_pipe_walks_then_ducks_into_it() {
        let mut slots = SaveSlots::new();
        let mut file_select = FileSelect::new(&mut slots);
        drop_in(&mut file_select, &mut slots);
        let mut guard = 0;
        loop {
            guard += 1;
            assert!(guard < 200, "Mario never reached the middle pipe");
            let press = aim_keys(
                Aim::Pipe(0x48),
                file_select.menu_state(),
                file_select.cursor().0,
            );
            let (held, pressed) = match press {
                Some(press) => {
                    let byte = u8::from(press.keys.right) * RIGHT
                        | u8::from(press.keys.left) * LEFT
                        | u8::from(press.keys.down) * DOWN;
                    if press.done {
                        file_select.step(&mut slots, byte, byte);
                        break;
                    }
                    (byte, byte)
                }
                None => (0, 0),
            };
            file_select.step(&mut slots, held, pressed);
        }
        assert_eq!(file_select.cursor().0, 0x48);
        assert_eq!(file_select.menu_state(), 5, "Down starts the pipe drop");
    }

    #[test]
    fn clicking_clear_jumps_instead_of_entering_a_pipe() {
        let mut slots = SaveSlots::new();
        let mut file_select = FileSelect::new(&mut slots);
        drop_in(&mut file_select, &mut slots);
        let mut guard = 0;
        loop {
            guard += 1;
            assert!(guard < 200, "Mario never jumped on Clear");
            let press = aim_keys(Aim::Clear, file_select.menu_state(), file_select.cursor().0);
            let (held, pressed) = match press {
                Some(press) => {
                    let byte = u8::from(press.keys.right) * RIGHT
                        | u8::from(press.keys.a) * sml2_core::front_end::keys::A;
                    if press.done {
                        file_select.step(&mut slots, byte, byte);
                        break;
                    }
                    (byte, byte)
                }
                None => (0, 0),
            };
            file_select.step(&mut slots, held, pressed);
        }
        assert_eq!(file_select.cursor().0, 0x88);
        assert_eq!(file_select.menu_state(), 4, "A starts the Clear hop");
    }
}
