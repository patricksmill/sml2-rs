# Settings Hall Room Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the settings hall an open brick room to the left of the pipes, colored by DX when DX mode is on, with Mario marching one station at a time and the current option spelled in the game's own letters.

**Architecture:** The file-select map stays the authority for every tile. `hall_backdrop` reads that map for columns 2–19 and, to the left of column 2, continues the room: the brick checker on rows 0–1 and on rows 16–17, and the open `$FF` cell at map `(4, 4)` in between. There is no pipe floor. Stations stay on the same `0x20` lattice as the save pipes and Clear. The door is `0x28`, and each hall pedestal is `0x20` further left. A direction commits a 32-frame, 1 px/frame march. The sign for the station he is on, or marching toward, is stamped in the sky with the ROM's menu/ending alphabet (see the verified font correction below) and the file-select digit sprites (`$80`–`$89`). The egui caption is removed. DX mode paints the room from `dx.file_select`; the letters are then stamped in that screen's dark background color.

**Tech Stack:** Rust, `sml2-core` front-end backdrop, `sml2-runtime` DX `screen` compositor, `sml2-desktop` hall stepper.

## Implementation status — completed 2026-10-23

- [x] Continue the open room between the matching brick ceiling and floor.
- [x] Commit 32-pixel marches on the pipe lattice, including click arrival and held-input chaining.
- [x] Draw the file-select Mario metasprite in the native hall.
- [x] Use DX file-select graphics/palettes with the display-filter fallback and live doorway.
- [x] Draw every option name and choice in the picture and remove the egui caption.

The detailed procedure below is retained as the original implementation proposal.
These verified corrections supersede its font assumptions and prototype signatures:

- The raw title VRAM at `$50 * 16` contains logo fragments, not an uppercase A;
  the `$50` code in the dormant `GAME CLEARED` text is not a usable direct
  glyph offset in this title asset. The hall uses the existing menu/ending
  font from bank `$1A:$4656` (already loaded for the Easy Mode card). Uppercase
  letters use `ASCII - '0'` (`A = $11`); its high bitplane supplies the letter
  stencil and its low bitplane supplies a background which must remain transparent.
  Digits remain file-select OBJ tiles `$80`–`$89`.
- `hall::alphabet` supplies that existing font to `pixels`, `stamp_sign` and
  `stamp_line`. `stamp_glyph<P: Copy>` shares clipped coverage for palette tokens
  and RGB pixels, avoiding duplicate byte and RGB implementations.
- The file select and hall both use a native 160×144 picture, letterboxed into
  enlarged views, so crossing the doorway preserves the framing. The DX
  file-select camera clamps at zero until the cursor passes x=152; subtracting
  on the unsigned cursor before conversion prevents a negative scroll.
- The hall camera follows each actual one-pixel Mario step, keeping his screen
  anchor at `DOOR` (x=40) and returning continuously to camera zero at the door.
  Starting a march does not move the camera. Both sign lines stay together
  within an eight-pixel screen margin while the destination station approaches.
- File-select mouse picks include the DX camera offset at the character door.
- Hall tests live in `app/hall/tests.rs`; the stepper and presentation stay in
  `app/hall.rs`.

Validation after the camera correction: all 1,683 core tests, 110 desktop tests,
and runtime suites pass with no failures (existing ignored tests remain ignored).
The formerly failing
`file_select_clear_draws_bomb_mario_in_dx_and_restores_the_regular_cursor` now
passes without exclusion. Regression tests cover every hall step in both
directions, complete sign visibility, file-select cursor placement, mouse picks
after scrolling, and matching framing across widescreen/zoom settings in both
rendering modes. The frozen replay sweep exits successfully. Rendered contact
sheets were visually checked for station signs, room geometry, and camera
placement during entry, movement, and return to the door.

## Global Constraints

- Do not change `TitleScreen::step` or `FileSelect::step`. Left at the first pipe stays a desktop-only hall entry.
- Station spacing is `0x20` (32), the same step as the pipes and Clear. The door station is `0x28`. Hall pedestal `i` is at `0x28 - 0x20 * (i + 1)`.
- The hall floor is the brick checker, the same tiles as the ceiling. Do not draw save-pipe tiles in the hall.
- A march is 32 move frames of 1 px, matching `FileSelect::state_left` / `state_right`. Releasing mid-march does not stop it. A new direction is ignored until the march ends.
- Holding the direction chains the next march on the following frame, the way a held pad chains pipe-to-pipe.
- The hall picture is 160×144 and letterboxed with `fixed_screen`. No motion smoothing on it.
- DX colors the hall only when `settings.dx_mode` is on, a bundle is installed, and `session.dx_supported()` is true. Otherwise the same geometry is palette indices through the display filter.
- Standing on the door (`mario == 0x28`, no march, no hop, no click goal) keeps drawing the live file select, cursor and all.
- No new files under `captures/` or `notes/`. Do not load `PORT_QUEUE.done.json`.
- Commits use Conventional Commits.
- The sign uses only `A`–`Z`, digits, and spaces. No `%`, `:`, or `×`.
- Tests: `cargo test -p sml2-core --lib front_end::screen::tests::hall_backdrop_opens_the_room_to_the_left -- --nocapture`, then `cargo test -p sml2-core --lib front_end::render::tests::the_hall_is_open_sky_over_a_brick_floor_and_mario_is_a_sprite -- --nocapture`, then `cargo test -p sml2-desktop --lib app::hall -- --nocapture`, then `cargo test -p sml2-runtime --lib render::dx::tests::dx_hall_colors_the_open_room_and_not_the_bricks -- --nocapture`, then `cargo test -p sml2-desktop --lib app::dx_tests::dx_mode_colors_the_settings_hall -- --nocapture`, then `cargo test -p sml2-desktop --lib app::hall::tests::the_sign_spells_the_station_in_game_letters -- --nocapture`.

---

### Task 1: Continue the brick room to the left of column 2

**Files:**
- Modify: `crates/sml2-core/src/front_end/screen.rs` (after `file_select_backdrop`)
- Test: `crates/sml2-core/src/front_end/screen/tests.rs`

**Interfaces:**
- Consumes: `file_select_backdrop`.
- Produces: `pub fn hall_backdrop(map: &[u8], tx: i32, ty: i32, line: usize) -> ((usize, usize), usize)`. Columns `2..20` and rows `0..18` return that map cell. Elsewhere, rows `0..2` and rows `16..18` return the brick checker `(tx.rem_euclid(2), ty.rem_euclid(2))`. Rows `2..16` return the open cell `(4, 4)` (tile `$FF` on the real map). Anything outside rows `0..18` delegates to `file_select_backdrop`.

- [ ] **Step 1: Write the failing backdrop test**

Add `hall_backdrop` as a stub at the bottom of `file_select_backdrop` in `crates/sml2-core/src/front_end/screen.rs`:

```rust
pub fn hall_backdrop(map: &[u8], tx: i32, ty: i32, line: usize) -> ((usize, usize), usize) {
    let _ = (map, tx, ty);
    ((0, 0), line)
}
```

Add to `crates/sml2-core/src/front_end/screen/tests.rs`:

```rust
#[test]
fn hall_backdrop_opens_the_room_to_the_left() {
    let map = vec![0u8; TILEMAP_LEN];
    // The three save pipes and Clear stay the real map.
    assert_eq!(hall_backdrop(&map, 10, 4, 1), ((10, 4), 1));
    assert_eq!(hall_backdrop(&map, 2, 16, 0), ((2, 16), 0));
    // Left of column 2: open sky, brick ceiling, brick floor. Not pipes.
    assert_eq!(hall_backdrop(&map, 0, 4, 2), ((4, 4), 2));
    assert_eq!(hall_backdrop(&map, -1, 4, 2), ((4, 4), 2));
    assert_eq!(hall_backdrop(&map, -1, 0, 3), ((1, 0), 3));
    assert_eq!(hall_backdrop(&map, -2, 16, 7), ((0, 0), 7));
    assert_eq!(hall_backdrop(&map, -1, 16, 0), ((1, 0), 0));
    assert_eq!(hall_backdrop(&map, 0, 17, 4), ((0, 1), 4));
    assert_eq!(hall_backdrop(&map, 1, 17, 4), ((1, 1), 4));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sml2-core --lib front_end::screen::tests::hall_backdrop_opens_the_room_to_the_left -- --nocapture`

Expected: FAIL. `hall_backdrop(&map, 10, 4, 1)` is `((0, 0), 1)`.

- [ ] **Step 3: Implement the room continuation**

Replace the stub:

```rust
/// The settings hall to the left of the file select. Columns 2–19 are the
/// map. Further left, rows 0–1 and 16–17 are the brick checker and rows
/// 2–15 are the open `$FF` cell at map `(4, 4)`.
pub fn hall_backdrop(map: &[u8], tx: i32, ty: i32, line: usize) -> ((usize, usize), usize) {
    if (2..20).contains(&tx) && (0..18).contains(&ty) {
        return ((tx as usize, ty as usize), line);
    }
    if !(0..18).contains(&ty) {
        return file_select_backdrop(map, tx, ty, line);
    }
    let row = ty as usize;
    if row < 2 || row >= 16 {
        return ((tx.rem_euclid(2) as usize, row.rem_euclid(2)), line);
    }
    ((4, 4), line)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p sml2-core --lib front_end::screen::tests::hall_backdrop_opens_the_room_to_the_left -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-core/src/front_end/screen.rs crates/sml2-core/src/front_end/screen/tests.rs
git commit -m "$(cat <<'EOF'
fix(core): continue the file-select bricks to the left of the pipes

The hall keeps the brick ceiling and a brick floor, and leaves the space between them open.
EOF
)"
```

---

### Task 2: March one pipe per direction, on the pipe lattice

**Files:**
- Modify: `crates/sml2-desktop/src/app/hall.rs`
- Modify: `crates/sml2-desktop/src/app/mod.rs` (`pedestal_at` call in `game_view`)
- Test: `crates/sml2-desktop/src/app/hall.rs`

**Interfaces:**
- Consumes: `catalog::HALL_PEDESTALS`, `catalog::cycle`.
- Produces:
  - `pub(super) const STEP: i32 = 0x20`
  - `pub(super) const DOOR: i32 = 0x28`
  - `Hall { mario, hop, goal, walk: i8, walk_timer: u8, facing: i8, camera: i32 }`
  - `fn pedestal_x(index: usize) -> i32` = `DOOR - STEP * (index as i32 + 1)`
  - `fn pedestal_at(native_x: u8, hall: &Hall) -> Option<usize>` using `hall.camera + native_x`
  - `Hall::enter()` stands at `DOOR`, `facing = -1`, `camera = 0`, `walk = 0`
  - `fn step(...) -> bool` as below. `true` means leave, and only when standing on `DOOR` with Right.

- [ ] **Step 1: Write the failing march test**

In `hall.rs`, add the new fields to `Hall` and to `Hall::enter()` so the test compiles. Leave `enter()`'s `mario` at `0` and `step` unchanged; the new assertions fail.

```rust
pub walk: i8,
pub walk_timer: u8,
pub facing: i8,
pub camera: i32,
```

`enter()` sets those four to `0`, `-1`, `0` (`facing` is `-1`). Add the constants `STEP` and `DOOR` next to `GAP`. Keep `GAP` until Step 3 deletes it.

Add this test:

```rust
#[test]
fn a_held_direction_marches_one_pipe_and_stops_on_the_lattice() {
    let mut hall = Hall::enter();
    let mut settings = Settings::default();
    let mut debug = false;
    assert!(!step(&mut hall, true, false, false, &mut settings, &mut debug));
    assert_eq!(hall.mario, DOOR, "the decision frame does not slide");
    assert_eq!(hall.walk, -STEP as i8);
    for _ in 0..31 {
        assert!(!step(
            &mut hall, false, false, false, &mut settings, &mut debug
        ));
    }
    assert_ne!(hall.walk, 0, "releasing does not cancel the march");
    assert!(!step(
        &mut hall, false, false, false, &mut settings, &mut debug
    ));
    assert_eq!(hall.mario, pedestal_x(0));
    assert_eq!(hall.walk, 0);
    let rested = hall.mario;
    assert!(!step(
        &mut hall, false, false, false, &mut settings, &mut debug
    ));
    assert_eq!(hall.mario, rested, "he stands on the pipe");
    assert!((0..160).contains(&(hall.mario - hall.camera)));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sml2-desktop --lib app::hall::tests::a_held_direction_marches_one_pipe_and_stops_on_the_lattice -- --nocapture`

Expected: FAIL. `hall.mario` is `-1`, not `DOOR` (`40`).

- [ ] **Step 3: Replace free sliding with 32-frame marches**

Delete `GAP`. Replace `pedestal_x`, `step`, `camera_left`, and `pedestal_at`:

```rust
pub(super) const STEP: i32 = 0x20;
pub(super) const DOOR: i32 = 0x28;

pub(super) fn pedestal_x(index: usize) -> i32 {
    DOOR - STEP * (index as i32 + 1)
}

fn furthest() -> i32 {
    pedestal_x(catalog::HALL_PEDESTALS.len() - 1)
}

fn aim_camera(mario: i32, camera: i32, dir: i8) -> i32 {
    let dest = mario + i32::from(dir) * STEP;
    let dest_screen = dest - camera;
    if (8..152).contains(&dest_screen) {
        camera
    } else if dir < 0 {
        dest - 48
    } else {
        (dest - 112).min(0)
    }
}

fn begin_walk(hall: &mut Hall, dir: i8) {
    hall.facing = dir;
    hall.walk = dir * STEP as i8;
    hall.camera = aim_camera(hall.mario, hall.camera, dir);
}

fn land(hall: &mut Hall, settings: &mut Settings, debug: &mut bool) {
    let Some(index) = hall.goal else {
        return;
    };
    if pedestal_x(index) != hall.mario {
        return;
    }
    catalog::cycle(catalog::HALL_PEDESTALS[index], settings, debug);
    hall.hop = 12;
    hall.goal = None;
}

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
    if hall.walk != 0 {
        let dir = hall.walk.signum();
        hall.mario += i32::from(dir);
        hall.walk -= dir;
        hall.walk_timer = hall.walk_timer.wrapping_add(1);
        if hall.walk_timer >= 0x0C {
            hall.walk_timer = 0;
        }
        if hall.walk == 0 {
            land(hall, settings, debug);
        }
        return false;
    }
    if left || right {
        hall.goal = None;
    }
    if let Some(index) = hall.goal {
        let target = pedestal_x(index);
        if hall.mario == target {
            land(hall, settings, debug);
        } else {
            begin_walk(hall, if hall.mario > target { -1 } else { 1 });
        }
        return false;
    }
    if right && hall.mario == DOOR {
        return true;
    }
    if left && hall.mario > furthest() {
        begin_walk(hall, -1);
    } else if right && hall.mario < DOOR {
        begin_walk(hall, 1);
    } else if jump {
        if let Some(index) = (0..catalog::HALL_PEDESTALS.len())
            .find(|index| hall.mario == pedestal_x(*index))
        {
            catalog::cycle(catalog::HALL_PEDESTALS[index], settings, debug);
            hall.hop = 12;
        }
    }
    false
}

pub(super) fn camera_left(hall: &Hall) -> i32 {
    hall.camera
}

pub(super) fn pedestal_at(native_x: u8, hall: &Hall) -> Option<usize> {
    let world = hall.camera + i32::from(native_x);
    (0..catalog::HALL_PEDESTALS.len()).find(|index| (world - pedestal_x(*index)).abs() <= 12)
}
```

`Hall::enter()`:

```rust
pub(super) fn enter() -> Self {
    Self {
        mario: DOOR,
        hop: 0,
        goal: None,
        walk: 0,
        walk_timer: 0,
        facing: -1,
        camera: 0,
    }
}
```

`pixels` still calls `camera_left(hall.mario)`. Change that call, and the picture test's call, to `camera_left(hall)`. The rectangle picture is replaced in Task 3; this only keeps it compiling. `camera_left` now takes `&Hall`.

In `crates/sml2-desktop/src/app/mod.rs`, the click handler calls `hall::pedestal_at(x, hall.mario)`. Change it to `hall::pedestal_at(x, hall)`.

Add this test under the march test:

```rust
#[test]
fn every_march_stays_on_the_native_screen_and_a_click_hops_on_arrival() {
    let mut hall = Hall::enter();
    let mut settings = Settings::default();
    let mut debug = false;
    for _ in 0..catalog::HALL_PEDESTALS.len() {
        for _ in 0..33 {
            assert!(!step(
                &mut hall, true, false, false, &mut settings, &mut debug
            ));
            let screen = hall.mario - hall.camera;
            assert!(
                (0..160).contains(&screen),
                "screen x {screen} at mario {} camera {}",
                hall.mario,
                hall.camera
            );
        }
    }
    assert_eq!(hall.mario, furthest());
    assert_eq!(hall.walk, 0);
    step(&mut hall, true, false, false, &mut settings, &mut debug);
    assert_eq!(hall.mario, furthest(), "the last pipe does not march further");
    assert_eq!(hall.walk, 0);

    let mut hall = Hall::enter();
    hall.goal = Some(0);
    step(&mut hall, false, false, false, &mut settings, &mut debug);
    for _ in 0..31 {
        step(&mut hall, false, false, false, &mut settings, &mut debug);
    }
    assert_eq!(hall.hop, 0);
    step(&mut hall, false, false, false, &mut settings, &mut debug);
    assert_eq!(hall.mario, pedestal_x(0));
    assert!(hall.hop > 0);
    assert_eq!(settings.zoom, crate::settings::Zoom::Integer(1));
}
```

`walking_left_reaches_volume_and_a_jump_raises_it` and `walking_right_through_the_door_leaves` stay. Volume is index 3 in both `PEDESTALS` and `HALL_PEDESTALS`, now at `0x28 - 0x80 = -88`. Four marches are 132 calls; the loop bound of 200 still covers it. Right from `enter()` is Right from the door, so it still leaves.

`the_doorway_frame_is_palette_indices` asserts `hall.mario == 0`. Change that assertion to `assert_eq!(hall.mario, DOOR)`.

- [ ] **Step 4: Run the hall tests**

Run: `cargo test -p sml2-desktop --lib app::hall -- --nocapture`

Expected: PASS. One left press-and-release lands on pedestal 0 (`8`) and stops. Twelve left marches stay inside screen x `0..160` and stop on the last pedestal. A click goal on Zoom arrives and sets zoom to `1×`. Volume still rises from `0.7` to `0.8`. Right on the door leaves.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-desktop/src/app/hall.rs crates/sml2-desktop/src/app/mod.rs
git commit -m "$(cat <<'EOF'
fix(desktop): march the settings hall one pipe at a time

Left or right commits a 32-pixel walk on the same lattice as the save pipes and Clear, and finishes on a station.
EOF
)"
```

---

### Task 3: Draw the open room and the file-select Mario

**Files:**
- Modify: `crates/sml2-core/src/front_end/screen.rs` (add `render_backdrop_output`)
- Modify: `crates/sml2-core/src/front_end/render.rs` (add `render_hall_output`)
- Modify: `crates/sml2-core/src/front_end/mod.rs` (re-export `render_hall_output`)
- Test: `crates/sml2-core/src/front_end/render/tests.rs`
- Modify: `crates/sml2-desktop/src/app/hall.rs` (`pixels`, delete `fill`)
- Modify: `crates/sml2-desktop/src/app/mod.rs` (`frame_image` passes object tables)

**Interfaces:**
- Consumes: `hall_backdrop`, `FILE_SELECT_BGP`, `draw_metasprite` (private in `render.rs`), `ObjectTables`, `OutputMode`.
- Produces:
  - `pub fn render_backdrop_output(vram, tilemap, bgp, tile_data, view, scroll_x, output, backdrop) -> Vec<u8>` — every pixel goes through `backdrop`; there is no native-rect copy.
  - `pub fn render_hall_output(assets: &FrontEndAssets, tables: &ObjectTables, scroll_x: i32, mario: (u8, u8), sprite: u8, output: OutputMode) -> Vec<u8>`
  - `pub(super) fn sprite_frame(hall: &Hall) -> u8` — hop `9`; marching left `((walk_timer & 0x0C) >> 2) + 5`, right `+ 1`; standing left `4`, standing right `0`.
  - `pub(super) fn presentation(hall: &Hall) -> (u8, u8, u8)` — screen x `(mario - camera) as u8`, y `0x7C` minus `10` while `hop > 0`, plus `sprite_frame`.
  - `pixels(hall, assets, tables) -> Vec<u8>` calls `render_hall_output` with `OutputMode::PaletteIndices`.

- [ ] **Step 1: Write the failing picture test**

Add to `crates/sml2-core/src/front_end/render/tests.rs`:

```rust
#[test]
fn the_hall_is_open_sky_over_a_brick_floor_and_mario_is_a_sprite() {
    use crate::front_end::screen::hall_backdrop;
    use crate::video::OutputMode;
    let _ = hall_backdrop;
    let mut vram = vec![0u8; VRAM_LEN];
    // Signed `$FF` is color 0. Signed `$F8` (the brick checker) is color 3.
    paint(&mut vram, 0xF8 * 16, 3);
    for tile in 0..128 {
        paint(&mut vram, tile * 16, 1);
    }
    let mut map = vec![0u8; TILEMAP_LEN];
    map[1] = 0xF8;
    map[4 * 32 + 4] = 0xFF;
    let assets = FrontEndAssets::new(
        vec![0u8; VRAM_LEN],
        vec![0u8; TILEMAP_LEN],
        vram,
        map,
        vec![0u8; crate::front_end::DEMO_TAPES_LEN],
    )
    .unwrap();
    let frame = render_hall_output(
        &assets,
        &tables(),
        -72,
        (48, 0x7C),
        4,
        OutputMode::PaletteIndices,
    );
    assert_eq!(frame.len(), 160 * 144);
    assert_eq!(frame[0], 3, "the ceiling is the brick checker");
    assert_eq!(frame[32 * 160], 0, "the space under the ceiling is open");
    assert_eq!(frame[128 * 160], 3, "the floor is the same bricks");
    assert!(
        frame.iter().any(|token| token >> 2 == 1),
        "Mario is an object sprite"
    );
}
```

`render_hall_output` does not exist yet, so this fails to compile. Add the stub in `render.rs`:

```rust
pub fn render_hall_output(
    _assets: &FrontEndAssets,
    _tables: &ObjectTables,
    _scroll_x: i32,
    _mario: (u8, u8),
    _sprite: u8,
    _output: OutputMode,
) -> Vec<u8> {
    Vec::new()
}
```

Export it from `crates/sml2-core/src/front_end/mod.rs` next to `render_file_select`:

```rust
pub use render::{render_file_select, render_hall_output, render_title};
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sml2-core --lib front_end::render::tests::the_hall_is_open_sky_over_a_brick_floor_and_mario_is_a_sprite -- --nocapture`

Expected: FAIL. The frame is empty, so the length assertion fails.

- [ ] **Step 3: Sample the backdrop for every pixel and draw the cursor metasprite**

Add to `screen.rs` after `render_bg_wide_output`. It is the wide renderer’s off-map loop, used for every pixel, with `scroll_x` added to the view x:

```rust
#[allow(clippy::too_many_arguments)]
pub fn render_backdrop_output(
    vram: &[u8],
    tilemap: &[u8],
    bgp: u8,
    tile_data: TileData,
    view: (usize, usize),
    scroll_x: i32,
    output: OutputMode,
    backdrop: Backdrop,
) -> Vec<u8> {
    let shades = output.palette(bgp, PaletteSource::Bg);
    let tiles: Vec<[u8; 64]> = (0..=255u8)
        .map(|tile| {
            let offset = match tile_data {
                TileData::Unsigned => tile as usize * 16,
                TileData::Signed if tile < 0x80 => 0x1000 + tile as usize * 16,
                TileData::Signed => tile as usize * 16,
            };
            vram.get(offset..offset + 16)
                .and_then(|b| b.try_into().ok())
                .map_or([0; 64], tile_colors)
        })
        .collect();
    let mut out = Vec::with_capacity(view.0 * view.1);
    for y in 0..view.1 as i32 {
        for x in 0..view.0 as i32 {
            let world_x = x + scroll_x;
            let ((col, row), line) = backdrop(
                tilemap,
                world_x.div_euclid(8),
                y.div_euclid(8),
                y.rem_euclid(8) as usize,
            );
            let tile = tiles[tilemap[row * 32 + col] as usize];
            out.push(shades[tile[line * 8 + world_x.rem_euclid(8) as usize] as usize]);
        }
    }
    out
}
```

`render_bg_wide_output` already imports `PaletteSource` via `crate::video`. `render_backdrop_output` is in the same module, so it has the same imports. Add `PaletteSource` to the existing `use crate::video::...` if it is not already there (`tile_colors` and `OutputMode` are; `PaletteSource` is used by `render_bg_output` in this file, so it is in scope).

Replace the `render_hall_output` stub:

```rust
pub fn render_hall_output(
    assets: &FrontEndAssets,
    tables: &ObjectTables,
    scroll_x: i32,
    mario: (u8, u8),
    sprite: u8,
    output: OutputMode,
) -> Vec<u8> {
    use super::screen::{hall_backdrop, render_backdrop_output, TileData};
    let mut frame = render_backdrop_output(
        assets.file_select_vram(),
        assets.file_select_tilemap(),
        FILE_SELECT_BGP,
        TileData::Signed,
        (160, 144),
        scroll_x,
        output,
        hall_backdrop,
    );
    draw_metasprite(
        &mut frame,
        160,
        144,
        assets.file_select_vram(),
        output,
        tables,
        sprite,
        mario,
        false,
    );
    frame
}
```

In `hall.rs`, delete `fill` and replace `pixels`. Add the sprite helpers above `pixels`:

```rust
pub(super) fn sprite_frame(hall: &Hall) -> u8 {
    if hall.hop > 0 {
        return 9;
    }
    if hall.walk != 0 {
        let frame = (hall.walk_timer & 0x0C) >> 2;
        return if hall.facing < 0 {
            frame + 5
        } else {
            frame + 1
        };
    }
    if hall.facing < 0 { 4 } else { 0 }
}

pub(super) fn presentation(hall: &Hall) -> (u8, u8, u8) {
    let x = (hall.mario - hall.camera) as u8;
    let y = 0x7C_u8.wrapping_sub(if hall.hop > 0 { 10 } else { 0 });
    (x, y, sprite_frame(hall))
}

pub(super) fn pixels(
    hall: &Hall,
    assets: &sml2_core::front_end::FrontEndAssets,
    tables: &sml2_core::object::ObjectTables,
) -> Vec<u8> {
    let (x, y, sprite) = presentation(hall);
    sml2_core::front_end::render_hall_output(
        assets,
        tables,
        hall.camera,
        (x, y),
        sprite,
        sml2_core::video::OutputMode::PaletteIndices,
    )
}
```

Delete the test `the_picture_puts_mario_on_the_volume_pedestal`. It asserts the shade-3 rectangle. Update `the_doorway_frame_is_palette_indices` to pass tables:

```rust
fn tables() -> sml2_core::object::ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .expect("object tables")
}
```

and call `pixels(&hall, &test_assets(), &tables())`.

In `frame_image`, the hall branch calls `hall::pixels(hall, &game.session.assets.front_end)`. Add the third argument `&game.session.assets.object_tables`.

Add to the hall tests:

```rust
#[test]
fn the_walk_uses_the_file_select_frames() {
    let mut hall = Hall::enter();
    assert_eq!(sprite_frame(&hall), 4, "standing, facing left");
    let mut settings = Settings::default();
    let mut debug = false;
    step(&mut hall, true, false, false, &mut settings, &mut debug);
    assert_eq!(sprite_frame(&hall), 5, "left march frame 0");
    for _ in 0..4 {
        step(&mut hall, false, false, false, &mut settings, &mut debug);
    }
    assert_eq!(sprite_frame(&hall), 6, "walk_timer 4 advances the frame");
}
```

The decision frame does not increment `walk_timer` (`walk != 0` is drawn with timer 0, frame 5). The next four calls are move frames; after the fourth, `walk_timer` is 4 and `(4 & 0x0C) >> 2` is 1, so the sprite is 6.

- [ ] **Step 4: Run the picture tests and the hall tests**

Run: `cargo test -p sml2-core --lib front_end::render::tests::the_hall_is_open_sky_over_a_brick_floor_and_mario_is_a_sprite -- --nocapture`

Expected: PASS. Pixel `(0, 0)` is index 3, `(0, 32)` is index 0, `(0, 128)` is index 3, and at least one pixel is an object token.

Run: `cargo test -p sml2-desktop --lib app::hall -- --nocapture`

Expected: PASS, including the walk-frame test and the palette-index doorway test.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-core/src/front_end/screen.rs crates/sml2-core/src/front_end/render.rs crates/sml2-core/src/front_end/mod.rs crates/sml2-core/src/front_end/render/tests.rs crates/sml2-desktop/src/app/hall.rs crates/sml2-desktop/src/app/mod.rs
git commit -m "$(cat <<'EOF'
fix(desktop): draw the settings hall as open sky over bricks

Mario uses the file-select walk metasprite, and the floor is the same brick checker as the ceiling.
EOF
)"
```

---

### Task 4: Color that room with the DX file-select screen

**Files:**
- Modify: `crates/sml2-runtime/src/render/dx.rs` (`screen` gains two flags; add `Session::render_dx_hall`)
- Modify: `crates/sml2-desktop/src/app/mod.rs` (`frame_image`)
- Test: `crates/sml2-runtime/src/render/dx.rs`, `crates/sml2-desktop/src/app/dx_tests.rs`

**Interfaces:**
- Consumes: `hall_backdrop`, `presentation(hall) -> (u8, u8, u8)`, `dx.player(character, 0)`, `fixed_screen`, `Door` shortcut `hall.mario == DOOR && hall.walk == 0 && hall.hop == 0 && hall.goal.is_none()`.
- Produces: `pub fn render_dx_hall(&self, dx: &Presentation, character: Character, scroll_x: i32, mario: (u8, u8, u8)) -> Result<Vec<[u8; 3]>, String>` — 160×144 RGB. Background scroll is `scroll_x`. Mario’s cells are already in screen space, so they are not shifted by the scroll. Every background pixel uses `hall_backdrop`, including columns 0–1 of the file-select map.

- [ ] **Step 1: Write the failing DX color test**

In `crates/sml2-runtime/src/render/dx.rs`, `screen` stays a wrapper. The test calls the new `screen_layers` before it exists. Add the test next to `file_select_scroll_keeps_the_character_door_on_screen`:

```rust
#[test]
fn dx_hall_colors_the_open_room_and_not_the_bricks() {
    use sml2_core::front_end::screen::hall_backdrop;
    use sml2_core::hardware_palettes::rgb555;
    let mut bg = [0x1f, 0].repeat(32);
    bg[8] = 0xe0;
    bg[9] = 0x03;
    let mut attributes = vec![0u8; 0x400];
    attributes[0] = 1;
    attributes[1] = 1;
    attributes[32] = 1;
    attributes[33] = 1;
    let picture = Screen {
        vram: [vec![0; 0x1800], vec![0; 0x1800]],
        tilemap: vec![0; 0x400],
        attributes,
        bg_palettes: bg,
        obj_palettes: [0xe0, 3].repeat(32),
    };
    let frame = screen_layers(
        &picture,
        &[],
        None,
        (160, 144),
        hall_backdrop,
        -72,
        false,
        true,
    )
    .unwrap();
    assert_eq!(frame[0], rgb555(0x03e0), "ceiling bricks use palette 1");
    assert_eq!(frame[32 * 160], rgb555(0x001f), "the open room is not the brick checker");
}
```

This fails to compile because `screen_layers` is missing.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sml2-runtime --lib render::dx::tests::dx_hall_colors_the_open_room_and_not_the_bricks -- --nocapture`

Expected: FAIL to compile. `screen_layers` is not defined.

- [ ] **Step 3: Sample the hall backdrop in the DX compositor and use it from the desktop**

Replace `fn screen` with a wrapper plus `screen_layers`. The body is the current `screen`, with two changes: `cells_follow_scroll` chooses the cell x offset, and `backdrop_everywhere` forces the backdrop even inside the 160×144 map (so columns 0–1 are not the brick wall).

```rust
fn screen(
    screen: &Screen,
    cells: &[Cell],
    player: Option<&sml2_dx::PlayerGraphics>,
    view: (usize, usize),
    backdrop: Backdrop,
    scroll_x: i32,
) -> Result<Vec<[u8; 3]>, String> {
    screen_layers(screen, cells, player, view, backdrop, scroll_x, true, false)
}

#[allow(clippy::too_many_arguments)]
fn screen_layers(
    screen: &Screen,
    cells: &[Cell],
    player: Option<&sml2_dx::PlayerGraphics>,
    view: (usize, usize),
    backdrop: Backdrop,
    scroll_x: i32,
    cells_follow_scroll: bool,
    backdrop_everywhere: bool,
) -> Result<Vec<[u8; 3]>, String> {
    use super::view::{enlarged, margin, NATIVE};
    let (view, at) = if enlarged(view) {
        (view, margin(view))
    } else {
        (NATIVE, (0, 0))
    };
    let palettes = CgbPalettes::from_bytes(&screen.bg_palettes, &screen.obj_palettes)
        .map_err(|e| e.to_string())?;
    let mut vram = screen.vram.clone();
    if let Some(player) = player {
        vram[0][..0x800].copy_from_slice(&player.normal);
        vram[1][..0x800].copy_from_slice(&player.bank1);
    }
    let banks = [&vram[0][..], &vram[1][..]];
    let mut background = Vec::with_capacity(view.0 * view.1);
    for y in 0..view.1 as i32 {
        let ny = y - at.1;
        for x in 0..view.0 as i32 {
            let nx = x - at.0;
            let world_x = nx + scroll_x;
            let on_map = !backdrop_everywhere
                && (0..160).contains(&world_x)
                && (0..144).contains(&ny);
            let ((col, row), line) = if on_map {
                (
                    ((world_x / 8) as usize, (ny / 8) as usize),
                    (ny & 7) as usize,
                )
            } else {
                backdrop(
                    &screen.tilemap,
                    world_x.div_euclid(8),
                    ny.div_euclid(8),
                    ny.rem_euclid(8) as usize,
                )
            };
            let cell = row * 32 + col;
            let attr = CgbAttributes::from_byte(screen.attributes[cell]);
            background.push(BackgroundPixel {
                index: cgb::sample_tile(
                    banks,
                    0x800 + video::bg_tile_offset(screen.tilemap[cell]),
                    attr,
                    world_x.rem_euclid(8) as u8,
                    line as u8,
                )
                .map_err(|e| e.to_string())?,
                priority: attr.priority(),
            });
        }
    }
    let mut layers = super::layers::Layers::new(view, 0, background[0]);
    let cell_scroll = if cells_follow_scroll { scroll_x } else { 0 };
    push_cells(
        &mut layers.cells,
        banks,
        cells.to_vec(),
        0,
        (at.0 - cell_scroll, at.1),
        (0, 0),
    )?;
    let (_, objects) = layers.compose(1, (0, 0), |_, _| false);
    cgb::compose_frame(&background, &objects, true, &palettes).map_err(|e| e.to_string())
}
```

Add `Session::render_dx_hall` next to `render_dx_file_select`:

```rust
pub fn render_dx_hall(
    &self,
    dx: &Presentation,
    character: Character,
    scroll_x: i32,
    mario: (u8, u8, u8),
) -> Result<Vec<[u8; 3]>, String> {
    use sml2_core::actor::ScreenPosition;
    use sml2_core::front_end::screen::hall_backdrop;
    use sml2_core::player::{append_cells, CellStyle};
    let player = dx.player(character, 0);
    let mut tables = self.assets.object_tables.clone();
    tables.player_mappings = player.mappings.clone();
    let mut cells = Vec::new();
    let _ = append_cells(
        &tables,
        mario.2,
        ScreenPosition {
            x: mario.0,
            y: mario.1,
        },
        CellStyle::default(),
        &mut cells,
    );
    screen_layers(
        &dx.file_select,
        &cells,
        Some(player),
        (160, 144),
        hall_backdrop,
        scroll_x,
        false,
        true,
    )
}
```

`render_dx_file_select` already imports `player::append_cells` and `ScreenPosition` at the top of the method. Use those same paths; if `ScreenPosition` is `sml2_core::actor::ScreenPosition` in this file, match the import the existing method uses (`crate` re-exports or the full path). Do not add a second `append_cells`.

In `frame_image`, replace the hall block so the door stays on the live file select and every other pose takes the DX hall when DX is on:

```rust
if let Some(hall) = &self.hall {
    let on_door = hall.mario == hall::DOOR
        && hall.walk == 0
        && hall.hop == 0
        && hall.goal.is_none();
    if !on_door {
        let dx = self
            .dx_bundle
            .as_ref()
            .filter(|_| self.settings.dx_mode && game.session.dx_supported());
        if let Some(dx) = dx {
            let (x, y, sprite) = hall::presentation(hall);
            match game
                .session
                .render_dx_hall(dx, self.settings.character, hall.camera, (x, y, sprite))
            {
                Ok(rgb) => {
                    let frame = fixed_screen(view, [0, 0, 0], |_| rgb);
                    return Some(ColorImage::new(
                        [view.0, view.1],
                        frame
                            .into_iter()
                            .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                            .collect(),
                    ));
                }
                Err(error) => self.dx_message = error,
            }
        }
        let indices = fixed_screen(view, OutputMode::PaletteIndices.blank(3), |_| {
            hall::pixels(hall, &game.session.assets.front_end, &game.session.assets.object_tables)
        });
        let pixels: Vec<PalettePixel> = indices
            .iter()
            .map(|&token| PalettePixel::from_token(token))
            .collect();
        return Some(colorized(
            &pixels,
            self.settings.filter,
            &game.session.assets.cartridge_header,
            view,
        ));
    }
}
```

When `on_door` is true, execution falls through to the existing DX / filter render of the frozen file select.

Add to `crates/sml2-desktop/src/app/dx_tests.rs`:

```rust
#[test]
fn dx_mode_colors_the_settings_hall() {
    use sml2_core::hardware_palettes::rgb555;
    let (mut app, _, path) = super::tests::app();
    let mut dx = red_bundle();
    dx.file_select.bg_palettes[8] = 0xe0;
    dx.file_select.bg_palettes[9] = 0x03;
    dx.file_select.attributes[0] = 1;
    dx.file_select.attributes[1] = 1;
    dx.file_select.attributes[32] = 1;
    dx.file_select.attributes[33] = 1;
    app.dx_bundle = Some(dx);
    app.settings.dx_mode = true;
    app.settings.widescreen = crate::settings::Widescreen::Off;
    app.settings.camera_zoom = crate::settings::CameraZoom::Native;
    let mut hall = hall::Hall::enter();
    hall.mario = hall::pedestal_x(1);
    hall.camera = -72;
    app.hall = Some(hall);
    let image = app.frame_image(Smooth::OFF).unwrap();
    assert_eq!(
        image.pixels[0],
        Color32::from_rgb(rgb555(0x03e0)[0], rgb555(0x03e0)[1], rgb555(0x03e0)[2])
    );
    assert_eq!(
        image.pixels[32 * 160],
        Color32::from_rgb(rgb555(0x001f)[0], rgb555(0x001f)[1], rgb555(0x001f)[2])
    );
    let _ = std::fs::remove_dir_all(path);
}
```

`frame_image` and `hall` are visible to `dx_tests` because it is a child module of `app`. `Smooth` is already imported in `mod.rs` and is in the parent. `Color32` comes from `eframe::egui`, which `mod.rs` imports; `dx_tests` is a child, so it can use `Color32` if the parent import is `use eframe::egui::{..., Color32, ...}`. It is. If the child cannot see `Smooth` or `Color32`, import them in the test from the same paths `mod.rs` uses.

- [ ] **Step 4: Run the DX tests and the desktop lib**

Run: `cargo test -p sml2-runtime --lib render::dx::tests::dx_hall_colors_the_open_room_and_not_the_bricks -- --nocapture`

Expected: PASS. Ceiling pixel is green `rgb555(0x03E0)`. The pixel on row 4 is red `rgb555(0x001F)`.

Run: `cargo test -p sml2-runtime --lib render::dx::tests::file_select_scroll_keeps_the_character_door_on_screen -- --nocapture`

Expected: PASS. The wrapper still scrolls the character door.

Run: `cargo test -p sml2-desktop --lib app::dx_tests::dx_mode_colors_the_settings_hall -- --nocapture`

Expected: PASS. The desktop frame is the DX hall, not the gray filter.

Run: `cargo test -p sml2-desktop --lib -- --test-threads=8`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-runtime/src/render/dx.rs crates/sml2-desktop/src/app/mod.rs crates/sml2-desktop/src/app/dx_tests.rs
git commit -m "$(cat <<'EOF'
fix(desktop): color the settings hall with the DX file select

DX mode paints the hall from the file-select palettes and player graphics. The display filter remains the fallback.
EOF
)"
```

---

### Task 5: Spell the current option with the game's letters

**Files:**
- Modify: `crates/sml2-core/src/front_end/render.rs` (add `stamp_glyph`)
- Modify: `crates/sml2-core/src/front_end/mod.rs` (export `stamp_glyph`)
- Modify: `crates/sml2-desktop/src/app/catalog.rs` (`sign`)
- Modify: `crates/sml2-desktop/src/app/hall.rs` (stamp the sign inside `pixels`)
- Modify: `crates/sml2-desktop/src/app/mod.rs` (stamp the same sign onto the DX frame; delete the egui caption)
- Test: `crates/sml2-desktop/src/app/hall.rs`, `crates/sml2-core/src/front_end/render/tests.rs`

**Interfaces:**
- Consumes: `title_vram` (the `$41E5` image, byte 0 is VRAM `$8000`), `file_select_vram` OBJ digits at `(0x80 + digit) * 16`, `hall_backdrop`, `presentation`.
- Produces:
  - `pub fn stamp_glyph(frame: &mut [u8], stride: usize, bytes: &[u8], origin: (i32, i32), token: u8)` — every non-zero 2bpp pixel becomes `token`.
  - `pub(super) fn sign(pedestal: Pedestal, settings: &Settings, debug: bool) -> (String, String)` — name, then value. Both are `A`–`Z`, digits, and spaces.
  - `pub(super) fn sign_station(hall: &Hall) -> Option<usize>` — the pedestal he is standing on, or the one `mario + walk` will land on.

The name and value are 8×8 tiles, centered on that station, at world y `48` and `64`. A space advances 8px and draws nothing. Letters use title tiles `$50` (`A`) through `$69` (`Z`), which is the "GAME CLEARED" alphabet (`R` is `$61`). Digits use the file-select sprites `$80`–`$89`.

| Pedestal | Name | Value |
|---|---|---|
| Zoom | `ZOOM` | `AUTO`, or the digit `1`–`8` |
| Fullscreen | `FULL` | `ON` or `OFF` |
| Filter | `FILTER` | `GRAY`, `DMG`, `POCKET`, `GBC`, `AMBER`, or `SGB 1A` (digit, letter, no hyphen) |
| Volume | `VOL` | the percent with no `%` sign, `0` through `100` |
| Mute | `MUTE` | `ON` or `OFF` |
| DeadZone | `DEAD` | the percent with no `%` sign |
| Widescreen | `WIDE` | `OFF` or `169` |
| CameraZoom | `CAM` | `100`, `125`, `150`, or `200` |
| Motion | `SMOOTH` | `ON` or `OFF` |
| DxMode | `DX` | `ON` or `OFF` |
| Toolbar | `BAR` | `ON` or `OFF` |
| Debug | `DEBUG` | `ON` or `OFF` |

- [ ] **Step 1: Write the failing sign test**

Add to `hall.rs` tests:

```rust
#[test]
fn the_sign_spells_the_station_in_game_letters() {
    let settings = Settings::default();
    let debug = false;
    assert_eq!(catalog::sign(Pedestal::Zoom, &settings, debug).0, "ZOOM");
    assert_eq!(catalog::sign(Pedestal::Zoom, &settings, debug).1, "AUTO");
    assert_eq!(catalog::sign(Pedestal::Volume, &settings, debug), ("VOL".into(), "70".into()));
    assert_eq!(catalog::sign(Pedestal::Mute, &settings, debug).1, "OFF");
    let mut muted = settings.clone();
    catalog::cycle(Pedestal::Mute, &mut muted, &mut false);
    assert_eq!(catalog::sign(Pedestal::Mute, &muted, false).1, "ON");
    let mut hall = Hall::enter();
    hall.mario = pedestal_x(3);
    assert_eq!(sign_station(&hall), Some(3));
    hall.walk = -32;
    assert_eq!(sign_station(&hall), Some(4), "a left march shows the station he is walking toward");
}
```

`catalog::sign` and `sign_station` do not exist yet. Add stubs that return `("".into(), "".into())` and `None` so the test compiles and the first assertion fails.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sml2-desktop --lib app::hall::tests::the_sign_spells_the_station_in_game_letters -- --nocapture`

Expected: FAIL. The Zoom name is `""`, not `"ZOOM"`.

- [ ] **Step 3: Stamp the sign and drop the egui caption**

Add to `render.rs` and export it from `front_end/mod.rs`:

```rust
pub fn stamp_glyph(frame: &mut [u8], stride: usize, bytes: &[u8], origin: (i32, i32), token: u8) {
    if bytes.len() < 16 || stride == 0 {
        return;
    }
    let height = frame.len() / stride;
    for y in 0..8 {
        let lo = bytes[y * 2];
        let hi = bytes[y * 2 + 1];
        for x in 0..8 {
            let bit = 7 - x;
            let color = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
            if color == 0 {
                continue;
            }
            let px = origin.0 + x as i32;
            let py = origin.1 + y as i32;
            if px < 0 || py < 0 || px >= stride as i32 || py >= height as i32 {
                continue;
            }
            frame[py as usize * stride + px as usize] = token;
        }
    }
}
```

Add `sign` and the line table to `catalog.rs`. `sign_station` goes in `hall.rs`:

```rust
pub(super) fn sign_station(hall: &Hall) -> Option<usize> {
    let x = hall.mario + i32::from(hall.walk);
    (0..catalog::HALL_PEDESTALS.len()).find(|index| pedestal_x(*index) == x)
}
```

Glyph lookup, used by both the palette frame and the DX frame:

```rust
fn glyph<'a>(
    assets: &'a sml2_core::front_end::FrontEndAssets,
    ch: u8,
) -> Option<&'a [u8]> {
    let tile = match ch {
        b'A'..=b'Z' => u16::from(0x50 + (ch - b'A')),
        b'0'..=b'9' => 0x80 + u16::from(ch - b'0'),
        _ => return None,
    };
    let vram = if ch.is_ascii_digit() {
        assets.file_select_vram()
    } else {
        assets.title_vram()
    };
    vram.get(tile as usize * 16..).map(|bytes| &bytes[..16])
}

pub(super) fn stamp_line(
    frame: &mut [u8],
    stride: usize,
    assets: &sml2_core::front_end::FrontEndAssets,
    text: &str,
    center_world_x: i32,
    world_y: i32,
    camera: i32,
    token: u8,
) {
    let width = text.len() as i32 * 8;
    let mut world_x = center_world_x - width / 2;
    for ch in text.bytes() {
        if let Some(bytes) = glyph(assets, ch) {
            let origin = (world_x - camera, world_y);
            sml2_core::front_end::stamp_glyph(frame, stride, bytes, origin, token);
        }
        world_x += 8;
    }
}
```

At the end of `pixels`, when `sign_station` is `Some(index)`, stamp `sign(...)` with the name at world y `48` and the value at world y `64`, token `3`, centered on `pedestal_x(index)`.

In `frame_image`, after a successful `render_dx_hall`, stamp the same two lines into the 160×144 buffer before `fixed_screen`. The token there is not a palette index. Write the file-select background color 3 instead: `dx.file_select.bg_palettes` bytes 6 and 7 are palette 0 color 3, decoded with `sml2_core::hardware_palettes::rgb555`. Add `stamp_glyph_rgb(frame, stride, bytes, origin, rgb: [u8; 3])` next to `stamp_glyph`, same coverage, storing three bytes per pixel. Stride for the RGB buffer is `160 * 3` only if the buffer is packed RGB; it is a `Vec<[u8; 3]>`, so stamp by index `py * 160 + px` and assign the `[u8; 3]`. Do not reuse the byte-token function for that buffer.

Delete the egui caption in `game_view`: the block that calls `ui.painter().text` with `catalog::label` for the hall. The words are in the picture now.

- [ ] **Step 4: Run the sign test and the hall tests**

Run: `cargo test -p sml2-desktop --lib app::hall::tests::the_sign_spells_the_station_in_game_letters -- --nocapture`

Expected: PASS. Zoom reads `ZOOM` / `AUTO`, volume reads `VOL` / `70`, mute flips from `OFF` to `ON`, and a left march points at the next station.

Run: `cargo test -p sml2-desktop --lib app::hall -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/sml2-core/src/front_end/render.rs crates/sml2-core/src/front_end/mod.rs crates/sml2-desktop/src/app/catalog.rs crates/sml2-desktop/src/app/hall.rs crates/sml2-desktop/src/app/mod.rs
git commit -m "$(cat <<'EOF'
fix(desktop): spell each settings option in the game's letters

The station Mario is on shows its name and value with the title alphabet and the file-select digits.
EOF
)"
```
