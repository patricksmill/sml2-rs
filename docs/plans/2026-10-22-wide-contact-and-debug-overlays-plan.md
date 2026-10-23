# Wide-view contact aliasing fix and debug overlays — plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Stop phantom contacts in enlarged views (Mario hurt by enemies 256 px away) and add debug overlays for hitboxes and the original 160×144 screen.

**Root cause (confirmed by a probe test):** the ROM's contact tests (`$2EF5` `collide`, `fire_hit`, `below_bonk`, shell `ko_overlap`) compare 8-bit screen bytes `B_SCRX`/`B_SCRY` with wrapping arithmetic. Natively the alive window (camera ±`$A0`) keeps every object within 256 px of anything it can touch, so the wrap never aliases. With a view margin, slots live at camera ±(`$A0` + margin); at a 384×216 view an enemy at Mario +261..269 px or −251..−243 px hurts him exactly like one at +5..13 px.

**Fix:** only while `view_margin != (0, 0)` (native stays byte-identical to the ROM):
- `collide`, `fire_hit`, `below_bonk`: a slot outside the *native* alive window (camera ±`$A0` on both axes, camera = `self.camera_position`, the camera its screen bytes were computed with) takes part in no contact.
- `ko_overlap`: a target slot whose world position differs from the shell's (`st.x()`, `st.y()`) by 128 px or more on either axis (wrapping `i16` difference) is skipped.

**Overlays (desktop debug panel, not persisted):**
- "Original screen border": outline of the native 160×144 rect at `view::margin(view)` in levels, demos and the overworld when the view is enlarged.
- "Hitboxes": boxes derived from `$2EF5`'s arithmetic so that a Mario box overlapping an object box ⇔ contact. Mario green, interacting objects yellow, contact-excluded (aliasing) objects grey.

## Global Constraints

- At native view (`view_margin == (0, 0)`) simulation is byte-identical: `bash captures/sweep.sh` unchanged (only the pre-existing `traj4_ff0 ... y=53 standby=2 active=2 objslots=490 odd=204` line).
- Overlays never change simulation or the rendered frame; they are painted by the desktop over the game image.
- Conventional Commits.

---

### Task 1: Core — no contacts through 8-bit screen wrap in enlarged views

**Files:**
- Modify: `crates/sml2-core/src/objects/interaction.rs` (`collide`, `fire_hit`, `below_bonk`)
- Modify: `crates/sml2-core/src/objects/shell.rs` (`ko_overlap`)
- Modify: `crates/sml2-core/src/objects/mod.rs` (helper next to `window`/`window_hit`)
- Test: `crates/sml2-core/src/objects/tests/engine.rs`

**Interfaces — Produces:** `pub fn ObjectSim::contact_excluded(&self, i: usize) -> bool` (used by Task 2's overlay).

- [ ] **Step 1: Failing tests** (append to `engine.rs`; helpers `tables()`, `blank_layout()`, `set_x`, `set_y` exist there):

```rust
/// Every horizontal offset a 384-px view keeps alive, with an active harmful
/// slot on Mario's row: which offsets hurt him.
fn hurt_offsets(margin: (u16, u16)) -> Vec<i32> {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    let mut base = ObjectSim::new(Vec::new(), cam);
    base.configure(0);
    base.view_margin = margin;
    base.active = 1;
    let mario = MarioContact { x: cam, y: 0x0100, ..Default::default() };
    let reach = 0xA0 + i32::from(margin.0);
    (-reach..=reach)
        .filter(|&dx| {
            let mut o = base.clone();
            let s = &mut o.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_RECHI] = 0xAD;
            s.b[B_TYPE] = 0x40;
            s.b[B_HB_TOP] = 0;
            s.b[B_HB_BOTTOM] = 16;
            s.b[B_HB_LEFT] = 0;
            s.b[B_HB_RIGHT] = 16;
            set_x(&mut s.b, (i32::from(cam) + dx) as u16);
            set_y(&mut s.b, 0x0100 + 20);
            for _ in 0..2 {
                o.step(cam, 0x0100, &layout, &t, &mario, false);
            }
            o.mario_dead != 0
        })
        .collect()
}

#[test]
fn an_enlarged_view_never_hurts_mario_through_the_screen_byte_wrap() {
    assert_eq!(hurt_offsets((112, 36)), (5..=13).collect::<Vec<_>>());
}

#[test]
fn a_native_view_keeps_the_rom_contact() {
    assert_eq!(hurt_offsets((0, 0)), (5..=13).collect::<Vec<_>>());
}
```

Add equivalent focused tests for `fire_hit` and `below_bonk` (an active slot whose `B_SCRX`/`B_SCRY` equal the fireball/bonk screen coordinates: with `view_margin = (112, 36)`, `camera_position = (cam, cam_y)` and the slot's world `x = cam + 0x100` the call reports no hit and sets no flag; with world `x` inside camera ±`$A0` it hits as before) and for `ko_overlap` (target slot with matching screen bytes but world `x` 256 px from the staged shell: no `B_FLAGS |= 0x02` with a margin; flagged with margin `(0, 0)`). Read each function to set the exact bytes it tests; run each new test against the unmodified code and confirm the margin cases FAIL before implementing.

- [ ] **Step 2: Run** `cargo test -p sml2-core enlarged_view` (and the other new names) — expect FAIL on the margin cases.

- [ ] **Step 3: Implement** in `objects/mod.rs`:

```rust
    /// In an enlarged view slots live past the native alive window, where
    /// their 8-bit screen bytes wrap onto contacts 256 px away; those slots
    /// take part in no contact. Native views never exclude a slot.
    pub fn contact_excluded(&self, i: usize) -> bool {
        if self.view_margin == (0, 0) {
            return false;
        }
        let c = self.camera_position;
        let s = &self.slots[i];
        !(Self::window_hit(s.x(), Self::window(c.x, 0xA0))
            && Self::window_hit(s.y(), Self::window(c.y, 0xA0)))
    }
```

Call it (`if self.contact_excluded(i) { continue; }`) right after the existing flag/type gates in `collide`, and in `fire_hit` and `below_bonk` (convert their `iter_mut()` loops to index loops `for i in 0..SLOT_COUNT` so the helper can borrow `self`; keep every other line's behaviour). In `ko_overlap`, after the type gate:

```rust
            if self.view_margin != (0, 0) {
                let near = |a: u16, b: u16| (a.wrapping_sub(b) as i16).unsigned_abs() < 0x80;
                let o = &self.slots[j];
                if !near(o.x(), st.x()) || !near(o.y(), st.y()) {
                    continue;
                }
            }
```

(Use the slot/staging position accessors that exist — `x()`/`y()` on `ObjSlot` and `Staging` in `staging.rs`.)

- [ ] **Step 4: Run** `cargo test -p sml2-core` — all pass.

- [ ] **Step 5: Gate:** `set -o pipefail; bash captures/sweep.sh 2>&1 | tail -n 25` — only the pre-existing `traj4_ff0` line.

- [ ] **Step 6: Commit** `fix(core): no contacts through the screen-byte wrap in enlarged views`

---

### Task 2: Debug overlays (hitboxes, original screen border)

**Files:**
- Modify: `crates/sml2-core/src/objects/interaction.rs` (contact box geometry next to `collide`)
- Create: `crates/sml2-runtime/src/render/overlay.rs` (register `pub mod overlay;` in `render.rs`)
- Modify: `crates/sml2-desktop/src/app/mod.rs` (fields, `game_view` painting), `crates/sml2-desktop/src/app/debug.rs` (two checkboxes)

**Interfaces:**
- Consumes: `ObjectSim::contact_excluded(i)`.
- Produces:
  - core: `pub struct ContactBox { pub x0: i32, pub y0: i32, pub x1: i32, pub y1: i32 }` (half-open, in `$2EF5`'s OAM space: screen px = x − 8, y − 16) and on `ObjectSim`: `pub fn mario_contact_box(&self, cam_x: u16, cam_y: u16) -> ContactBox`, `pub fn slot_contact_box(&self, i: usize, cam_x: u16, cam_y: u16) -> Option<ContactBox>` (`None` unless the slot is active and passes `collide`'s flag-independent type gate `$2EDD`).
  - runtime: `pub enum OverlayKind { Mario, Object, Excluded }`, `pub struct OverlayBox { pub kind: OverlayKind, pub x: i32, pub y: i32, pub w: i32, pub h: i32 }` (view pixels); `Session::overlay_boxes(&self, view) -> Vec<OverlayBox>` (levels/demos only, empty elsewhere); `Session::native_border(&self, view) -> Option<(i32, i32, i32, i32)>` (`Some((mx, my, 160, 144))` for levels, demos and the overworld when `view::enlarged(view)`).

**Geometry (derived from `collide`):** with `m = (mario.x − cam_x + 96, mario.y − cam_y + 98)` as true `i32` (no `u8` wrap) and `e` = 8 big / 0 small, ducking 0 big / −8 small:
- Mario: `x0 = m.x − 4, x1 = m.x + 6, y0 = m.y − e, y1 = m.y + 21`.
- Object at true OAM position `o = (x − cam_x + 88, y − cam_y + 88 + (mario.pan & 3))` with `HB_*` read as `i8`: `x0 = o.x + left, x1 = o.x + right, y0 = o.y + top, y1 = o.y + bottom`.
- Then `collide`'s x test `mario_sx − (o.x + left − 5) < right − left + 9` and y test `mario_sy − (o.y + top − 20) < bottom − top + 20 + e` hold ⇔ the half-open boxes overlap. Verify this algebra in the test below rather than trusting it; adjust the constants if the test disagrees and say so in the report.

- [ ] **Step 1: Failing core test** (`engine.rs`): for a grid of Mario offsets `dx in -40..40, dy in -48..48`, powerups small/big and ducking on/off, one harmful slot with hitbox `(top, bottom, left, right) = (0xF8, 8, 0xFA, 6)` at a fixed position, margin 0, `cam = (0x200, 0x100)`: run `collide` on a fresh clone and assert "slot got `B_FLAGS & 0x10`" equals "`mario_contact_box` overlaps `slot_contact_box`". (Set `B_SCRX`/`B_SCRY` exactly as `active_update` computes them from the slot's world position, or run one `step` first.)

- [ ] **Step 2:** implement the two core functions; run `cargo test -p sml2-core contact_box`.

- [ ] **Step 3: Runtime** `overlay.rs`: map boxes to view pixels (`vx = x0 − 8 + mx`, `vy = y0 − 16 + my`, `w = x1 − x0`, `h = y1 − y0`, `(mx, my) = view::margin(view)`), kind `Excluded` when `contact_excluded(i)`, `Object` otherwise; Mario box only when `ctx.spr` has a player. Unit test: in a level session after a few ticks at view `(160, 144)` the Mario box lies around Mario's drawn position (inside the frame) and at `(384, 216)` it moves by exactly the margin `(112, 36)`; `native_border((160,144))` is `None`, `native_border((384,216))` is `Some((112, 36, 160, 144))` in a level.

- [ ] **Step 4: Desktop:** `DesktopApp` fields `show_hitboxes: bool`, `show_native_border: bool` (default false). In `debug.rs` `debug_controls`, add an "Overlays" collapsing section with two `ui.checkbox`es. In `game_view`, after `ui.painter().image(...)`, when a game exists and a toggle is on, paint 1-px strokes inside `rect` (egui 0.33 `painter.rect_stroke(rect, 0.0, Stroke::new(1.0, color), StrokeKind::Inside)`), mapping view pixel `(x, y, w, h)` to `rect.min + vec2(x, y) * (rect.width() / view.0)`; clip to `rect` (`ui.painter().with_clip_rect(rect)`). Colours: Mario `Color32::GREEN`, Object `Color32::YELLOW`, Excluded `Color32::GRAY`, border `Color32::from_rgb(255, 0, 255)`. Extract the view→screen mapping as a small pure function and unit-test it (scale 3, origin offset).

- [ ] **Step 5: Run** `cargo test -p sml2-core -p sml2-runtime -p sml2-desktop`.

- [ ] **Step 6: Commit** `feat: debug overlays for hitboxes and the original screen border`
