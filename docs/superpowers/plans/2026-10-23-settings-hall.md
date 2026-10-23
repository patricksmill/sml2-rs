# Settings Hall Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a long room to the left of the file select where Mario jumps to cycle settings, and a plain settings window on a toolbar that can be hidden.

**Architecture:** The ROM file select does not change. At the left pipe (`cursor_x == 0x28`, `menu_state == 1`) a Left press is consumed by the desktop and opens `Hall` instead of moving the cursor. The hall lives in negative world pixels; world `0` is the doorway back to the pipes. Each jump cycles one shared setting. The toolbar Settings button opens an `egui::Window` with the same chrome as `Debug · F1` and edits those same settings. `show_toolbar` hides `TopBottomPanel::top("toolbar")`. F10 toggles it back, including while the bar is hidden. `Screen::Settings` stays in the code for the filter preview; the toolbar and the pause menu no longer navigate to it.

**Tech Stack:** Rust, `sml2-desktop` (eframe/egui), `Settings` JSON, `render_bg_wide_output` plus `file_select_backdrop` for the hall picture.

## Global Constraints

- Do not change `TitleScreen::step` or `FileSelect::step`. Left at the first pipe is already a no-op in the ROM (`cursor_x >= 0x30` is required to walk left).
- The hall and the window call the same `cycle(Pedestal, &mut Settings)` function.
- A jump cycles forward one step. It does not open a file dialog, capture a binding, or reset all settings. Those three stay buttons in the window.
- Settings JSON version becomes `3`. Missing `show_toolbar` loads as `true`.
- No new files under `captures/` or `notes/`.
- Commits use Conventional Commits.
- Depends on [`2026-10-23-title-is-the-game.md`](2026-10-23-title-is-the-game.md) only for the pause-menu Settings button. This plan still works if that button is the current "Settings" row that calls `navigate(Screen::Settings)`.
- Desktop tests: `cargo test -p sml2-desktop --lib`.

---

### Task 1: One settings list, a plain window, a hideable toolbar

**Files:**
- Create: `crates/sml2-desktop/src/app/catalog.rs`
- Modify: `crates/sml2-desktop/src/settings.rs` (`SETTINGS_VERSION`, `show_toolbar`)
- Modify: `crates/sml2-desktop/src/app/mod.rs` (`settings_open`, toolbar gate, F10, Settings button)
- Modify: `crates/sml2-desktop/src/app/panels.rs` (pause-menu Settings opens the window)
- Test: `crates/sml2-desktop/src/app/catalog.rs`, `crates/sml2-desktop/src/app/tests.rs`

**Interfaces:**
- Consumes: `Settings` fields, `Filter::cycle`, `Widescreen::next`, `CameraZoom::next`, `Zoom`.
- Produces:
  - `Pedestal` with these variants, in this order: `Zoom`, `Fullscreen`, `Filter`, `Volume`, `Mute`, `DeadZone`, `Widescreen`, `CameraZoom`, `Motion`, `DxMode`, `Character`, `Toolbar`, `Debug`.
  - `fn cycle(pedestal: Pedestal, settings: &mut Settings, debug: &mut bool)`
  - `fn label(pedestal: Pedestal, settings: &Settings, debug: bool) -> String` — `"Volume 70%"` form.
  - `Settings.show_toolbar: bool`, default `true`, serialized.
  - `DesktopApp.settings_open: bool`.

- [ ] **Step 1: Write the failing catalog test**

Create `crates/sml2-desktop/src/app/catalog.rs`:

```rust
//! Settings shared by the hall and the plain window. A cycle is one jump.

use super::DesktopApp;
use crate::settings::{CameraZoom, Filter, Settings, Widescreen, Zoom};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pedestal {
    Zoom,
    Fullscreen,
    Filter,
    Volume,
    Mute,
    DeadZone,
    Widescreen,
    CameraZoom,
    Motion,
    DxMode,
    Character,
    Toolbar,
    Debug,
}

pub(super) const PEDESTALS: [Pedestal; 13] = [
    Pedestal::Zoom,
    Pedestal::Fullscreen,
    Pedestal::Filter,
    Pedestal::Volume,
    Pedestal::Mute,
    Pedestal::DeadZone,
    Pedestal::Widescreen,
    Pedestal::CameraZoom,
    Pedestal::Motion,
    Pedestal::DxMode,
    Pedestal::Character,
    Pedestal::Toolbar,
    Pedestal::Debug,
];

pub(super) fn cycle(_pedestal: Pedestal, _settings: &mut Settings, _debug: &mut bool) {}

pub(super) fn label(pedestal: Pedestal, settings: &Settings, debug: bool) -> String {
    let _ = (pedestal, settings, debug);
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_jump_cycles_volume_mute_and_the_toolbar() {
        let mut settings = Settings::default();
        let mut debug = false;
        assert!((settings.volume - 0.7).abs() < f32::EPSILON);
        cycle(Pedestal::Volume, &mut settings, &mut debug);
        assert!((settings.volume - 0.8).abs() < f32::EPSILON);
        for _ in 0..4 {
            cycle(Pedestal::Volume, &mut settings, &mut debug);
        }
        assert!((settings.volume - 0.1).abs() < f32::EPSILON, "100% wraps to 0%");
        cycle(Pedestal::Mute, &mut settings, &mut debug);
        assert!(settings.muted);
        cycle(Pedestal::Toolbar, &mut settings, &mut debug);
        assert!(!settings.show_toolbar);
        cycle(Pedestal::Debug, &mut settings, &mut debug);
        assert!(debug);
        assert_eq!(label(Pedestal::Mute, &settings, debug), "Mute on");
    }
}
```

Add to `crates/sml2-desktop/src/app/tests.rs`:

```rust
#[test]
fn f10_hides_the_toolbar_and_the_settings_button_opens_a_window() {
    let (mut app, ctx, path) = app();
    app.screen = Screen::Game;
    app.settings.show_toolbar = true;
    frame(&mut app, &ctx, vec![key(egui::Key::F10, true)]);
    assert!(!app.settings.show_toolbar);
    app.settings.show_toolbar = true;
    app.input.event(Button::key("F10"), false);
    frame(&mut app, &ctx, vec![]);
    app.settings_open = true;
    frame(&mut app, &ctx, vec![]);
    assert!(app.settings_open);
    assert_ne!(app.screen, Screen::Settings);
    let _ = std::fs::remove_dir_all(path);
}
```

The `key` helper already exists in `tests.rs`. `settings_open` will not compile until Step 3. That compile error is the expected failure of this test. Run the catalog test first.

- [ ] **Step 2: Run the catalog test to verify it fails**

Run: `cargo test -p sml2-desktop --lib app::catalog::tests::a_jump_cycles_volume_mute_and_the_toolbar -- --nocapture`

Expected: FAIL. `volume` is still `0.7` because `cycle` is empty.

- [ ] **Step 3: Cycle the real fields, then show the window**

Replace `cycle` and `label`:

```rust
pub(super) fn cycle(pedestal: Pedestal, settings: &mut Settings, debug: &mut bool) {
    match pedestal {
        Pedestal::Zoom => {
            let current = match settings.zoom {
                Zoom::Auto => 0,
                Zoom::Integer(n) => n as i32,
            };
            let next = (current + 1) % 9;
            settings.zoom = if next == 0 { Zoom::Auto } else { Zoom::Integer(next as u8) };
        }
        Pedestal::Fullscreen => settings.fullscreen = !settings.fullscreen,
        Pedestal::Filter => settings.filter = settings.filter.cycle(1),
        Pedestal::Volume => {
            settings.volume = ((settings.volume * 10.0).round() + 1.0).rem_euclid(11.0) / 10.0;
        }
        Pedestal::Mute => settings.muted = !settings.muted,
        Pedestal::DeadZone => {
            let mut percent = (settings.dead_zone * 100.0).round() as i32 + 5;
            if percent > 80 {
                percent = 10;
            }
            settings.dead_zone = percent as f32 / 100.0;
        }
        Pedestal::Widescreen => settings.widescreen = settings.widescreen.next(),
        Pedestal::CameraZoom => settings.camera_zoom = settings.camera_zoom.next(),
        Pedestal::Motion => settings.motion_smoothing = !settings.motion_smoothing,
        Pedestal::DxMode => settings.dx_mode = !settings.dx_mode,
        Pedestal::Character => {
            settings.character = match settings.character {
                sml2_core::character::Character::Mario => sml2_core::character::Character::Luigi,
                sml2_core::character::Character::Luigi => sml2_core::character::Character::Mario,
            };
        }
        Pedestal::Toolbar => settings.show_toolbar = !settings.show_toolbar,
        Pedestal::Debug => *debug = !*debug,
    }
}

pub(super) fn label(pedestal: Pedestal, settings: &Settings, debug: bool) -> String {
    match pedestal {
        Pedestal::Zoom => format!("Zoom {}", match settings.zoom {
            Zoom::Auto => "auto".to_string(),
            Zoom::Integer(n) => format!("{n}×"),
        }),
        Pedestal::Fullscreen => format!("Fullscreen {}", on(settings.fullscreen)),
        Pedestal::Filter => format!("Filter {}", settings.filter.label()),
        Pedestal::Volume => format!("Volume {:.0}%", settings.volume * 100.0),
        Pedestal::Mute => format!("Mute {}", on(settings.muted)),
        Pedestal::DeadZone => format!("Dead zone {:.0}%", settings.dead_zone * 100.0),
        Pedestal::Widescreen => format!("Widescreen {}", settings.widescreen.label()),
        Pedestal::CameraZoom => format!("Camera {}", settings.camera_zoom.label()),
        Pedestal::Motion => format!("Smoothing {}", on(settings.motion_smoothing)),
        Pedestal::DxMode => format!("DX mode {}", on(settings.dx_mode)),
        Pedestal::Character => format!("Character {}", match settings.character {
            sml2_core::character::Character::Mario => "Mario",
            sml2_core::character::Character::Luigi => "Luigi",
        }),
        Pedestal::Toolbar => format!("Toolbar {}", on(settings.show_toolbar)),
        Pedestal::Debug => format!("Debug {}", on(debug)),
    }
}

fn on(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}
```

`CameraZoom` and `Widescreen` and `Filter` are imported so `label` can call `.label()` and `.next()` is used from `cycle`. Drop an unused import if the compiler warns. `Filter` is used.

In `settings.rs`, set `pub const SETTINGS_VERSION: u32 = 3`. Add the field after `motion_smoothing`:

```rust
    /// Draw the top toolbar. F10 toggles this while it is hidden.
    #[serde(default = "toolbar_shown")]
    pub show_toolbar: bool,
```

Add `show_toolbar: true` to `Default`. Add this function in the same file:

```rust
fn toolbar_shown() -> bool {
    true
}
```

In `Settings::load`, the version-1 migration already writes `value["version"] = SETTINGS_VERSION`. Extend the condition so a version-2 file takes the same path:

```rust
        if value.is_object()
            && (version.is_none()
                || version.and_then(serde_json::Value::as_u64) == Some(1)
                || version.and_then(serde_json::Value::as_u64) == Some(2))
        {
            value["version"] = SETTINGS_VERSION.into();
```

Keep the existing SGB `"Sgb"` rewrite inside that block. `#[serde(default = "toolbar_shown")]` fills `show_toolbar` when the key is absent.

In `app/mod.rs`, add `mod catalog;` next to `mod panels;`. Add `settings_open: bool` to `DesktopApp` and initialize it to `false` in `new`.

In `ui`, show the toolbar only when asked. Replace the call that always shows it:

```rust
        if self.settings.show_toolbar && (self.screen == Screen::Game || self.screen == Screen::Editor) {
            self.toolbar(ctx);
        }
```

The old code shows the toolbar unconditionally for those screens. Delete that unconditional call so the panel is not added twice.

In `shortcuts`, next to the F1 handler:

```rust
        if self.input.take_press(&Button::key("F10")) {
            self.settings.show_toolbar = !self.settings.show_toolbar;
            self.persist();
        }
```

`persist` is the existing method that writes settings. Use it so a hidden bar survives a restart.

In `toolbar`, change the Settings button so it opens the window:

```rust
                if ui.button("Settings").clicked() {
                    self.settings_open = true;
                    release_focus(ctx);
                }
```

Delete the two lines that set `return_screen` and `navigate` to `Screen::Settings` from that button.

Add `settings_window` next to `debug_panel` and call it from `ui` immediately after `self.debug_panel(ctx)`:

```rust
    fn settings_window(&mut self, ctx: &Context) {
        if !self.settings_open {
            return;
        }
        let mut open = self.settings_open;
        egui::Window::new("Settings")
            .open(&mut open)
            .default_width(360.0)
            .show(ctx, |ui| {
                for pedestal in catalog::PEDESTALS {
                    let text = catalog::label(pedestal, &self.settings, self.debug);
                    if ui.button(text).clicked() {
                        if pedestal == catalog::Pedestal::Fullscreen {
                            self.fullscreen(ctx);
                        } else {
                            catalog::cycle(pedestal, &mut self.settings, &mut self.debug);
                        }
                        self.persist();
                        release_focus(ctx);
                    }
                }
                ui.separator();
                ui.label("F10 hides this bar. Bindings, ROM import, and reset stay on the full settings page.");
                if ui.button("All settings").clicked() {
                    self.return_screen = self.screen;
                    self.navigate(ctx, Screen::Settings);
                    release_focus(ctx);
                }
            });
        self.settings_open = open;
    }
```

In `home_panel`, the Settings button currently navigates to `Screen::Settings`. Change that branch to:

```rust
            if self.menu.button(ui, "Settings") {
                self.settings_open = true;
            }
```

Leave `return_screen` for the "All settings" row inside the window. `fullscreen` already flips `settings.fullscreen`, so the window calls it instead of `cycle` for that one pedestal.

- [ ] **Step 4: Run the catalog test and the toolbar test**

Run: `cargo test -p sml2-desktop --lib app::catalog app::tests::f10_hides_the_toolbar_and_the_settings_button_opens_a_window -- --nocapture`

Expected: PASS. Volume goes `0.7 → 0.8 → 0.9 → 1.0 → 0.0 → 0.1`. F10 clears `show_toolbar`. Opening the flag does not switch to `Screen::Settings`.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-desktop/src/app/catalog.rs crates/sml2-desktop/src/app/mod.rs crates/sml2-desktop/src/app/panels.rs crates/sml2-desktop/src/app/tests.rs crates/sml2-desktop/src/settings.rs
git commit -m "$(cat <<'EOF'
feat(desktop): add a plain settings window and a hideable toolbar

F10 hides the bar. The window and the later hall share one cycle list.
EOF
)"
```

---

### Task 2: Walk left from the first pipe into the hall

**Files:**
- Create: `crates/sml2-desktop/src/app/hall.rs`
- Modify: `crates/sml2-desktop/src/app/mod.rs` (field, tick gate)
- Test: `crates/sml2-desktop/src/app/hall.rs`

**Interfaces:**
- Consumes: `catalog::Pedestal`, `catalog::PEDESTALS`, `catalog::cycle`, `PlayerKeys`, `FileSelect::menu_state`, `FileSelect::cursor`.
- Produces:
  - `Hall { mario: i32, hop: u8, goal: Option<usize> }`
  - `fn pedestal_x(index: usize) -> i32` equals `-48 * (index as i32 + 1)`
  - `fn step(hall: &mut Hall, left: bool, right: bool, jump: bool, settings: &mut Settings, debug: &mut bool) -> bool` — `true` means Mario walked through the door (`mario > 0`) and the caller drops the hall
  - `DesktopApp.hall: Option<Hall>`

- [ ] **Step 1: Write the failing hall test**

Create `crates/sml2-desktop/src/app/hall.rs` with the test and a stub `step` that returns `false` and leaves `mario` at 0:

```rust
//! The room to the left of the file select. World 0 is the doorway.

use super::catalog::{self, Pedestal};
use crate::settings::Settings;

pub(super) const GAP: i32 = 48;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Hall {
    pub mario: i32,
    pub hop: u8,
    pub goal: Option<usize>,
}

impl Hall {
    pub(super) fn enter() -> Self {
        Self { mario: 0, hop: 0, goal: None }
    }
}

pub(super) fn pedestal_x(index: usize) -> i32 {
    -GAP * (index as i32 + 1)
}

pub(super) fn step(
    hall: &mut Hall,
    left: bool,
    right: bool,
    jump: bool,
    settings: &mut Settings,
    debug: &mut bool,
) -> bool {
    let _ = (hall, left, right, jump, settings, debug);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walking_left_reaches_volume_and_a_jump_raises_it() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        let volume_at = pedestal_x(
            catalog::PEDESTALS
                .iter()
                .position(|p| *p == Pedestal::Volume)
                .unwrap(),
        );
        for _ in 0..200 {
            if hall.mario == volume_at {
                break;
            }
            assert!(!step(&mut hall, true, false, false, &mut settings, &mut debug));
        }
        assert_eq!(hall.mario, volume_at);
        assert!(!step(&mut hall, false, false, true, &mut settings, &mut debug));
        assert!((settings.volume - 0.8).abs() < f32::EPSILON);
        assert!(hall.hop > 0);
        let during_hop = settings.volume;
        step(&mut hall, false, false, true, &mut settings, &mut debug);
        assert!((settings.volume - during_hop).abs() < f32::EPSILON, "the hop ignores A");
    }

    #[test]
    fn walking_right_through_the_door_leaves() {
        let mut hall = Hall::enter();
        let mut settings = Settings::default();
        let mut debug = false;
        assert!(step(&mut hall, false, true, false, &mut settings, &mut debug));
    }
}
```

- [ ] **Step 2: Run the hall test to verify it fails**

Run: `cargo test -p sml2-desktop --lib app::hall::tests::walking_left_reaches_volume_and_a_jump_raises_it -- --nocapture`

Expected: FAIL. `hall.mario` stays `0`, so it never equals the volume pedestal.

- [ ] **Step 3: Move one pixel per press-frame and cycle on the way down**

Replace `step`:

```rust
pub(super) fn step(
    hall: &mut Hall,
    left: bool,
    right: bool,
    jump: bool,
    settings: &mut Settings,
    debug: &mut bool,
) -> bool {
    if hall.hop > 0 {
        hall.hop -= 1;
        return false;
    }
    if let Some(index) = hall.goal {
        let target = pedestal_x(index);
        if hall.mario < target {
            hall.mario += 1;
        } else if hall.mario > target {
            hall.mario -= 1;
        } else {
            catalog::cycle(catalog::PEDESTALS[index], settings, debug);
            hall.hop = 12;
            hall.goal = None;
        }
        return false;
    }
    if left {
        hall.mario -= 1;
    } else if right {
        hall.mario += 1;
    }
    if hall.mario > 0 {
        return true;
    }
    if jump {
        if let Some(index) = (0..catalog::PEDESTALS.len()).find(|index| {
            (hall.mario - pedestal_x(*index)).abs() <= 8
        }) {
            catalog::cycle(catalog::PEDESTALS[index], settings, debug);
            hall.hop = 12;
        }
    }
    false
}
```

Volume is `PEDESTALS[3]`, at `pedestal_x(3) == -192`. The test walks left one pixel per call, so 192 calls land on it. The loop bound of 200 covers that.

In `app/mod.rs`, add `mod hall;` and `hall: Option<hall::Hall>` on `DesktopApp`, initialized to `None`. Clear it in `navigate`.

Inside the tick loop, after keys are sampled and before `game.tick`:

```rust
                if let Some(mut hall) = self.hall.take() {
                    let jump = keys.a && !self.prev_hall_a;
                    self.prev_hall_a = keys.a;
                    let leave = hall::step(
                        &mut hall,
                        keys.left,
                        keys.right,
                        jump,
                        &mut self.settings,
                        &mut self.debug,
                    );
                    if leave {
                        self.persist();
                    } else {
                        self.hall = Some(hall);
                    }
                    keys = PlayerKeys::default();
                } else if let sml2_runtime::session::SessionState::FileSelect(file_select) =
                    &game.session.state
                {
                    if file_select.menu_state() == 1
                        && file_select.cursor().0 == 0x28
                        && keys.left
                        && !self.prev_hall_left
                    {
                        self.hall = Some(hall::Hall::enter());
                        keys.left = false;
                    }
                    self.prev_hall_left = keys.left;
                }
```

Add `prev_hall_a: bool` and `prev_hall_left: bool` next to `hall`, both starting `false`. `keys` here is the `PlayerKeys` about to be passed to `game.tick`. Clearing `keys` while the hall is open freezes the file select, so the pipe cursor stays at `0x28`. Swallowing `left` on the entry frame keeps the ROM from seeing a press it would ignore anyway.

This block sits where the tick loop can see `keys` and `game`. If Task 3 of the title plan split that loop, put this block in the same place, still before `game.tick`.

- [ ] **Step 4: Run the hall tests**

Run: `cargo test -p sml2-desktop --lib app::hall -- --nocapture`

Expected: PASS. Mario walks to `-192`, volume becomes `0.8`, a second A during `hop` does not change it, and one Right from `0` returns `true`.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-desktop/src/app/hall.rs crates/sml2-desktop/src/app/mod.rs
git commit -m "$(cat <<'EOF'
feat(desktop): walk left from the first pipe into the settings hall

Left at the left pipe freezes the file select. A jump cycles the pedestal underneath.
EOF
)"
```

---

### Task 3: Draw the hall and let a click walk to a pedestal

**Files:**
- Modify: `crates/sml2-desktop/src/app/hall.rs`
- Modify: `crates/sml2-desktop/src/app/mod.rs` (`frame_image`, `game_view` click)
- Test: `crates/sml2-desktop/src/app/hall.rs`

**Interfaces:**
- Consumes: `Hall`, `pedestal_x`, `render_bg_wide_output`, `file_select_backdrop`, `FILE_SELECT_BGP`, `FrontEndAssets::file_select_vram`, `FrontEndAssets::file_select_tilemap`. Confirm those two asset methods exist on `FrontEndAssets` before calling them; the file-select renderer already borrows the same bytes under those names or as `title_vram`-style getters. Use the getters `render_file_select_output` uses.
- Produces: `fn pixels(hall: &Hall, assets: &FrontEndAssets) -> Vec<u8>` — 160×144 palette indices. `fn camera_left(mario: i32) -> i32` is `(mario - 100).min(-20)`. `fn pedestal_at(native_x: u8, mario: i32) -> Option<usize>` sets `hall.goal`.

- [ ] **Step 1: Write the failing picture test**

Add to `hall.rs` tests:

```rust
    #[test]
    fn the_picture_puts_mario_on_the_volume_pedestal() {
        let mut hall = Hall::enter();
        hall.mario = pedestal_x(3);
        let assets = sml2_core::front_end::FrontEndAssets::default();
        let frame = pixels(&hall, &assets);
        assert_eq!(frame.len(), 160 * 144);
        let screen_x = (hall.mario - camera_left(hall.mario)) as usize;
        let mario = frame[100 * 160 + screen_x];
        assert_eq!(mario, 3, "Mario is palette index 3");
        assert_ne!(frame[8 * 160 + 8], 3, "the checker behind him is not Mario");
    }
```

`FrontEndAssets` may not implement `Default`. If it does not, construct it the way `crates/sml2-core/src/front_end/render/tests.rs` builds a file-select fixture (empty vram of `VRAM_LEN` and an empty tilemap of `TILEMAP_LEN`). Do not load `extracted/` in this test.

`pixels` and `camera_left` are missing, so this test fails to compile. Add empty stubs that return `vec![]` and `0` so the assertion fails instead:

```rust
pub(super) fn camera_left(mario: i32) -> i32 {
    let _ = mario;
    0
}

pub(super) fn pixels(_hall: &Hall, _assets: &sml2_core::front_end::FrontEndAssets) -> Vec<u8> {
    Vec::new()
}
```

- [ ] **Step 2: Run the picture test to verify it fails**

Run: `cargo test -p sml2-desktop --lib app::hall::tests::the_picture_puts_mario_on_the_volume_pedestal -- --nocapture`

Expected: FAIL. The frame is empty, so the length assertion fails.

- [ ] **Step 3: Paint the backdrop, the pedestals, and Mario**

```rust
pub(super) fn camera_left(mario: i32) -> i32 {
    (mario - 100).min(-20)
}

pub(super) fn pedestal_at(native_x: u8, mario: i32) -> Option<usize> {
    let world = camera_left(mario) + native_x as i32;
    (0..catalog::PEDESTALS.len()).find(|index| (world - pedestal_x(*index)).abs() <= 12)
}

pub(super) fn pixels(hall: &Hall, assets: &sml2_core::front_end::FrontEndAssets) -> Vec<u8> {
    use sml2_core::front_end::file_select::FILE_SELECT_BGP;
    use sml2_core::front_end::render::render_file_select;
    use sml2_core::front_end::screen::{file_select_backdrop, render_bg_wide_output, TileData};
    use sml2_core::video::OutputMode;
    let native = render_file_select(assets, (160, 144));
    let mut frame = render_bg_wide_output(
        &native,
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        TileData::Signed,
        (160, 144),
        (-camera_left(hall.mario), 0),
        OutputMode::PaletteIndices,
        file_select_backdrop,
    );
    let cam = camera_left(hall.mario);
    for index in 0..catalog::PEDESTALS.len() {
        fill(&mut frame, pedestal_x(index) - cam - 8, 120, 16, 8, 2);
    }
    let hop = if hall.hop > 0 { 10 } else { 0 };
    fill(&mut frame, hall.mario - cam - 4, 104 - hop, 8, 16, 3);
    frame
}

fn fill(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, shade: u8) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if (0..160).contains(&px) && (0..144).contains(&py) {
                frame[(py * 160 + px) as usize] = shade;
            }
        }
    }
}
```

`render_file_select` returns grayscale bytes, and `render_bg_wide_output` copies those bytes inside the native rect. Palette indices are only guaranteed for the backdrop and for `fill`. The Mario pixel in the test is a `fill` of shade 3, and the checker pixel at `(8, 8)` is backdrop. If `render_file_select`'s grayscale values collide with shade 3 inside the native rect, the test point `(screen_x, 100)` must fall on the `fill`, not on the native copy. `camera_left(-192)` is `-292`, and Mario's screen x is `-192 - (-292) = 100`. That is outside the native rect (native starts at view x `292`), so the pixel is the fill. The checker sample `(8, 8)` is also outside the native rect. Both are backdrop-or-fill. Good.

If `file_select_vram` / `file_select_tilemap` have different names, use the accessors in `render_file_select_output` in `crates/sml2-core/src/front_end/render.rs`.

In `frame_image`, before the DX branch:

```rust
        if let Some(hall) = &self.hall {
            let indices = hall::pixels(hall, &game.session.assets.front_end);
            return Some(colorized(
                &indices,
                self.settings.filter,
                &game.session.assets.cartridge_header,
                (160, 144),
            ));
        }
```

`colorized` expects the same palette-index buffer the session renderer returns. If it expects RGB instead, map index `i` through `self.settings.filter` the way `frame_image` already maps `render_palette_frame_smooth`. Match that function's input. Do not invent a second color path.

In `game_view`, when `self.hall` is `Some`, a click inside the picture sets `hall.goal` from `pedestal_at` and does not call `on_picture_click`. The title plan's click handler stays for the title and the pipes.

Paint the current pedestal's `catalog::label` with `ui.painter().text` at the top of `rect`, monospace 14px, white. That is the only text on the picture. The debug window's font is the egui default; use `egui::FontId::monospace(14.0)` here so the caption matches that plain UI.

- [ ] **Step 4: Run the hall tests and the desktop lib**

Run: `cargo test -p sml2-desktop --lib app::hall -- --nocapture`

Expected: PASS. Mario's pixel is index 3. The checker pixel is not.

Then: `cargo test -p sml2-desktop --lib -- --test-threads=8`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-desktop/src/app/hall.rs crates/sml2-desktop/src/app/mod.rs
git commit -m "$(cat <<'EOF'
feat(desktop): draw the settings hall to the left of the pipes

The doorway stays on the right. A click walks Mario to that pedestal and jumps.
EOF
)"
```
