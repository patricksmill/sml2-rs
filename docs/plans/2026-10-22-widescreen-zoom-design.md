# Widescreen and camera zoom-out — design

Status: approved 2026-10-22. Next step: implementation plan.

## Goal

Two desktop mod settings that enlarge the game view:

- **Widescreen**: Off (native 10:9) or 16:9.
- **Camera zoom**: 100%, 125% or 150% of the native height.

Together they select the view size in game pixels:

| Zoom | Widescreen off | 16:9    |
|------|----------------|---------|
| 100% | 160×144        | 256×144 |
| 125% | 200×180        | 320×180 |
| 150% | 240×216        | 384×216 |

200% and motion smoothing: see 2026-10-22-zoom200-motion-smoothing-design.md.

At 160×144 (both settings at their defaults) every output, test and sweep
result must stay byte-identical to the current build.

## Decisions

- **Spawning follows the visible area.** The object engine's camera windows
  grow with the view, so enemies do not pop in or vanish inside the frame.
  This changes gameplay while the mod is on; that is accepted.
- **Per-handler screen checks are out of scope.** Handlers that compare
  screen-relative bytes against fixed edges (e.g. the charging enemy's reset
  at screen X ≥ `$B8`) keep native behaviour. Each one found is recorded in
  the "Known quirks" section below for follow-up fixes.
- **Status bar** stays 160 px wide, centred horizontally, at the bottom of
  the view.
- **Overworld map widens**; title, file select, castle/ending/game-over
  screens and the easy-mode card stay native size, centred with bars.
- **DX renderer widens** under the same rules as the native renderer.

## 1. Settings and simulation

- `sml2-desktop` `Settings` gains `widescreen: Widescreen { Off, Wide16x9 }`
  and `camera_zoom: CameraZoom { Native, Zoom125, Zoom150 }` (serde default
  Off / Native, so old settings files load unchanged). Both are shown on the
  Mods settings page. A pure function maps the pair to `(w, h)` per the table.
- `Session` gains `view: (u16, u16)` (default 160×144), set by the host when
  settings change. It is presentation and spawn configuration, not save data.
- The object engine (`objects/mod.rs`, `engine`) widens its windows by the
  view margin, `mx = (w − 160) / 2` and `my = (h − 144) / 2`:
  - act: X ±(`$60` + mx), Y ±(`$60` + my)
  - scan: X ±(`$70` + mx)
  - alive: X ±(`$A0` + mx), Y ±(`$A0` + my)
  The margin is passed in explicitly; with `(0, 0)` the code path is
  identical to today.

## 2. Rendering

### Level view (`sml2-runtime/src/render.rs`)

- **Tile wrap:** `window_coord` only wraps outside the level when the view is
  wider or taller than 256 px; inside the level it samples the layout
  directly, so no columns repeat at 320/384 px. Views ≤ 256 px keep the
  current wrap exactly.
- **Sprite offset:** every OBJ path (player, shield, streak counter,
  bumpers, fireballs, level objects, boss OAM) adds the view margin
  `(mx, my)` to its screen position. Positions are derived from world
  coordinates minus the camera in `i32`, never through `u8` screen bytes,
  so sprites more than 255 px from the native origin do not wrap.
- **Boss OAM:** Boss cells keep the core pass's native clipping (simulation
  output) and are shifted by the view margin.
- **Status bar:** drawn at `x = (w − 160) / 2`, `y = h − 8` (bottom of the
  view) when the view is larger than native; at 160×144 it stays at the
  current `hud_y` position.

### Overworld (`sml2-core/src/map.rs`, `session.rs`)

- **Hub (screen 0):** the view is centred on the native camera rect and
  clamped to the 512×512 hub, so no wrapped content is shown.
- **Zone screens (256×144):** drawn in full and centred; any remaining area is
  filled with bars (shade 0 / blank).
- **Decor cull:** `decor_offscreen`'s X/Y ranges widen by the view margin so
  decor in the enlarged view is drawn.

### Fixed screens

Title, file-select loading, file select, castle sequence, game over, ending
and the easy-mode card render at 160×144 and are composited centred into the
view with bars.

### DX (`sml2-runtime/src/render/dx.rs`)

Every hardcoded `160`/`144` becomes the view size; the DX level, map and
fixed-screen paths follow the same rules as above.

### Desktop presentation (`sml2-desktop`)

`frame_image`, `colorized`, the texture allocation, `game_rectangle`,
`effective_scale` and the integer-zoom window sizing use the view size
instead of 160×144 (e.g. 384×216 at 5× is exactly 1920×1080). The settings
page caption "The game view stays 160 × 144" is updated.

## 3. Testing

- **Parity gate:** `cargo test -p sml2-core`, the runtime/desktop tests and
  `captures/sweep.sh` pass unchanged with the default view.
- **New unit tests:**
  - engine windows at 384×216 equal native windows widened by (112, 36);
  - Mario's cells are centred at every view size (with the status bar
    disabled, the centre 160×144 slice of a larger frame matches the native
    frame in a level away from its edges);
  - a 384-px view inside a level shows no repeated columns;
  - the status bar sits at the bottom centre of a larger view;
  - the hub view clamps to the 512×512 map edges;
  - fixed screens are centred with bars;
  - settings map to the table above and old settings files deserialize.

## Known quirks (filled in during implementation)

Handlers whose fixed screen-edge checks still act at the native edge:

- `objects/enemies/charging_enemy.rs`: resets at screen X ≥ `$B8`.
- `objects/enemies/flashing_mine.rs`: acts at screen X ≥ the course's
  `screen_limit`.
- `objects/enemies/reactions.rs`: defeat ends at screen Y ≥ `$A0`.
- `objects/bosses/shared/defeat_fall.rs`: falls while screen Y < `$D0`.
- `objects/bosses/patrol/behavior.rs`: turns at screen X ≥ 160 / < 16.
- `objects/bosses/patrol/child/behavior.rs`: despawns at screen X ≥ 184.
- `objects/bosses/sweep/child/behavior.rs`: despawns at screen X ≥ 184.
- `objects/bosses/mario_zone/pigs.rs`: walls at screen X ≥ `$A0` / < `$10`.
- `objects/bosses/mario_zone/pig2_motion.rs`: turns at screen X < 16 / ≥ 160.
- `objects/bosses/wario/form1.rs`: walls at screen X < `$98` / ≥ `$10`.
- `objects/bosses/wario/form2.rs`: turns at screen X < 32 / ≥ 144.
- `objects/bosses/wario/form3.rs`: turns at screen X < 24 / ≥ 152; exits at
  ≥ 192.
- `objects/bosses/wario/phase1_exit.rs`: exits while screen X < `$C0`.
- `objects/bosses/wario/phase2_exit.rs`: picks its animation by screen X ≥ 192.
- `objects/bosses/wario/moving_child.rs`,
  `objects/bosses/wario/trajectory_child.rs`: despawn at screen X ≥ 176.

Renderer limitations:

- The wide DX hub picks the east-half palette set and attributes once from
  the real camera, so a view straddling cam_x `$E7` colours its far half with
  one set.
- The hub stitcher assumes views ≤ 512 px.
