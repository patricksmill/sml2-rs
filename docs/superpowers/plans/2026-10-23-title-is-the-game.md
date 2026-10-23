# Title Is the Game Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Launch into the ROM title, delete the desktop card's duplicate Play / Title buttons and decorative save pipes, and let a mouse click drive the title and the file-select pipes the way the pad already does.

**Architecture:** The in-game title is the title. `TitleScreen` (`$00`) and `FileSelect` (`$19`) already are that screen: START opens the pipes, Mario walks between slots at `0x28` / `0x48` / `0x68`, Down enters the pipe he is standing on, and A at `0x88` is the Clear jump. The desktop Home card is only the pause menu (Settings, scenery editor, Quit). A click is converted to the same `PlayerKeys` the pad produces and handed to the existing session step. `TitleScreen::step` and `FileSelect::step` stay untouched. Quit stays on Esc. The Settings button this plan leaves on the pause menu is replaced by the settings hall and the plain settings window in [`2026-10-23-settings-hall.md`](2026-10-23-settings-hall.md). Do not build a Mario settings room in this plan.

**Tech Stack:** Rust, `sml2-desktop` (eframe/egui), `sml2-core::front_end`, `sml2-runtime` session.

## Global Constraints

- Pointer clicks inject `PlayerKeys` only. Do not change `TitleScreen::step` or `FileSelect::step`.
- Pipe columns stay the ROM cursor positions: `0x28`, `0x48`, `0x68` enter on Down; `0x88` is the Clear jump on A; the DX door is the walk that starts at `0x88` toward `0xC8`.
- A click during the title's first 64 frames (fade high byte still `5`) does nothing.
- Clicks outside `SessionState::Title` and `SessionState::FileSelect` do nothing. A click must not press Start during a level.
- No new files under `captures/` or `notes/`. Do not edit `docs/plans/2026-10-19-menus-save-flow.md` (it records what already shipped).
- Commits use Conventional Commits.
- Desktop tests: `cargo test -p sml2-desktop --lib`.

---

### Task 1: Pause menu stops pretending to be the title

**Files:**
- Modify: `crates/sml2-desktop/src/app/panels.rs` (replace `home_panel`, delete `pipe_stages`)
- Modify: `crates/sml2-desktop/src/app/theme.rs` (delete `save_pipe`, lines 333-375)
- Modify: `crates/sml2-desktop/src/app/tests.rs` (controller test, pipe persistence test)
- Test: `crates/sml2-desktop/src/app/tests.rs`

**Interfaces:**
- Consumes: `DesktopApp::menu`, `DesktopApp::navigate`, `Session::enter_title`, `SaveSlots::load_slot`, `SaveSlots::valid`.
- Produces: Home card buttons, in order, when a game is loaded and the session is not the title or file select: `Settings`, `Scenery editor`, `Quit to title`, `Quit`. On the title, file select, or file-select loading, `Quit to title` is omitted. When no game is loaded, the first button is `Retry loading game`. No `Play / Resume`, no `Title / File Select`, no `SAVE PIPES` row.

- [ ] **Step 1: Write the failing tests**

Replace `controller_can_open_settings_and_confirm_play_without_leaking_a_jump` in `crates/sml2-desktop/src/app/tests.rs` with these two tests. Add `use eframe::egui::ViewportCommand;` next to the existing egui uses if it is not already imported (the file uses `egui::` through `frame`'s return type; import `ViewportCommand` from `eframe::egui`).

```rust
#[test]
fn menu_confirm_opens_settings_and_escape_returns_to_the_game() {
    let (mut app, ctx, path) = app();
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Home);
    app.input.event(Button::Pad(PadButton::South), true);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Settings, "the first menu row is Settings");
    assert_eq!(app.game.as_ref().unwrap().session.frame_n, 0);
    app.input.event(Button::Pad(PadButton::South), false);
    app.input.event(Button::key("Escape"), true);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Home);
    app.input.event(Button::key("Escape"), false);
    app.input.event(Button::key("Escape"), true);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Game);
    let game = app.game.as_ref().unwrap();
    assert!(game.session.pending_level.is_none());
    assert!(matches!(game.session.state, SessionState::Overworld(_)));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn quit_to_title_is_absent_while_the_session_is_on_the_title() {
    let (mut app, ctx, path) = app();
    app.game.as_mut().unwrap().session.enter_title();
    app.screen = Screen::Home;
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), false);
    app.input.event(Button::Pad(PadButton::Down), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), false);
    app.input.event(Button::Pad(PadButton::South), true);
    let output = frame(&mut app, &ctx, vec![]);
    assert!(
        output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .iter()
            .any(|c| matches!(c, ViewportCommand::Close)),
        "Settings, Scenery editor, Quit — the third row closes the window"
    );
    assert_eq!(app.screen, Screen::Home);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn quit_to_title_from_the_map_opens_the_in_game_title() {
    let (mut app, ctx, path) = app();
    // Session::new is the overworld, so the pause menu offers Quit to title.
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), false);
    app.input.event(Button::Pad(PadButton::Down), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), false);
    app.input.event(Button::Pad(PadButton::South), true);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Game);
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));
    let _ = std::fs::remove_dir_all(path);
}
```

In `home_pipes_show_the_saved_stage_tally_across_a_restart`, delete the `slots` closure that calls `pipe_stages`. Assert the record byte directly:

```rust
    let tally = |app: &DesktopApp| {
        let slots = &app.game.as_ref().unwrap().session.slots;
        slots
            .load_slot(0)
            .filter(|_| slots.valid(0))
            .map(|record| record[0x42])
    };
```

Replace `assert_eq!(slots(&app)[0], None, ...)` with `assert_eq!(tally(&app), None, "an unsaved edit does not reach the pipe")`. Replace `assert_eq!(slots(&app)[0], Some(31))` with `assert_eq!(tally(&app), Some(0x31))`. Replace `assert_eq!(slots(&restarted), [Some(31), None, None])` with `assert_eq!(tally(&restarted), Some(0x31))`.

- [ ] **Step 2: Run the menu test to verify it fails**

Run: `cargo test -p sml2-desktop --lib app::tests::menu_confirm_opens_settings_and_escape_returns_to_the_game -- --nocapture`

Expected: FAIL. South still activates `Play / Resume`, so `app.screen` is `Screen::Game`, not `Screen::Settings`.

- [ ] **Step 3: Remove the duplicate title and the pipe widgets**

Delete `pipe_stages` and `use sml2_core::front_end::SaveSlots;` from `panels.rs`. `SessionState` stays imported.

Add this helper above `impl DesktopApp` in `panels.rs`:

```rust
fn on_front_end(state: &SessionState) -> bool {
    matches!(
        state,
        SessionState::Title(_)
            | SessionState::FileSelect(_)
            | SessionState::FileSelectLoading(_)
    )
}
```

Replace `home_panel` with:

```rust
    pub(super) fn home_panel(&mut self, ctx: &Context, ui: &mut Ui) {
        theme::logo(ui, "SUPER MARIO LAND 2", "6 GOLDEN COINS");
        ui.add_space(14.0);
        theme::card().show(ui, |ui| {
            ui.set_width(theme::BUTTON_SIZE.x);
            ui.spacing_mut().item_spacing.y = 6.0;
            if self.game.is_none() {
                if self.menu.button(ui, "Retry loading game") {
                    self.load_game();
                    if self.game.is_some() {
                        self.navigate(ctx, Screen::Game);
                    }
                }
            }
            if self.menu.button(ui, "Settings") {
                self.return_screen = Screen::Home;
                self.navigate(ctx, Screen::Settings);
            }
            if self.game.is_some() && self.menu.button(ui, "Scenery editor") {
                self.open_editor(ctx);
            }
            let quit_to_title = self
                .game
                .as_ref()
                .is_some_and(|game| !on_front_end(&game.session.state));
            if quit_to_title && self.menu.button(ui, "Quit to title") {
                if let Some(game) = &mut self.game {
                    game.session.enter_title();
                }
                self.navigate(ctx, Screen::Game);
            }
            if self.menu.button(ui, "Quit") {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            if let Some(message) = &self.status {
                ui.add_space(6.0);
                theme::error(ui, message);
            }
            if !self.allow_save && self.menu.button(ui, "Reset unreadable settings") {
                self.reset_settings();
            }
        });
    }
```

Delete `save_pipe` from `theme.rs` (the function whose comment is `A green warp pipe standing for a save slot`). Leave the `Sense` import; other painters still use it.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p sml2-desktop --lib app::tests::menu_confirm_opens_settings_and_escape_returns_to_the_game app::tests::quit_to_title_is_absent_while_the_session_is_on_the_title app::tests::quit_to_title_from_the_map_opens_the_in_game_title app::tests::home_pipes_show_the_saved_stage_tally_across_a_restart -- --nocapture`

Expected: PASS. `menu_confirm` still sees `Screen::Home` on the first frame because Task 2 has not changed boot yet. `quit_to_title_from_the_map` reaches `SessionState::Title`. The tally byte `0x42` is BCD `0x31` after save.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-desktop/src/app/panels.rs crates/sml2-desktop/src/app/theme.rs crates/sml2-desktop/src/app/tests.rs
git commit -m "$(cat <<'EOF'
feat(desktop): drop the pause menu's duplicate title and save pipes

The pipes that load a game are the file select. The home card is the pause menu.
EOF
)"
```

---

### Task 2: Boot into the in-game title

**Files:**
- Modify: `crates/sml2-desktop/src/app/mod.rs` (`DesktopApp::new`, the `running` flag in `ui`)
- Modify: `crates/sml2-desktop/src/app/tests.rs`
- Modify: `crates/sml2-desktop/src/app/dx_tests.rs` (the browser-Enter assertions)
- Modify: `crates/sml2-desktop/src/main.rs` (help line)
- Modify: `docs/desktop.md`
- Modify: `docs/feature-inventory.md` (one clause)
- Test: `crates/sml2-desktop/src/app/tests.rs`

**Interfaces:**
- Consumes: `Session::enter_title`, `Session::enter_file_select`, `Options::play`, `Options::boot_title`, `Options::boot_select`. Task 1's pause menu.
- Produces: With a loaded game, startup `screen` is `Screen::Game`. State is `Title` unless `--play` (overworld), `--file-select` (file select), `--room`, or `--reveal`. `--title` stays "same as a normal launch". While `dx_picker` is open the session does not tick.

- [ ] **Step 1: Write the failing boot tests**

Add to `crates/sml2-desktop/src/app/tests.rs`:

```rust
#[test]
fn launch_opens_the_in_game_title() {
    let (app, _, path) = app();
    assert_eq!(app.screen, Screen::Game);
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn play_flag_skips_the_title_for_the_overworld() {
    let (app, _, path) = app();
    let options = Options {
        assets: app.options.assets.clone(),
        paths: AppPaths {
            settings: app.options.paths.settings.clone(),
            save: app.options.paths.save.clone(),
        },
        audio: false,
        filter: None,
        play: true,
        boot_title: false,
        boot_select: false,
        room: None,
        reveal: false,
        scale: None,
        fullscreen: false,
        smoke_frames: None,
        screenshot: None,
    };
    let restarted = DesktopApp::new(&Context::default(), options, Settings::default(), None);
    assert_eq!(restarted.screen, Screen::Game);
    assert!(matches!(
        restarted.game.as_ref().unwrap().session.state,
        SessionState::Overworld(_)
    ));
    let _ = std::fs::remove_dir_all(path);
}
```

`Options` is not `Clone`. The literal above is the whole flag set.

`menu_confirm_opens_settings_and_escape_returns_to_the_game` starts on Home today. After this task the first frame is the title, which ticks. Replace that test with:

```rust
#[test]
fn menu_confirm_opens_settings_and_escape_returns_to_the_title() {
    let (mut app, ctx, path) = app();
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Game);
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));
    app.input.event(Button::key("Escape"), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::key("Escape"), false);
    assert_eq!(app.screen, Screen::Home);
    app.input.event(Button::Pad(PadButton::South), true);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Settings, "the first menu row is Settings");
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));
    let _ = std::fs::remove_dir_all(path);
}
```

`quit_to_title_from_the_map_opens_the_in_game_title` frames once while the session is still the overworld. After this task that first frame is the title, so Quit to title is hidden and two Downs hit Quit. Replace the body so it builds an overworld on purpose:

```rust
#[test]
fn quit_to_title_from_the_map_opens_the_in_game_title() {
    let (mut app, ctx, path) = app();
    app.options.play = true;
    let options = Options {
        assets: app.options.assets.clone(),
        paths: AppPaths {
            settings: app.options.paths.settings.clone(),
            save: app.options.paths.save.clone(),
        },
        audio: false,
        filter: None,
        play: true,
        boot_title: false,
        boot_select: false,
        room: None,
        reveal: false,
        scale: None,
        fullscreen: false,
        smoke_frames: None,
        screenshot: None,
    };
    let mut app = DesktopApp::new(&Context::default(), options, Settings::default(), None);
    app.controllers = Controllers::disconnected();
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::key("Escape"), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::key("Escape"), false);
    assert_eq!(app.screen, Screen::Home);
    app.input.event(Button::Pad(PadButton::Down), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), false);
    app.input.event(Button::Pad(PadButton::Down), true);
    frame(&mut app, &ctx, vec![]);
    app.input.event(Button::Pad(PadButton::Down), false);
    app.input.event(Button::Pad(PadButton::South), true);
    frame(&mut app, &ctx, vec![]);
    assert_eq!(app.screen, Screen::Game);
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));
    let _ = std::fs::remove_dir_all(path);
}
```

In `fullscreen_shortcut_requests_a_native_transition_without_ticking_the_menu` and `binding_capture_updates_controls_without_forwarding_the_capture_press`, set `app.screen = Screen::Home;` immediately after `app()` and before the first `frame` call, so `frame_n` stays 0. Those tests are about the menu, not the title.

In `dx_tests.rs` `system_picker_without_selection_opens_browser_and_preserves_installation`, replace both `assert!(app.screen == Screen::Home, ...)` assertions with:

```rust
    assert_eq!(app.screen, Screen::Game, "browser Enter must not leave the title");
    assert!(matches!(
        app.game.as_ref().unwrap().session.state,
        SessionState::Title(_)
    ));
```

The Escape assertion becomes `assert_eq!(app.screen, Screen::Game);`. Add `use sml2_runtime::session::SessionState;` to `dx_tests.rs` if it is not in scope (`super::tests` does not re-export it).

- [ ] **Step 2: Run the boot test to verify it fails**

Run: `cargo test -p sml2-desktop --lib app::tests::launch_opens_the_in_game_title -- --nocapture`

Expected: FAIL. `app.screen` is `Screen::Home` and the session is `SessionState::Overworld`.

- [ ] **Step 3: Open the title after a successful load**

In `DesktopApp::new`, delete the `let screen = if options.play || ...` block. Set `screen: Screen::Home` in the struct literal.

After `app.load_game();` and `app.restore_dx();`, replace the `boot_select` / `boot_title` / `room` / `reveal` chain with:

```rust
        if app.game.is_some() {
            app.screen = Screen::Game;
            if app.options.boot_select {
                app.game.as_mut().unwrap().session.enter_file_select();
            } else if app.options.boot_title || !app.options.play {
                app.game.as_mut().unwrap().session.enter_title();
            }
            if let Some(room) = app.options.room {
                if let Some(game) = &mut app.game {
                    if room >= 13 {
                        game.session.begin_bell_room(room, 0, 0);
                    } else {
                        game.session.begin_coin_room(room);
                    }
                }
            } else if app.options.reveal {
                if let Some(game) = &mut app.game {
                    let mut seq = sml2_core::castle_sequence::CastleSequence::default();
                    seq.coins = [0x80; 6];
                    game.session.begin_castle_sequence(seq, 8, 0, 0);
                }
            }
        }
```

A failed load stays on Home so Retry and the error stay visible. `--play` without `--title` leaves `Session::new`'s overworld. `--room` and `--reveal` still overwrite whatever the boot branch staged, which is what `main.rs` does today when those flags set `play`.

In `ui`, the session ticks only while the picture is actually playing. Change `running` to:

```rust
        let running = focused
            && self.screen == Screen::Game
            && !self.paused
            && self.capture.is_none()
            && self.dx_picker.is_none()
            && !self.screenshot_pending;
```

In `docs/desktop.md`, after the `./target/debug/sml2-desktop --play` block, add:

```markdown
A normal launch opens the game's own title screen. START, or a click on the
picture, opens the file select: Mario walks to a pipe and ducks into it, and
he jumps on Clear. Esc, or the toolbar Menu button, opens the pause menu for
Settings, the scenery editor, and Quit. Quit to title is on that menu only
after you have left the title. `--play` skips the title and opens the
overworld. `--title` is the same as a normal launch. `--file-select` opens
the pipes directly.
```

In `main.rs`, change the help's first sentence so a reader sees the same rule. Replace the printed string's `--play] [--title]` description by leaving the flag list as it is and appending one sentence before `Esc / Controller Start`:

`Launch opens the in-game title. --play skips to the overworld.`

In `docs/feature-inventory.md`, replace this clause in the `menus-save-flow` row:

`Normal play: Home `Title / File Select` or `--title``

with:

`Normal play: the desktop opens the in-game title; START opens the three pipes. --play skips to the overworld. --title matches a normal launch`

- [ ] **Step 4: Run the desktop lib tests**

Run: `cargo test -p sml2-desktop --lib -- --test-threads=8`

Expected: PASS. `launch_opens_the_in_game_title` sees `Screen::Game` and `SessionState::Title`. `play_flag_skips_the_title_for_the_overworld` sees `SessionState::Overworld`. The DX browser test stays on `Screen::Game` with the title still up after Enter. Fullscreen and binding-capture still report `frame_n == 0` because they sit on Home.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-desktop/src/app/mod.rs crates/sml2-desktop/src/app/tests.rs crates/sml2-desktop/src/app/dx_tests.rs crates/sml2-desktop/src/main.rs docs/desktop.md docs/feature-inventory.md
git commit -m "$(cat <<'EOF'
feat(desktop): boot into the in-game title

The pause menu is no longer the first screen. --play still opens the overworld.
EOF
)"
```

---

### Task 3: A click does what the pad does

**Files:**
- Create: `crates/sml2-desktop/src/app/pointer.rs`
- Modify: `crates/sml2-desktop/src/app/mod.rs` (`mod pointer`, two fields, `navigate`, `game_view`, the tick loop)
- Test: `crates/sml2-desktop/src/app/pointer.rs`

**Interfaces:**
- Consumes: `FileSelect::menu_state`, `FileSelect::cursor`, `TitleScreen::timer`, `render::view::margin`, `PlayerKeys`, Task 2's boot state.
- Produces:
  - `column(native_x: u8, door: bool) -> Aim`
  - `native_point(view: (usize, usize), pixel: (i32, i32)) -> Option<(u8, u8)>`
  - `view_pixel(rect: egui::Rect, view: (usize, usize), pointer: egui::Pos2) -> Option<(i32, i32)>`
  - `aim_keys(aim: Aim, menu_state: u8, cursor_x: u8) -> Option<AimPress>`
  - `Aim` is `Pipe(u8)` (`0x28`, `0x48`, or `0x68`), `Clear`, or `Door`.
  - `AimPress { keys: PlayerKeys, done: bool }`. `done` means this press entered a pipe, started the Clear jump, or started the door walk.

- [ ] **Step 1: Write the failing pointer tests**

Create `crates/sml2-desktop/src/app/pointer.rs` with only the tests and empty types so the test file compiles far enough to fail on missing behavior. The assertions below are the contract. Implement the types in this step exactly, and leave `aim_keys` returning `None` so the walk test fails.

```rust
//! Mouse picks on the title and the file select. The ROM step still owns the motion.

use eframe::egui::{self, Rect};
use sml2_core::front_end::{FileSelect, SaveSlots};
use sml2_core::front_end::keys::{DOWN, LEFT, RIGHT};
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

pub(super) fn view_pixel(rect: Rect, view: (usize, usize), pointer: egui::Pos2) -> Option<(i32, i32)> {
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
    let _ = (aim, menu_state, cursor_x);
    None
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
            let press = aim_keys(Aim::Pipe(0x48), file_select.menu_state(), file_select.cursor().0);
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
```

Confirm `sml2_core::front_end::keys` exports `A`, `DOWN`, `LEFT`, `RIGHT`. They are `pub` constants in `crates/sml2-core/src/front_end/keys.rs`. If the module is private, use the numeric values the file already documents (`RIGHT = 0x10`, `LEFT = 0x20`, `DOWN = 0x80`, `A = 0x01`) and do not make the module public.

- [ ] **Step 2: Run the walk test to verify it fails**

Run: `cargo test -p sml2-desktop --lib app::pointer::tests::clicking_the_middle_pipe_walks_then_ducks_into_it -- --nocapture`

Expected: FAIL with `Mario never reached the middle pipe`, because `aim_keys` returns `None`.

- [ ] **Step 3: Turn a column into one pad press per input frame**

Replace `aim_keys` with:

```rust
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
```

In `app/mod.rs`, add `mod pointer;` next to `mod panels;`.

Add fields to `DesktopApp`, next to `game_rect`:

```rust
    aim: Option<pointer::Aim>,
    start_click: bool,
```

Initialize both in `DesktopApp::new`: `aim: None`, `start_click: false`.

At the end of `navigate`, clear them so a click cannot fire after the menu opens:

```rust
        self.aim = None;
        self.start_click = false;
```

Add this method in `pointer.rs`:

```rust
impl super::DesktopApp {
    pub(super) fn on_picture_click(&mut self, pos: egui::Pos2, rect: egui::Rect) {
        let Some(game) = &self.game else {
            return;
        };
        let view = self.settings.view();
        let Some(pixel) = view_pixel(rect, view, pos) else {
            return;
        };
        let Some((x, _)) = native_point(view, pixel) else {
            return;
        };
        use sml2_runtime::session::SessionState;
        match &game.session.state {
            SessionState::Title(title) if title_accepts_start(title.timer()) => {
                self.start_click = true;
                self.aim = None;
            }
            SessionState::FileSelect(_) => {
                let door = self.settings.dx_mode && self.dx_bundle.is_some();
                self.aim = Some(column(x, door));
                self.start_click = false;
            }
            _ => {}
        }
    }
}
```

In `game_view`, after `self.paint_overlays(ui, rect);`, register the picture as a click target:

```rust
        let picture = ui.interact(rect, egui::Id::new("game-picture"), egui::Sense::click());
        if picture.clicked() {
            if let Some(pos) = picture.interact_pointer_pos() {
                self.on_picture_click(pos, rect);
            }
        }
```

Keep `set_view` and `audio_running` where they are, but end that borrow before the tick loop. Replace the `if let Some(game) = &mut self.game { ... for _ in 0..ticks }` block with:

```rust
        if let Some(game) = &mut self.game {
            game.session.set_view(self.settings.view());
            game.audio_running(running && !self.settings.muted);
        }
        let volume = if self.settings.muted { 0.0 } else { self.settings.volume };
        for _ in 0..ticks {
            let snap = self.front_snap();
            let mut clear_aim = false;
            let mut clear_start = false;
            if let Some(game) = self.game.as_mut() {
                dx::prepare_tick(&mut game.session, &self.settings, self.dx_bundle.is_some());
                let mut keys = if running {
                    self.input.sample_player(&self.settings.bindings)
                } else {
                    PlayerKeys::default()
                };
                let merged = merge_pointer(&snap, &keys, self.aim, self.start_click);
                keys = merged.keys;
                clear_aim = merged.clear_aim;
                clear_start = merged.clear_start;
                game.tick(keys, volume);
                self.settings.character = game.session.character;
            }
            if clear_aim {
                self.aim = None;
            }
            if clear_start {
                self.start_click = false;
            }
        }
```

Add `use sml2_core::player_sim::PlayerKeys;` to `mod.rs` if the loop is the first use.

Add the snapshot and merge next to `aim_keys` in `pointer.rs`:

```rust
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
            return FrontSnap { kind: FrontKind::Other, menu_state: 0, cursor_x: 0 };
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
            _ => FrontSnap { kind: FrontKind::Other, menu_state: 0, cursor_x: 0 },
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
    PointerMerge { keys, clear_aim, clear_start }
}
```

`PlayerKeys` is `Copy`. The `*` in `let mut keys = *keys` is the copy.

`front_snap` for the title does not need the timer. `on_picture_click` already refuses the click while `title_accepts_start` is false, so `start_click` is only set when the title will accept START.

- [ ] **Step 4: Run the pointer tests and the desktop lib**

Run: `cargo test -p sml2-desktop --lib app::pointer -- --nocapture`

Expected: PASS. Middle pipe ends at cursor `0x48` and `menu_state` 5. Clear ends at cursor `0x88` and `menu_state` 4. A pixel in the letterbox bar is `None`.

Then: `cargo test -p sml2-desktop --lib -- --test-threads=8`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-desktop/src/app/pointer.rs crates/sml2-desktop/src/app/mod.rs
git commit -m "$(cat <<'EOF'
feat(desktop): click the title and the file-select pipes

A click becomes the same pad press Mario already obeys. The ROM step is unchanged.
EOF
)"
```
