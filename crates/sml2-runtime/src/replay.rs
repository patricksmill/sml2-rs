//! Frozen-corpus replay adapter and byte verification.

use crate::content::{load_map_nodes, load_scene_scripts};
use crate::divider_clock::VirtualDividerClock;
use crate::game::{
    apply_level_entry, build_contact, note_handoff_screen, sample_level_player_inputs,
    scene_frame_head, seed_level_timer,
};
use crate::image_output::write_png_scaled;
use crate::render::{apply_tile_anim, hud_row_status, render_view, HudDigits, SpriteCtx};
use crate::{SCREEN_H, SCREEN_W};
use serde::Deserialize;
use sml2_core::ending_entry::{EndingEntry, EndingFrame, EndingInput};
use sml2_core::level::{HudStrip, LevelLayout, LevelManifest};
use sml2_core::objects::FireHit;
use sml2_core::player_sim::{PlayerKeys, PlayerSim};
use sml2_core::scene::{boss_def_index, boss_def_portable, SceneVm};
use std::fs;
use std::path::Path;

/// One captured live frame (`trajectory_capture*.py`) for sim replays.
#[derive(Deserialize)]
struct Frame {
    x: u16,
    y: u16,
    hvel: u8,
    vvel: u8,
    ground: bool,
    dir_right: bool,
    camx: u8,
    #[serde(default)]
    camxh: u8,
    #[serde(default)]
    camy: u8,
    #[serde(default)]
    camyh: u8,
    #[serde(default)]
    frame: u8,
    #[serde(default)]
    edge: u8,
    #[serde(default)]
    limit: bool,
    #[serde(default)]
    autoscroll: u8,
    #[serde(default)]
    en_r: bool,
    #[serde(default)]
    en_l: bool,
    #[serde(default)]
    slots: Vec<SlotJ>,
    #[serde(default)]
    sptr: u16,
    #[serde(default)]
    spx: u16,
    #[serde(default)]
    sdir: u8,
    #[serde(default)]
    standby: u8,
    #[serde(default)]
    active: u8,
    #[serde(default)]
    easy: u8,
    #[serde(default)]
    odd: u8,
    phase: u8,
    /// `$FFF0` (the full byte; `& 3` gates sprite toggles / state machines).
    #[serde(default)]
    ff0: Option<u8>,
    /// `sObjectUpdatePhase`.
    #[serde(default)]
    uphase: Option<u8>,
    /// `sPipeTravelDirection` ($A221) — arming + travel state.
    #[serde(default)]
    pipe: Option<u8>,
    /// `sScreenTransition` ($A20E) — the `$0DED` scroll state.
    #[serde(default)]
    tr: Option<u8>,
    /// The engine's previous-camera latch ($AF24/$AF25) — the scan-direction
    /// source. Captures sampled at the level's very first frame need it
    /// (the load leaves it 0, so frame 1 scans forward).
    #[serde(default)]
    pcx: Option<u16>,
    /// `sMoveFlags`.
    #[serde(default)]
    mf: Option<u8>,
    /// `sObjectFacing`.
    #[serde(default)]
    fac: Option<u8>,
    /// `sMarioDead`.
    #[serde(default)]
    dead: Option<u8>,
    /// `sStompBounceTimer` / `sStompChainCount` / `sMoneyBagCount`.
    #[serde(default)]
    sbt: Option<u8>,
    #[serde(default)]
    scc: Option<u8>,
    #[serde(default)]
    mb: Option<u8>,
    /// `sCameraPanTimer`.
    #[serde(default)]
    pan: Option<u8>,
    /// `sLives` ($A22C, BCD) — present in full-death-cycle captures.
    #[serde(default)]
    lives: Option<u8>,
    /// `sCurPowerup` / `sInvincibilityTimer` / `sGoalBell` / `sSecretRouteFlag`
    /// — the `$3451` item-consumer observables.
    #[serde(default)]
    pow: Option<u8>,
    #[serde(default)]
    inv: Option<u8>,
    #[serde(default)]
    goal: Option<u8>,
    #[serde(default)]
    secret: Option<u8>,
    /// `sMoonPhysics` ($A287) — the level's `LevelPropertiesSpace` byte. The sim
    /// derives it from the manifest (`PlayerSim::seed_moon_physics`), so this is
    /// a CHECK of the derivation, not a source for it (traj19/level 20 → $01).
    #[serde(default)]
    moon: Option<u8>,
    /// `sSpinJumpBounce` ($A220) — the ceiling-bonk flag (`$FF` after a bump).
    #[serde(default)]
    spin: Option<u8>,
    /// `sCoinLow`/`sCoinHigh` ($A262/$A263, BCD).
    #[serde(default)]
    coins: Option<u8>,
    #[serde(default)]
    coinshi: Option<u8>,
    /// The 8 `sBlockAnimSlots` ($AA80) as 16-byte hex dumps — the brick-bump
    /// system's whole state (type, timer, Y, X per slot).
    #[serde(default)]
    bslots: Vec<String>,
    /// The `$1CBA` exit-tile arm's observables (`captures/traj17.json`):
    /// `sExitTileFlag` ($A2B4), `sLowTimeFlag` ($A53F), the exit fade's
    /// `sFadeTimerLow`/`High` ($A266/$A267) and `sLevelTally` ($A2C5, BCD).
    /// All optional, so every capture recorded before traj17 still replays.
    #[serde(default)]
    exit: Option<u8>,
    #[serde(default)]
    lowtime: Option<u8>,
    #[serde(default)]
    fadelo: Option<u8>,
    #[serde(default)]
    fadehi: Option<u8>,
    #[serde(default)]
    tally: Option<u8>,
    /// The fade/map/reload gap carried by the first post-respawn record of a
    /// full-death-cycle capture.
    #[serde(default)]
    gap: Option<GapJ>,
    /// The exit-tile arm's writes, carried on the arm's own record
    /// (`captures/traj17.json`) — checked once, outside the per-frame table.
    #[serde(default)]
    exit_arm: Option<ExitArmJ>,
    /// 16 raw 32-byte slot dumps (64 hex chars each) — the full-byte check.
    #[serde(default)]
    slot_bytes: Vec<String>,
    #[allow(dead_code)]
    timer: u8,
    right: bool,
    left: bool,
    up: bool,
    down: bool,
    a: bool,
    b: bool,
}

#[derive(Deserialize, Default)]
struct SlotJ {
    #[serde(default)]
    state: u8,
    #[serde(default)]
    x: u16,
    #[serde(default)]
    y: u16,
    #[serde(rename = "def_", default)]
    def_: u8,
    #[serde(default)]
    sprite: u8,
    #[serde(default)]
    rec: u16,
}

/// The unmodelled stretch of a full-death-cycle capture: the state machine
/// between the fade trigger and the level re-entry (the `$14` lives/save/map
/// setup → the input-dependent overworld map → the reload), as
/// `[[state, ticks], ...]` polled from `$FF9B`, plus the total tick gap.
#[derive(Deserialize)]
struct GapJ {
    total: u64,
    seq: Vec<[u64; 2]>,
}

/// The `$1CBA` exit-tile arm's facts that a per-frame table cannot express,
/// carried on the arm's own record by `captures/trajectory_capture17.py`: the
/// frame it fired on, the tile the probe read, and the fade span measured from
/// outside the in-level hook (`$05E4` stops firing once `$FF9B` leaves 4, so
/// the capture splices the frames after it in from a `$028D` hook).
#[derive(Deserialize)]
struct ExitArmJ {
    /// The in-level record the arm fired on — the LAST `$05E4` record; every
    /// record after it came from the `$028D` splice.
    frame: usize,
    #[allow(dead_code)]
    tick: u64,
    x: u16,
    y: u16,
    /// `sCollisionTile` at the arm (the `$4A` the probe read).
    col: u8,
    music: u8,
    #[allow(dead_code)]
    exit: u8,
    #[allow(dead_code)]
    lowtime: u8,
    tally_before: u8,
    tally_after: u8,
    #[allow(dead_code)]
    fade_first: u16,
    #[allow(dead_code)]
    fade_last: u16,
    /// `[[$FF9B, ticks], ...]` polled per tick from the arm onwards.
    state_seq: Vec<[u64; 2]>,
}

/// Replay a captured trajectory through `sml2_core::player_sim` and report
/// frame-by-frame divergences from the live console state. With `filmstrip`,
/// also renders every 30th frame (sim + camera + player draw) as a 2x PNG.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_replay(
    path: &Path,
    level_entry: sml2_core::level::LevelEntry,
    manifest: &LevelManifest,
    layout: &LevelLayout,
    bg_base: &[u8],
    anim_rows: &[u8],
    hud: &HudStrip,
    hud_y: u8,
    mut spr: Option<SpriteCtx>,
    filmstrip: Option<&Path>,
    edge_map: Option<Vec<u8>>,
    scene_map: Option<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let frames: Vec<Frame> = serde_json::from_str(&fs::read_to_string(path)?)?;
    if frames.len() < 2 {
        return Err("replay: need at least 2 frames".into());
    }
    // The level's 48-byte sScreenEdgeMap: with it the port derives `$FF98`
    // itself and the capture's byte becomes a check. Without it the captured
    // byte is injected as before (and the run says so, so the gate cannot
    // quietly pass with the port's own edge map never exercised).
    let own_edges = matches!(edge_map.as_ref(), Some(e) if e.len() == 48);
    if !own_edges {
        println!("note: level edge table missing — replaying with the captured $FF98");
    }
    // Same rule for `sSceneMap`: nothing in a capture records scene data, so a
    // missing file means `scene_id` stays 0 and no transition can ever start a
    // scene. The extractor writes one file per level, so this never fires in
    // the gate — it is the flag for the case where it should have.
    if !matches!(scene_map.as_ref(), Some(m) if m.len() == 48) {
        println!("note: level scene map missing — sSceneId will stay 0");
    }
    // The scene scripts (bank 5 `$4125`): one ROM-wide blob, so it is loaded
    // here rather than threaded through `main`. Level 1's scene map is all
    // zeros, so its `sSceneId` never latches and the VM never runs a byte.
    let scripts = load_scene_scripts();
    let mut scene_vm = SceneVm::new();
    let mut ending_entry = EndingEntry::default();
    let mut ending_sequence = sml2_core::ending_sequence::EndingSequenceController::new();
    let mut ending_scene = sml2_core::ending_scene::EndingScene::default();
    let nodes = load_map_nodes();
    let mut sim = PlayerSim::new();
    sim.powerup = 0; // level-0 spawn starts small
    sim.secret_route = level_entry.secret_route;
    sim.level_progress_flags = level_entry.progress_flags;
    sim.level_progress_flags = level_entry.progress_flags;
    seed_level_timer(&mut sim, manifest);
    let f0 = &frames[0];
    sim.x = f0.x;
    sim.y = f0.y;
    sim.hvel = f0.hvel;
    sim.vvel = f0.vvel;
    sim.on_ground = f0.ground;
    sim.in_air = !f0.ground;
    sim.dir_right = f0.dir_right;
    sim.walk_timer = f0.timer;
    sim.lives = f0.lives.unwrap_or(5);
    // `$04DD`/`home.asm:661`: `sMoonPhysics` is the level's own property byte,
    // so the sim DERIVES it from the manifest — never from the record (the
    // edge-map rule: an injected value leaves the derivation untested in play).
    // Captures that carry `moon` make it a CHECK instead: traj19 (level 20)
    // records `$01` on all 1309 frames, which is `LevelPropertiesSpace[19]`.
    sim.seed_moon_physics(manifest.properties.space_physics);
    let mut cam = sml2_core::camera::CameraSim::new(
        f0.camx as u16 | ((f0.camxh as u16 & 0x0F) << 8),
        f0.camy as u16 | ((f0.camyh as u16 & 0x0F) << 8),
    );
    cam.pan_timer = f0.pan.unwrap_or(0); // documented initial snapshot only
    cam.update_scroll();
    cam.prev_mario_y_lo = f0.y as u8;
    // Frozen tapes begin from a live snapshot, rather than the level loader.
    // Seed its projection once; subsequent Mario draws own this latch.
    cam.mario_screen_x = (f0.x as u8).wrapping_sub(cam.x as u8).wrapping_add(96);
    cam.map_scroll_limit = f0.limit;
    cam.auto_scroll = f0.autoscroll;
    if let Some(e) = edge_map.as_ref() {
        cam.edge_map.copy_from_slice(e);
    }
    // `$0609`: the header's +18 high nibble is the initial `sSceneState`, and
    // the 48-byte scene map is what `$0F0B` latches `sSceneId` from. Level 1's
    // map is all zeros, so this cannot move the gate — it is wired for the
    // scene levels, where a transition under a nonzero cell starts a scene.
    cam.scene_state = manifest.header.scene_state;
    if let Some(m) = scene_map.as_ref() {
        if m.len() == 48 {
            cam.scene_map.clone_from(m);
        }
    }
    let mut obj = sml2_core::objects::ObjectSim::new(manifest.spawns.records.clone(), cam.x);
    obj.easy_mode = f0.easy != 0;
    obj.configure(manifest.level_id);
    obj.fff0 = f0.ff0.unwrap_or(u8::from(f0.odd != 0));
    obj.update_phase = f0.uphase.unwrap_or(0);
    // The toggle flips `phase`, so the recorded phase seeds the bool directly
    // (a capture sampled before the level's first frame has uphase 0).
    if let Some(u) = f0.uphase {
        obj.phase = u == 1;
    }
    if let Some(p) = f0.pcx {
        obj.prev_cam_x = p;
    }
    obj.odd = obj.fff0 & 1 == 1;
    // The object handlers need the anim→sprite/hitbox tables; clone them out
    // of the sprite context so the loop can still borrow it mutably.
    let tables = spr
        .as_ref()
        .map(|s| s.tables.clone())
        .ok_or("replay requires objects.json (no sprite context)")?;
    // $0DAF's capture-less flag is RETIRED (2026-10-14): traj19 replays level 20
    // (`--level 19`, listed in `captures/sweep.sh`) with `camx`/`camy`/`en_r`/
    // `en_l`/`edge` all-zero across 1308 compared frames, so the autoscroll path
    // is gate-verified now. The flag's job was to fail a gate that added such a
    // capture; traj19 is that capture, so it is the check that replaces it.
    if let Some(dir) = filmstrip {
        fs::create_dir_all(dir)?;
    }
    let mut counts: Vec<usize> = Vec::new();
    let mut names: Vec<&'static str> = Vec::new();
    let mut shown = 0usize;
    let mut gap_ok = true;
    let mut prev_a = f0.a;
    let mut prev_b = f0.b;
    // The first snapshot has explicit virtual DIV phase zero; the corpus has
    // no initial counter or within-frame sample timing evidence.
    let mut divider_clock = VirtualDividerClock::new(0);
    for (i, f) in frames.iter().enumerate().skip(1) {
        divider_clock.advance_tick();
        let keys = PlayerKeys {
            right: f.right,
            left: f.left,
            up: f.up,
            down: f.down,
            a: f.a,
            b: f.b,
            // The replay `Frame` carries no START/SELECT: no capture records
            // them, and the pause state is only reachable interactively.
            start: false,
            select: false,
        };
        let a_pressed = f.a && !prev_a;
        prev_a = f.a;
        // `hKeysPressed` bit 1 — the fire button is B, edge-triggered, so
        // holding it fires once.
        let b_pressed = f.b && !prev_b;
        prev_b = f.b;
        // $FF97 & 7 — the speed-dither phase. The engine increments it once
        // per frame ($0300/$2729 wait for the VBlank flag, then `inc`), so
        // the $181C integration always reads "previous residue + 1". The
        // capture samples $FF97 at the PyBoy frame boundary, which slips an
        // extra increment on traj10 f2986/2988/2989 — the recorded x-deltas
        // there only match the cadence value (f2988's console step is +1,
        // where $FF97 = 5 would force +2). Keep the record when it agrees
        // with the cadence; the death captures' per-logic-frame sampling
        // breaks the cadence elsewhere, so the record still wins there.
        let expect = (sim.phase + 1) & 7;
        sim.phase = if f.phase == (expect + 1) & 7 {
            expect
        } else {
            f.phase
        };
        // `$FF97`'s bits above 2 are not in the capture record, so the full
        // counter is a best-effort free run here (only `$532D`'s spin-jump pose
        // reads bit 3, and no capture spins). The *low* bits still come from the
        // record above, which is what every other reader uses.
        sim.frame_counter = sim.frame_counter.wrapping_add(1);
        // The camera bytes the *player* update reads are the ones written by
        // the previous frame's camera step: the ROM runs `$0F71` before
        // `$0B2A` (home.asm:799/803), so `$0FBF`'s clamp and `$1BFF`'s hazard
        // `sMarioScreenY` both see the camera as of the frame's START. Feed
        // them from the port's own camera object — exactly what `run_play`
        // does — rather than from `f.camx`/`f.camy`, which are the record's
        // POST-frame camera. Injecting the post-frame byte put the clamp one
        // frame ahead of the console and made the sim push Mario +1 early on
        // every frame the autoscroll camera stepped (traj19,
        // `autoscroll_clamp_x_divergence`). `cam.x`/`cam.y` here already equal
        // the record's camera for the PREVIOUS frame — the camx/camy checks
        // below still verify the derivation against the record.
        sim.camx = cam.x;
        sim.camy = cam.y;
        sim.pan_timer = cam.pan_timer;
        // $05E4/$0640: the frame loop's three branches. Dead (`$FF`) runs only
        // the arc + the object engine (the camera freezes); the death fade
        // (`$FF9B = 9`) leaves the level entirely, freezing everything.
        let ending_frame = scene_frame_head(
            &mut scene_vm,
            &mut ending_entry,
            &mut sim,
            &mut cam,
            EndingInput {
                held: keys,
                pressed: PlayerKeys {
                    a: a_pressed,
                    b: b_pressed,
                    ..PlayerKeys::default()
                },
            },
        );
        let gameplay = matches!(ending_frame, Some(EndingFrame::Gameplay(_)));
        let input = match ending_frame {
            Some(EndingFrame::Gameplay(input)) => input,
            _ => EndingInput::default(),
        };
        let keys = input.held;
        let (a_pressed, b_pressed) = (input.pressed.a, input.pressed.b);
        let was_dead = sim.dead == 0xFF;
        // The fade-trigger frame is the last one to run the object engine
        // (the dead branch continues into `$0652`); the true fade frames do
        // not. Capture that before the step flips it.
        //
        // `$FF9B` is what the frame loop dispatches on: 9 = the death fade
        // (state `$09`), 24 = the exit fade the `$1CBA` bell arm starts
        // (state `$18`). Both leave the in-level loop, so the player, camera
        // and objects all freeze while either is set. VBlank's `sFadeTimer`
        // decrement still runs, though: stepping the exit fade is `$3547`'s
        // poll, and its last call credits the BCD tally and marks the `$14`
        // hand-off. (The death fade's decrement stays with the gap handler
        // below, whose duration check owns that cycle.)
        //
        // $FF98 is read from the level's edge table at the camera's *current*
        // cell, before the camera moves ($0B39) — and the console's sampled
        // byte still holds that pre-move value on the frame a crossing
        // happens, so it is read here and checked after the step. Recomputing
        // it on a dead/fade frame is right: the camera has not moved, so the
        // cell (and the console's stale $FF98) is unchanged.
        let cam_edge_before = if own_edges { cam.edge_byte() } else { f.edge };
        let fade_at_entry = !gameplay || sim.dead_fade || sim.exit_phase;
        // `$FF9B = $14` is set by the exit fade's *last* frame, so the hand-off
        // body runs on the frame after it — keyed on the state at entry.
        let exit_done_at_entry = sim.exit_completed;
        if !gameplay {
            // The shared frame head dispatched this ending-entry frame.
            if let Some(EndingFrame::SequencePending) = ending_frame {
                let res =
                    ending_sequence.step(&mut sml2_core::ending_sequence::EndingStepContext {
                        scene: &mut ending_scene,
                        rom: None,
                        credits_text: None,
                        object_tables: Some(&tables),
                        decrement_fade: true,
                    });
                if res.sfx_request_0 != 0 {
                    sim.sfx0 = res.sfx_request_0;
                }
                if res.sfx_request_1 != 0 {
                    sim.sfx1 = res.sfx_request_1;
                }
            }
        } else if sim.dead_fade || sim.exit_phase {
            if sim.exit_phase {
                sim.exit_fade_step();
            }
            if exit_done_at_entry {
                let handoff = sim.map_handoff(&nodes);
                note_handoff_screen(&handoff);
            }
        } else if was_dead {
            sim.death_arc(cam.y);
        } else {
            // a screen transition skips the player update — the camera's
            // `$0DED` handler moves Mario instead.
            if cam.transition == 0 {
                // $307D — `$0F71`'s FIRST call: the B+Down/B+Up crouch-look
                // pan runs at the *top* of the player update, before the body,
                // so its `sMarioOnGround`/`sMarioOnObject` gate reads the
                // flags the previous frame settled (`PlayerSim::step` would
                // overwrite them). Still before `cam.step` below, and skipped
                // whenever `$0F71` is — see `CameraSim::crouch_pan`.
                cam.crouch_pan(&sim, &keys);
                sim.step(&keys, a_pressed, b_pressed, layout);
            } else {
                // home.asm ~115: VBlank decrements sFadeTimer unconditionally
                // on every in-level frame, including during camera transitions.
                sim.tick_fade_timer();
            }
            // `$3451` id 4's conditional `sStompChainCount` reset (star
            // pickup on an expired timer). The counter lives in the object
            // engine, so the consumer's one-shot flag is applied here —
            // before this frame's object pass can bump it again.
            if sim.take_stomp_chain_reset() {
                obj.stomp_chain = 0;
            }
            // $2F72 — the block-bump alloc's tail. It runs inside the player
            // update ($0F71), before $0B2A/$2062 recompute the scroll, so its
            // screen math reads the *previous* frame's sScroll — the camera
            // scroll_y/scroll_x latched by $2062 at the end of the previous frame.
            // The object engine ($8000) reads the below-hit flag later this frame,
            // so the pass has to land before `obj.step`; `resolve_bump_coin` is the
            // `$19D2` coin half of the split.
            if let Some((by, bx)) = sim.below_bonk.take() {
                let scroll_y = cam.scroll_y;
                let scroll_x = cam.scroll_x;
                let special = obj.below_bonk(by, bx, scroll_y, scroll_x);
                sim.resolve_bump_coin(special);
            }
            // A fireball's `$06` break reaches the same `$5960`/`$2F72` pair
            // from `$0F71`'s tail: the ball's own cell bytes arm the pass, and
            // only the player's `$19D2` bump owes the coin.
            if let Some((by, bx)) = sim.fire_below_bonk.take() {
                let scroll_y = cam.scroll_y;
                let scroll_x = cam.scroll_x;
                obj.below_bonk(by, bx, scroll_y, scroll_x);
            }
            // `$3261`'s `$2FED`: the ball's staged screen pair against the
            // object slots, before the object engine reads the flags it sets.
            for i in 0..sim.fire_scan.len() {
                let Some((sy, sx)) = sim.fire_scan[i].take() else {
                    continue;
                };
                let hit = obj.fire_hit(sy, sx);
                if hit == FireHit::Die {
                    sim.sfx0 = 2; // `$3007`'s sfx arm
                }
                sim.resolve_fire_hit(i, hit != FireHit::None);
            }
            cam.edge_now = if own_edges { None } else { Some(f.edge) };
            cam.step(&mut sim);
            cam.update_enables(sim.x);
        }
        // $FF98 for this frame. `own_edges` makes it the port's own derivation
        // from the level's 48-byte table (checked below); otherwise the
        // captured byte is what the camera ran with, so the check is a
        // tautology — the missing-table note above is what fails the gate.
        let edge_derived = if own_edges { cam_edge_before } else { f.edge };
        cam.update_player_pan(sim.y, was_dead, !fade_at_entry);
        let mario = build_contact(&sim, &keys, cam.scroll_pan);
        // $2D0A: the frame selection runs *before* the interaction pass, so
        // on the contact frame the sprite still shows the grounded pose.
        let frame_idx = if was_dead {
            107
        } else {
            sim.frame_index(sim.walk_timer)
        };
        // $2C82 (`$5997`): the brick-bump update sits between the frame
        // selection (`$2D0A`) and the object engine, so when a bump draws, its
        // frame is what `$FFC6` — and the capture's `frame` check — ends the
        // frame with. The dead branch skips it (no `$2C82` there), and so does
        // the level-leaving freeze.
        let frame_check = if was_dead || fade_at_entry {
            frame_idx
        } else {
            sim.block_update();
            sim.block_frame.unwrap_or(frame_idx)
        };
        if !fade_at_entry {
            // $8000's dispatcher gate reads sMarioDead; the interaction pass
            // then writes it back.
            sample_level_player_inputs(&mut obj, &sim);
            obj.div = divider_clock.sample();
            // `$0EB0`'s scene branch: taken only for a def whose handlers are
            // all ported (the fight then ends in the level clear, which reseeds
            // `sSceneState` and so clears the park); every other def is
            // reported, because parking with nothing to end the fight would
            // freeze the camera for good.
            if cam.scene_load_pending {
                let def_index = boss_def_index(manifest.level_id, cam.boss_fight_phase);
                if boss_def_portable(def_index) {
                    if let Some((music, level_song)) = cam.begin_scene_load(manifest.level_id) {
                        // `$0ECA`: the boss song goes to BOTH bytes.
                        sim.music_request = music;
                        sim.level_song = level_song;
                        obj.spawn_boss(def_index);
                    }
                } else if cam.take_blocked_scene_load() {
                    obj.note_blocked_scene_load();
                }
            }
            obj.step(
                cam.x,
                cam.y,
                layout,
                &tables,
                &mario,
                cam.boss_mode(), // `$0652`
            );
            // The write-back list lives in the sim so this loop and the play
            // loop cannot drift apart again (`infra_play_parity_harness`).
            sim.apply_object_result(&mut obj, keys.a);
            cam.apply_object_result(&mut obj);
        }
        // $20A4 owns the CPU across its HALT waits; the shared frame head
        // parks gameplay until its END returns to the main loop.
        if gameplay || ending_frame.is_none() {
            let scene_camera = (cam.x, cam.y);
            let stepped = scene_vm.step(&mut sim, &mut cam, &scripts);
            if let Some(s) = spr.as_mut() {
                crate::render::PipeBackground::update(
                    &mut s.pipe_background,
                    scene_camera,
                    &cam,
                    stepped,
                );
            }
        }
        // The full-death-cycle gap: after the fade-trigger frame the game
        // leaves the level (fade 192 frames → `$14` lives/save/map setup →
        // the overworld map, which waits for input → re-entry reloads the
        // level). Only the fade and the reload's final state are modelled;
        // the input-dependent map is skipped over the captured tick gap.
        //
        // The map itself IS modelled now — `sml2_core::map_sim` is state `$0C`
        // (its tick, the cursor's walk graph, the gate and the entry), driven
        // by its own unit tests — but this harness cannot drive it: the replay
        // is an in-level ($05E4) record, and a map frame has no in-level state
        // to compare against. So a state-`$0C` stretch in `g.seq` stays inside
        // `g.total` here and is *accounted for*, not simulated.
        if let Some(g) = &f.gap {
            // The trigger record already counted one tick; the gap includes
            // its entire fade/map duration, not just the unmodelled map tail.
            divider_clock.advance_ticks(g.total.saturating_sub(1));
            let fade_capture = g.seq.first().copied();
            let modeled = 192u64;
            let acct: u64 = 1 + g.seq.iter().map(|s| s[1]).sum::<u64>();
            let fade_ok = matches!(fade_capture, Some([9, n]) if n == modeled)
                && sim.dead_fade
                && sim.fade_timer == modeled as u16;
            // Run the modelled fade (its last step decrements the lives).
            while sim.fade_timer > 0 {
                sim.fade_step();
            }
            if sim.game_over {
                // $314B → $05: the borrow means the game overruns to the GAME
                // OVER screen instead of reloading the level. No capture
                // reaches zero lives, so this is the modelled-only branch:
                // $27AF's state writes, then $07's fade + START poll (which
                // the caller drives via `game_over_step`).
                sim.game_over_enter();
                // `$27AF` clears `sMoneyBagCount` along with the coins, and that
                // byte lives on the object engine, so the caller clears it here.
                obj.money_bag = 0;
            } else {
                // $0F2A + $04F1 + $EBBB: the reload puts Mario back at the
                // header spawn and clears the object scratch (fff0 persists).
                sim.respawn(level_entry.start.mario_x, level_entry.start.mario_y);
                seed_level_timer(&mut sim, manifest);
                apply_level_entry(&mut sim, &mut cam, level_entry);
                obj.reset_level_start();
            }
            let skipped = g.total.saturating_sub(1 + modeled);
            let ok = fade_ok && acct == g.total;
            println!(
                "  frame {i}: death cycle: fade {modeled} modeled vs {fade_capture:?} captured; +{skipped} unmodeled ticks skipped (seq {:?}, total {} vs {} accounted){}",
                g.seq,
                g.total,
                acct,
                if ok { "" } else { "  <- MISMATCH" }
            );
            gap_ok = gap_ok && ok;
        }
        let streak = if !fade_at_entry && !was_dead {
            obj.tick_stomp_bounce_timer()
        } else {
            None
        };
        if let Some(dir) = filmstrip {
            if i % 30 == 0 {
                if let Some(s) = spr.as_mut() {
                    s.player = Some((sim.x, sim.y, frame_idx));
                    s.streak_counter = streak;
                    s.player_pan = cam.player_pan;
                    s.scroll_pan = cam.scroll_pan;
                    // The dead path's own draw overwrites the staging pair.
                    s.bumpers = if was_dead {
                        Vec::new()
                    } else {
                        sim.block_draw.clone()
                    };
                    s.fireballs = sim.fire_draw.clone();
                }
                let mut rbg = bg_base.to_vec();
                apply_tile_anim(&mut rbg, &manifest.tile_anim, anim_rows, 0);
                let hud_live = hud_row_status(
                    hud,
                    HudDigits {
                        lives: sim.lives,
                        coins_high: sim.coins_high,
                        coins_low: sim.coins_low,
                        money_bag: obj.money_bag,
                        timer_high: sim.level_timer.high,
                        timer_low: sim.level_timer.low,
                    },
                );
                let frame = render_view(
                    manifest,
                    layout,
                    &sim.layout_writes,
                    &rbg,
                    cam.x,
                    cam.y,
                    SCREEN_W,
                    SCREEN_H,
                    Some((&hud_live, hud_y)),
                    spr.as_ref(),
                    None,
                )?;
                let out = dir.join(format!("frame_{i:04}.png"));
                write_png_scaled(&out, SCREEN_W, SCREEN_H, 2, &frame)?;
            }
        }
        let mut checks: Vec<(&'static str, bool, String)> = Vec::new();
        {
            let (slot_ok, slot_msg) = {
                let mut bad = false;
                let mut msg = String::new();
                for (k, sl) in f.slots.iter().enumerate() {
                    let s = &obj.slots[k];
                    let st = match s.state() {
                        sml2_core::objects::SlotState::Inactive => 0u8,
                        sml2_core::objects::SlotState::Standby => 1,
                        sml2_core::objects::SlotState::Active => 2,
                    };
                    let ok = if st != sl.state {
                        false
                    } else if st == 0 {
                        true
                    } else {
                        s.def() == sl.def_
                            && s.sprite() == sl.sprite
                            && s.rec_index() == sl.rec
                            && s.x() == sl.x
                            && s.y() == sl.y
                    };
                    if !ok {
                        bad = true;
                        if msg.is_empty() {
                            msg = format!(
                                "slot {k}: state sim={st} exp={} def sim={} exp={} spr sim={} exp={} rec sim=${:04X} exp=${:04X} pos sim=({},{})",
                                sl.state, s.def(), sl.def_, s.sprite(), sl.sprite, s.rec_index(), sl.rec,
                                s.x(), s.y()
                            );
                        }
                    }
                }
                (!bad, msg)
            };
            let mut push = |name: &'static str, ok: bool, msg: String| {
                checks.push((name, ok, msg));
            };
            push("x", sim.x == f.x, format!("x sim={} exp={}", sim.x, f.x));
            push("y", sim.y == f.y, format!("y sim={} exp={}", sim.y, f.y));
            push(
                "hvel",
                sim.hvel == f.hvel,
                format!("hvel sim=${:02X} exp=${:02X}", sim.hvel, f.hvel),
            );
            push(
                "vvel",
                sim.vvel == f.vvel,
                format!("vvel sim=${:02X} exp=${:02X}", sim.vvel, f.vvel),
            );
            push(
                "ground",
                sim.on_ground == f.ground,
                format!("ground sim={} exp={}", sim.on_ground, f.ground),
            );
            push(
                "timer",
                sim.walk_timer == f.timer,
                format!("timer sim={} exp={}", sim.walk_timer, f.timer),
            );
            push(
                "dir",
                sim.dir_right == f.dir_right,
                format!("dir sim={} exp={}", sim.dir_right, f.dir_right),
            );
            push(
                "frame",
                frame_check == f.frame,
                format!("frame sim={} exp={}", frame_check, f.frame),
            );
            push(
                "camx",
                (cam.x as u8) == f.camx && ((cam.x >> 8) as u8 & 0x0F) == (f.camxh & 0x0F),
                format!(
                    "camx sim=${:03X} exp=${:02X}{:X}",
                    cam.x,
                    f.camxh & 0x0F,
                    f.camx
                ),
            );
            push(
                "camy",
                (cam.y as u8) == f.camy && ((cam.y >> 8) as u8 & 0x0F) == (f.camyh & 0x0F),
                format!(
                    "camy sim=${:03X} exp=${:02X}{:X}",
                    cam.y,
                    f.camyh & 0x0F,
                    f.camy
                ),
            );
            push(
                "edge",
                edge_derived == f.edge,
                format!("edge sim=${:02X} exp=${:02X}", edge_derived, f.edge),
            );
            if let Some(pan) = f.pan {
                push(
                    "pan",
                    cam.pan_timer == pan,
                    format!("pan sim=${:02X} exp=${:02X}", cam.pan_timer, pan),
                );
            }
            push(
                "en_r",
                cam.right_enabled == f.en_r,
                format!("en_r sim={} exp={}", cam.right_enabled, f.en_r),
            );
            push(
                "en_l",
                cam.left_enabled == f.en_l,
                format!("en_l sim={} exp={}", cam.left_enabled, f.en_l),
            );
            push(
                "sptr",
                obj.stream_ptr() == f.sptr,
                format!("sptr sim=${:04X} exp=${:04X}", obj.stream_ptr(), f.sptr),
            );
            push(
                "spx",
                obj.spawn_x == f.spx,
                format!("spx sim={} exp={}", obj.spawn_x, f.spx),
            );
            push(
                "sdir",
                obj.scan_dir == f.sdir,
                format!("sdir sim={} exp={}", obj.scan_dir, f.sdir),
            );
            push(
                "standby",
                obj.standby == f.standby,
                format!("standby sim={} exp={}", obj.standby, f.standby),
            );
            push(
                "active",
                obj.active == f.active,
                format!("active sim={} exp={}", obj.active, f.active),
            );
            // The exit-tile arm's own state (only when the capture carries
            // it, so the older captures keep their check count).
            if let Some(v) = f.exit {
                push(
                    "exit",
                    sim.exit_tile_flag == v,
                    format!("exit sim=${:02X} exp=${:02X}", sim.exit_tile_flag, v),
                );
            }
            if let Some(v) = f.lowtime {
                push(
                    "lowtime",
                    sim.low_time_flag == v,
                    format!("lowtime sim=${:02X} exp=${:02X}", sim.low_time_flag, v),
                );
            }
            if let (Some(lo), Some(hi)) = (f.fadelo, f.fadehi) {
                let exp = ((hi as u16) << 8) | lo as u16;
                push(
                    "fade",
                    sim.fade_timer == exp,
                    format!("fade sim={} exp={}", sim.fade_timer, exp),
                );
            }
            if let Some(v) = f.tally {
                push(
                    "tally",
                    sim.level_tally == v,
                    format!("tally sim=${:02X} exp=${:02X}", sim.level_tally, v),
                );
            }
            push("objslots", slot_ok, slot_msg);
            push(
                "odd",
                obj.odd == (f.odd != 0),
                format!("odd sim={} exp={}", obj.odd, f.odd),
            );
            // `sMoonPhysics` ($A287) — the level's own property byte, DERIVED
            // from the manifest. traj19 (level 20, `space_physics` 1) is the one
            // capture that can see it: without the seed this reads $00 against
            // the record's $01 on every frame.
            if let Some(v) = f.moon {
                push(
                    "moon",
                    sim.moon == v,
                    format!("moon sim=${:02X} exp=${:02X}", sim.moon, v),
                );
            }
            // Raw slot bytes: the strongest check (every byte the engine
            // writes, including positions, sprites, frame state, flags).
            if !f.slot_bytes.is_empty() {
                let mut bad: Vec<usize> = Vec::new();
                let mut detail = String::new();
                for (k, hex) in f.slot_bytes.iter().enumerate() {
                    if k >= 16 {
                        break;
                    }
                    let Some(exp) = parse_slot_hex(hex) else {
                        continue;
                    };
                    if obj.slots[k].b != exp {
                        bad.push(k);
                        if detail.is_empty() {
                            let mut diffs = String::new();
                            for (j, &ev) in exp.iter().enumerate() {
                                if obj.slots[k].b[j] != ev {
                                    diffs += &format!(
                                        " +{j:02X} sim=${:02X} exp=${:02X};",
                                        obj.slots[k].b[j], ev
                                    );
                                }
                            }
                            detail = format!("slot {k}:{diffs}");
                        }
                    }
                }
                push("slots_raw", bad.is_empty(), detail);
            }
            if let Some(v) = f.mf {
                push(
                    "mf",
                    obj.move_flags == v,
                    format!("mf sim=${:02X} exp=${:02X}", obj.move_flags, v),
                );
            }
            if let Some(v) = f.fac {
                push(
                    "fac",
                    obj.facing == v,
                    format!("fac sim=${:02X} exp=${:02X}", obj.facing, v),
                );
            }
            if let Some(v) = f.dead {
                push(
                    "dead",
                    obj.mario_dead == v,
                    format!("dead sim={} exp={}", obj.mario_dead, v),
                );
            }
            if let Some(v) = f.lives {
                push(
                    "lives",
                    sim.lives == v,
                    format!("lives sim={:02X} exp={:02X}", sim.lives, v),
                );
            }
            if let Some(v) = f.pow {
                push(
                    "pow",
                    sim.powerup == v,
                    format!("pow sim={} exp={}", sim.powerup, v),
                );
            }
            if let Some(v) = f.inv {
                push(
                    "inv",
                    sim.invincibility == v,
                    format!("inv sim={} exp={}", sim.invincibility, v),
                );
            }
            if let Some(v) = f.goal {
                // The ROM stores 12 (`$3451` id 6); the port only ever tests it
                // for nonzero, so the sim keeps a bool.
                push(
                    "goal",
                    (if sim.goal_bell { 12 } else { 0 }) == v,
                    format!("goal sim={} exp={}", if sim.goal_bell { 12 } else { 0 }, v),
                );
            }
            if let Some(v) = f.secret {
                push(
                    "secret",
                    sim.secret_route == v,
                    format!("secret sim=${:02X} exp=${:02X}", sim.secret_route, v),
                );
            }
            if let Some(v) = f.sbt {
                push(
                    "sbt",
                    obj.stomp_bounce_timer == v,
                    format!("sbt sim={} exp={}", obj.stomp_bounce_timer, v),
                );
            }
            if let Some(v) = f.scc {
                push(
                    "scc",
                    obj.stomp_chain == v,
                    format!("scc sim={} exp={}", obj.stomp_chain, v),
                );
            }
            if let Some(v) = f.mb {
                push(
                    "mb",
                    obj.money_bag == v,
                    format!("mb sim=${:02X} exp=${:02X}", obj.money_bag, v),
                );
            }
            if let Some(v) = f.uphase {
                push(
                    "uphase",
                    obj.update_phase == v,
                    format!("uphase sim={} exp={}", obj.update_phase, v),
                );
            }
            if let Some(v) = f.ff0 {
                push(
                    "ff0",
                    obj.fff0 == v,
                    format!("ff0 sim=${:02X} exp=${:02X}", obj.fff0, v),
                );
            }
            if let Some(v) = f.pipe {
                push(
                    "pipe",
                    sim.pipe_dir == v,
                    format!("pipe sim=${:02X} exp=${:02X}", sim.pipe_dir, v),
                );
            }
            if let Some(v) = f.tr {
                push(
                    "tr",
                    cam.transition == v,
                    format!("tr sim=${:02X} exp=${:02X}", cam.transition, v),
                );
            }
            if let Some(v) = f.pcx {
                push(
                    "pcx",
                    obj.prev_cam_x == v,
                    format!("pcx sim=${:04X} exp=${:04X}", obj.prev_cam_x, v),
                );
            }
            if let Some(v) = f.spin {
                push(
                    "spin",
                    sim.spin_jump_bounce == v,
                    format!("spin sim=${:02X} exp=${:02X}", sim.spin_jump_bounce, v),
                );
            }
            if let Some(v) = f.coins {
                push(
                    "coins",
                    sim.coins_low == v,
                    format!("coins sim=${:02X} exp=${:02X}", sim.coins_low, v),
                );
            }
            if let Some(v) = f.coinshi {
                push(
                    "coinshi",
                    sim.coins_high == v,
                    format!("coinshi sim=${:02X} exp=${:02X}", sim.coins_high, v),
                );
            }
            // The bump slots, byte for byte — the timer sequence, the freed
            // slot's stale Y/X and the two-slot `$01` bump all live here.
            if !f.bslots.is_empty() {
                let mut bad: Vec<usize> = Vec::new();
                let mut detail = String::new();
                for (k, hex) in f.bslots.iter().enumerate() {
                    if k >= 8 {
                        break;
                    }
                    let Some(exp) = parse_slot16_hex(hex) else {
                        continue;
                    };
                    if sim.block_slots[k] != exp {
                        bad.push(k);
                        if detail.is_empty() {
                            let mut diffs = String::new();
                            for (j, &ev) in exp.iter().enumerate() {
                                if sim.block_slots[k][j] != ev {
                                    diffs += &format!(
                                        "+{j:02X} sim=${:02X} exp=${ev:02X}; ",
                                        sim.block_slots[k][j]
                                    );
                                }
                            }
                            detail = format!("bslot {k}: {diffs}");
                        }
                    }
                }
                push(
                    "bslots",
                    bad.is_empty(),
                    if bad.is_empty() {
                        String::new()
                    } else {
                        format!("{detail} ({} bad slots)", bad.len())
                    },
                );
            }
        }
        if counts.is_empty() {
            counts = vec![0; checks.len()];
            names = checks.iter().map(|(n, _, _)| *n).collect();
        }
        for (k, (_name, ok, msg)) in checks.iter().enumerate() {
            if !ok {
                counts[k] += 1;
                if shown < 20 {
                    println!("  frame {i}: {msg}");
                    shown += 1;
                }
            }
        }
    }
    // The exit-tile arm (`$1CBA`): the one-shot facts the per-frame table
    // cannot express (`captures/traj17.json`). `$05E4` is the state-$04 body's
    // entry, so it stops firing the moment `$FF9B` leaves 4 — the capture keeps
    // that hook for the whole in-level run and splices the frames AFTER the arm
    // in from a second hook at `$028D` (one fire per frame in *every* state), so
    // the fade, its `$3547` tally and the exit flag go through the ordinary
    // per-frame checks. What is left for here: the trigger's own geometry, the
    // arm record having that tail, the fade's DURATION (a per-frame table can't
    // count) and the hand-off state the fade's last frame moves to.
    let mut exit_arm_ok = true;
    if let Some(arm) = frames.iter().find_map(|f| f.exit_arm.as_ref()) {
        let mut probs: Vec<String> = Vec::new();
        if arm.frame + 1 > frames.len() - 1 {
            probs.push(format!(
                "arm frame {} has no tail records ({} total)",
                arm.frame,
                frames.len()
            ));
        }
        if !sim.exit_phase {
            probs.push("the port's exit arm did not fire".to_string());
        }
        // The poll starts on the tick AFTER the arm frame — the arm frame's own
        // start is still state 4 (`$05E4` ran and produced the last record),
        // and the switch to 24 happens inside it — so the measured span is the
        // frames after it and the arm frame itself makes up the rest.
        let cap_fade_raw = arm.state_seq.first().map(|s| s[1]).unwrap_or(0);
        let cap_fade = cap_fade_raw + 1;
        if arm.state_seq.first().map(|s| s[0]) != Some(24) {
            probs.push(format!(
                "state_seq does not start in 24: {:?}",
                arm.state_seq
            ));
        }
        if arm.state_seq.last().map(|s| s[0]) != Some(20) {
            probs.push(format!(
                "state_seq does not end in 20 ($14, the hand-off): {:?}",
                arm.state_seq
            ));
        }
        let modeled_fade = 192u64;
        if cap_fade != modeled_fade {
            probs.push(format!(
                "fade length captured={cap_fade} modeled={modeled_fade}"
            ));
        }
        // The tail records stop at the fade's LAST decrement, and the frame that
        // `$3547`'s tail charges the tally on never reaches `$028D` (its
        // gfx/save work spans the whole tick), so run the port's remaining fade
        // steps here and check the charge they produce against the capture's.
        let mut tail_steps = 0u64;
        while sim.fade_timer > 0 {
            sim.exit_fade_step();
            tail_steps += 1;
        }
        if !sim.exit_completed {
            probs.push("the modelled fade never reached its tail ($3547)".to_string());
        }
        if sim.level_tally != arm.tally_after {
            probs.push(format!(
                "tally sim=${:02X} exp=${:02X}",
                sim.level_tally, arm.tally_after
            ));
        }
        println!(
            "  exit arm: record {} of {}, tick {}, x={} y={} col=${:02X}, music={}, \
             fade {cap_fade} captured vs {modeled_fade} modeled \
             ({cap_fade_raw} ticks after the arm frame + the arm frame), \
             tally {:02X}->{:02X}, state_seq {:?}, +{tail_steps} modelled to the tail{}",
            arm.frame,
            frames.len() - 1,
            arm.tick,
            arm.x,
            arm.y,
            arm.col,
            arm.music,
            arm.tally_before,
            arm.tally_after,
            arm.state_seq,
            if probs.is_empty() {
                String::new()
            } else {
                format!("  <- MISMATCH: {}", probs.join("; "))
            }
        );
        exit_arm_ok = probs.is_empty();
    }
    let n = frames.len() - 1;
    println!("replay: {n} frames compared");
    print!("mismatches:");
    for (name, count) in names.iter().zip(counts.iter()) {
        print!(" {name}={count}");
    }
    println!();
    if !gap_ok {
        println!("note: the death-cycle gap check FAILED (fade length / tick accounting)");
    }
    if !exit_arm_ok {
        println!("note: the exit-tile arm check FAILED (see the `exit arm:` line above)");
    }
    if !obj.unported.is_empty() {
        println!(
            "note: unported bank-16 handler path(s) hit: {}",
            obj.unported
                .iter()
                .map(|a| format!("${a:05X}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    // A scene script opcode the port has no implementation for. The VM steps
    // over it with its real operand length, so the script still ends properly,
    // but the behaviour it was supposed to cause did not happen — and this line
    // is what fails the gate if a scene level is ever added to it.
    if !scene_vm.unported().is_empty() {
        println!(
            "note: scene script opcode(s) with no port implementation: {}",
            scene_vm
                .unported()
                .iter()
                .map(|o| format!("${o:02X}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    // A latched scene whose script cursor was never found: the VM stepped
    // nothing at all. This line is what keeps a scene capture from replaying
    // all-zero while the scene never ran — the exact failure the missing
    // `SceneVm::start()` call used to hide in both frame loops.
    if let Some(id) = scene_vm.bad_cursor_scene() {
        println!(
            "note: scene ${id:02X} latched but no script cursor was found - no scene byte ran"
        );
    }
    Ok(())
}

/// Parse a 32-hex-char dump into 16 bytes (a `sBlockAnimSlots` entry).
fn parse_slot16_hex(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Parse a 64-hex-char slot dump into 32 bytes.
fn parse_slot_hex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}
