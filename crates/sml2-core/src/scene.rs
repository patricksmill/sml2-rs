//! The scene-script VM (`$20A4`) and the table it runs.
//!
//! ## What a scene is
//!
//! Crossing a screen edge runs `$0B2A`'s trigger, which sets `sScreenTransition`
//! and calls `$0F0B` — and `$0F0B` latches `sSceneId` from the level's
//! `sSceneMap` cell under Mario (`(Yhi & $0F) * 16 + (Xhi & $0F)`). A nonzero
//! `sSceneId` is what makes the level a *scene* level: the main loop then calls
//! `$20A4` once per frame (home.asm:274, right after the in-level frame pass) and
//! that VM runs **one script byte per frame** — every opcode ends at `$21E8`
//! (`call $2728`, a VBlank wait) before the next fetch.
//!
//! ## The table
//!
//! `$4125` is a 32-entry `dw` table in bank 5, indexed `sSceneId * 2`; the
//! scripts it points at live just above it at `$4165..$41E4` and each ends in
//! `$FF`. `sml2-extract` writes the whole region (192 bytes = 64 table + 128
//! payload) as `extracted/scenes/script_table.bin`, so the pointers stay
//! bank-5 addresses and the port resolves them against the blob.
//!
//! ## What is ported
//!
//! `$4x` — "transition to screen (Y,X)": the operand's high nibble becomes the
//! Y *page* of both the camera and Mario, its low nibble their X page. Only the
//! high bytes are written, so this is a teleport to a 256-px cell boundary, and
//! it is the whole of the level-3 scenes (`40 03 FF` and `40 16 FF`).
//!
//! `$8x` — arms `sBossDefeated` with `$F0 | (opcode & $0F)`; the whole of the
//! two `81 FF` / `82 FF` scenes in level 25.
//!
//! `$FF` — END (`$21EE`): latch `sSceneTransitionFlag` into
//! `sSceneTransitionLatch`, then zero `sSceneId`, `sSceneIdHi`,
//! `sSceneTransitionFlag` and `sSceneDoneFlag`.
//!
//! Every other class is **stepped over with its real operand length** and
//! recorded in [`SceneVm::unported`], so a script still reaches its `$FF` and
//! the runtime can report exactly which opcode it could not honour. The lengths
//! come from the dispatch chain (`$20EB`-`$21AC`) and are validated by the data
//! itself: all 32 retail scripts parse into whole instructions ending precisely
//! on their `$FF` (see the `every_retail_script_parses_into_whole_instructions`
//! test). Two of them are worth flagging, because both were mis-stated when
//! this work was queued:
//!
//! * `$6x` takes **no** operand byte. `$25AF`'s first instruction is
//!   `ld a,[hli]` with HL already on the opcode, so it consumes the opcode byte
//!   itself and the nibble *is* the payload (the tileset index). Reading it as
//!   one operand would make all seven level-entry scripts pick tileset 0.
//! * `$Bx` takes **three**, not two.
//!
//! Classes `$7x`, `$9x`, `$Cx`, `$Dx`, `$Ex` (and any byte that is not exactly
//! `$FF`) have no handler at all: `$21A7`'s last compare falls to `$21E8`, which
//! waits a frame and re-fetches **with HL unchanged** — the VM halts forever
//! re-reading that byte. [`SceneStep::Stalled`] reproduces that without
//! advancing, and no retail script contains one.
//!
//! ## What is not ported
//!
//! `$Bx`/`$0x` (VRAM block transfers, recorded as unported) and the palette
//! writers `$1x`/`$5x`/`$Ax`. `$2x`, `$3x` and `$6x` are ported (see below). The
//! `$4x` direction streamers (`$2359`/`$23E4`/`$2524`/`$246F`) are deliberately
//! not modelled, and not flagged: they fill the BG window at `$9800` with the
//! incoming screen's strip data, which this port never needs — it renders from
//! the fully resident decompressed layout instead, so they have no observable
//! effect here.
//!
//! ## The arena arm (`$2x`) and what it still does not reach
//!
//! `$2x` is more than a state write: `sSceneState = 15` is the only thing that
//! makes `$0EB0`'s `sSceneState & $0F` test pass, and that test is the only
//! caller of `$3C94` — the bank-2 boss load (`$8D9C`: clearing `$AD00-$AF00`,
//! building `sObjectDefIndex` from `sBossFightPhase` and a per-level base,
//! `sTimerReload = 80`, and copying an 18-byte slot image per boss part to
//! `$AD05` at a `+$20` stride) plus the `$8DA9` per-frame driver. The port
//! records the arm ([`SceneVm::boss_arena`]) and the state it writes, but does
//! not spawn the boss: that load needs the `$4E3F` pointer table and the
//! per-phase object records, and no capture has ever reached it (the arena
//! scripts live in levels 8/10/13/16/17/20 and the castle). Queued as
//! `scene_boss_scripts`.
//!
//! `$6x`'s payload is recorded as `sLevelGfxIndex` and the reload it triggers is
//! a rendering-layer refresh: `$25AF` re-enters the level loader (bank 19, then
//! bank 7 at layout index 3), and this port already draws the level from its
//! extracted manifest, which carries that level's tileset.

use crate::camera::CameraSim;
use crate::player_sim::PlayerSim;

/// The operand byte count for each opcode class, read off the `$20EB`-`$21AC`
/// dispatch chain. `None` means the ROM has no handler for that class.
const OPERANDS: [Option<u8>; 16] = [
    Some(7), // $0x — VRAM block transfer (bank + 6 descriptor bytes)
    Some(1), // $1x — sLevelBGPalette
    Some(0), // $2x — sSceneState = 15 + the 96-frame fade
    Some(2), // $3x — camera Y then Mario Y (mode in the opcode's low nibble)
    Some(1), // $4x — the screen cell (Y page, X page)
    Some(0), // $5x — the three palettes to $FF (whiteout)
    Some(0), // $6x — the opcode's own nibble is the tileset index
    None,    // $7x — no handler
    Some(0), // $8x — sBossDefeated
    None,    // $9x — no handler
    Some(0), // $Ax — the 4-frame palette fade
    Some(3), // $Bx — VRAM transfer (bank + 2 descriptor bytes)
    None,    // $Cx — no handler
    None,    // $Dx — no handler
    None,    // $Ex — no handler
    None,    // $Fx — no handler ($FF is END, tested before the nibble dispatch)
];

/// The bank-5 address the extracted blob starts at: its first 64 bytes are the
/// `dw` table, so `$4125 + len` is the end of the payload.
const TABLE_ADDR: usize = 0x4125;

/// The 32-entry script table and its payload, as `sml2-extract` writes them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneScripts {
    blob: Vec<u8>,
}

impl SceneScripts {
    /// The extracted blob's exact size: the 64-byte `dw` table plus the
    /// 128-byte payload (`$4125..$41E5`, where the title-screen gfx begin).
    pub const BLOB_LEN: usize = 192;

    /// `None` unless the blob is exactly the extracted size.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        (bytes.len() == Self::BLOB_LEN).then(|| Self {
            blob: bytes.to_vec(),
        })
    }

    /// No table loaded: the VM is inert (the runtime says so once).
    pub fn is_empty(&self) -> bool {
        self.blob.is_empty()
    }

    /// `$20C5`: the script address for `sSceneId` (`$4125 + id * 2`), or `None`
    /// if the entry is missing or points outside the blob.
    pub fn script(&self, scene_id: u8) -> Option<u16> {
        let at = (scene_id as usize) * 2;
        let addr = (*self.blob.get(at)? as u16) | ((*self.blob.get(at + 1)? as u16) << 8);
        let off = (addr as usize).checked_sub(TABLE_ADDR)?;
        (off < self.blob.len()).then_some(addr)
    }

    /// The byte the cursor points at, or `None` outside the blob.
    fn byte_at(&self, addr: u16) -> Option<u8> {
        self.blob
            .get((addr as usize).checked_sub(TABLE_ADDR)?)
            .copied()
    }

    /// Every `$FF`-terminated script in the blob, as `(scene_id, bytes)` — for
    /// tests and for the extractor's own sanity checks.
    pub fn scripts(&self) -> Vec<(u8, Vec<u8>)> {
        let mut out = Vec::new();
        for id in 0..32u8 {
            let Some(addr) = self.script(id) else {
                continue;
            };
            let mut bytes = Vec::new();
            let mut pc = addr;
            while let Some(b) = self.byte_at(pc) {
                bytes.push(b);
                pc = pc.wrapping_add(1);
                if b == 0xFF {
                    break;
                }
            }
            out.push((id, bytes));
        }
        out
    }
}

/// What one call of [`SceneVm::step`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneStep {
    /// No table is loaded, so nothing can run at all.
    Idle,
    /// `sSceneId == 0`: `$20A4` jumped straight to the `$21EE` epilogue.
    NoScene,
    /// The entry prologue is spending its `$2728` frame wait; no byte ran.
    Entering,
    /// One implemented opcode ran.
    Ran(u8),
    /// A `$FF` ended the script (the epilogue has cleared the scene bytes).
    End,
    /// One opcode ran *and* its class has no port implementation: the byte was
    /// stepped over with its real operand length and recorded in
    /// [`SceneVm::unported`].
    Unported(u8),
    /// The ROM has no handler for this class either: it re-reads the same byte
    /// every frame. The port does too, without advancing.
    Stalled(u8),
    /// The cursor is outside the blob — only reachable with a corrupt table.
    BadCursor,
}

/// The `$20A4` cursor. The scene bytes themselves (`sSceneId`, the transition
/// flag/latch and the done flag) live on [`CameraSim`] with the other RAM the
/// scene code touches; only the script pointer survives across the halt in HL,
/// which is what this type holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneVm {
    /// The script cursor, a bank-5 address inside the blob (the ROM's HL).
    pub pc: u16,
    /// Prologue wait countdown: on console ($20A4 prologue $2CFF -> bank 1 $52E7,
    /// $2728 = the one-byte wait opcode, call $FFA0), sSceneId is latched at f52
    /// and the first script byte executes 15 frames later at f67.
    enter_wait: u8,
    /// `$0F0B` latched a scene, so the script table has been claimed and the
    /// cursor is live. While unarmed the cursor is 0 — see `step`.
    armed: bool,
    /// A latched scene whose script cursor was never found, so the VM ran
    /// nothing: reported through `bad_cursor_scene` and the harness's `note:`.
    bad_cursor_scene: Option<u8>,
    /// Opcode classes with no port implementation, deduped in hit order.
    unported: Vec<u8>,
    /// `$2x` has armed the boss arena. The ROM's `$0EB0` gate now calls
    /// `$3C94` → bank 2 `$8D9C`, which spawns the boss; this port has no boss
    /// to spawn, so it records the arm instead of silently dropping it.
    pub boss_arena: bool,
}

impl SceneVm {
    /// Prologue wait duration in frames between scene latch and the first script byte execution.
    /// On console ($20A4 prologue $2CFF -> bank 1 $52E7, $2728 call $FFA0),
    /// sSceneId is latched at f52 and first script opcode executes at f67 (15 frames).
    pub const PROLOGUE_FRAMES: u8 = 15;

    pub fn new() -> Self {
        Self::default()
    }

    /// `$0F0B` latched a scene id: arm the VM. The ROM's prologue then spends
    /// 15 frames (OAM latch, shadow close, VBlank wait, OAM DMA, bank select, table
    /// lookup, $2728 wait opcode) before the first byte runs.
    pub fn start(&mut self) {
        self.enter_wait = Self::PROLOGUE_FRAMES;
        self.armed = true;
        self.bad_cursor_scene = None;
        self.pc = 0;
    }

    /// Back to the unarmed state. The ROM's cursor (HL) is never zero, so
    /// `pc == 0` marks an unarmed VM — and an unarmed VM with a latched scene id
    /// is exactly the state `$0F0B` leaves behind: the latch is written by the
    /// CAMERA pass and `$20A4` runs later in the same frame, after the state
    /// dispatch, so the VM arms on its own first pass after a latch instead of
    /// needing a caller. That caller used to be missing entirely: `start()` had
    /// no call site anywhere, so a latched scene stepped `byte_at(0)` →
    /// `BadCursor`, ran NOTHING, and did it silently in BOTH frame loops.
    fn disarm(&mut self) {
        self.enter_wait = 0;
        self.armed = false;
        self.pc = 0;
    }

    /// A scene id is latched and no script cursor was ever found, so the VM
    /// stepped nothing. The harness prints this as a `note:`, which is what
    /// stops a scene capture from passing green on top of a dead VM — the exact
    /// failure the missing `start()` call would otherwise have hidden.
    pub fn bad_cursor_scene(&self) -> Option<u8> {
        self.bad_cursor_scene
    }

    /// One main-loop pass of `$20A4`. The ROM calls it once per frame, after
    /// the in-level frame pass, so a scene advances exactly one byte per frame.
    pub fn step(
        &mut self,
        sim: &mut PlayerSim,
        cam: &mut CameraSim,
        scripts: &SceneScripts,
    ) -> SceneStep {
        if scripts.is_empty() {
            return SceneStep::Idle;
        }
        // `$20A4`'s guard: `ld a,[sSceneId] / ld b,a / ld a,[sSceneIdHi] / or b
        // / jp z,$21EE`. `sSceneIdHi` is never written by any code path in this
        // ROM, so the test reduces to the id itself.
        if cam.scene_id == 0 {
            cam.end_scene();
            self.disarm();
            return SceneStep::NoScene;
        }
        // `$0F0B` latched a scene on this frame's camera pass, and `$20A4` runs
        // after the state dispatch — so the prologue starts here, on the pass
        // that follows the latch, and the first script byte runs on the next.
        if !self.armed {
            self.start();
        }
        if self.enter_wait > 0 {
            if self.pc == 0 {
                self.pc = scripts.script(cam.scene_id).unwrap_or(0);
            }
            self.enter_wait -= 1;
            return SceneStep::Entering;
        }
        let op = match scripts.byte_at(self.pc) {
            Some(b) => b,
            None => {
                self.bad_cursor_scene = Some(cam.scene_id);
                return SceneStep::BadCursor;
            }
        };
        if op == 0xFF {
            // The fetch loop does `inc hl` before jumping to the epilogue, so
            // the cursor ends one past the terminator.
            self.pc = self.pc.wrapping_add(1);
            cam.end_scene();
            return SceneStep::End;
        }
        let class = op >> 4;
        let Some(len) = OPERANDS[class as usize] else {
            // `$21A7`'s last compare falls through to `$21E8`, which waits and
            // re-fetches with HL unchanged: the ROM halts on this byte forever.
            self.note_unported(op);
            return SceneStep::Stalled(op);
        };
        let operand = scripts.byte_at(self.pc.wrapping_add(1)).unwrap_or(0);
        match class {
            0x2 => self.op_2x(sim, cam),
            0x3 => {
                let second = scripts.byte_at(self.pc.wrapping_add(2)).unwrap_or(0);
                self.op_3x(op, operand, second, sim, cam);
            }
            0x4 => self.op_4x(operand, sim, cam),
            0x6 => sim.level_gfx_index = op & 0x0F,
            0x8 => cam.boss_defeated = 0xF0 | (op & 0x0F),
            _ => self.note_unported(op),
        }
        self.pc = self.pc.wrapping_add(1 + len as u16);
        if matches!(class, 0x2 | 0x3 | 0x4 | 0x6 | 0x8) {
            SceneStep::Ran(op)
        } else {
            SceneStep::Unported(op)
        }
    }

    /// `$2x` / `$20EB`: arm the boss arena. `sSceneState = 15`, then
    /// `sFadeTimerHigh = 1` / `sFadeTimerLow = 96` (a 352-frame fade), then
    /// `sTileAnimConfig`, `sStarTimerHigh` and `sStarTimerLow` cleared. No
    /// operand — the dispatcher's `inc hl` skips only the opcode.
    ///
    /// `sSceneState = 15` is load-bearing: it is what makes `$0EB0`'s
    /// `sSceneState & $0F` test pass, the test that reaches `$3C94` and through
    /// it the bank-2 boss load. The port records the arm rather than pretending
    /// to spawn something ([`SceneVm::boss_arena`]), and it does not model the
    /// star timer at all (it has no star powerup), so only the fade and the
    /// state are observable here.
    fn op_2x(&mut self, sim: &mut PlayerSim, cam: &mut CameraSim) {
        cam.scene_state = 15;
        sim.fade_timer = 0x0160;
        self.boss_arena = true;
    }

    /// `$3x` / `$211E`: two operands, camera Y first, then Mario Y. A **zero low
    /// nibble means absolute**: the ROM writes only the low bytes (`$FFC8`,
    /// `$FFC0`), so the pair lands inside the current 256-px page and the high
    /// bytes keep whatever the camera had. Any other nibble is a 16-bit **add**
    /// of each operand to the same pair — the arena entrance's camera slide, and
    /// the nibble is the direction that slide's sprite selection keys off.
    fn op_3x(&mut self, op: u8, cam_y: u8, mario_y: u8, sim: &mut PlayerSim, cam: &mut CameraSim) {
        if op & 0x0F == 0 {
            cam.y = (cam.y & 0xFF00) | cam_y as u16;
            sim.y = (sim.y & 0xFF00) | mario_y as u16;
        } else {
            cam.y = cam.y.wrapping_add(cam_y as u16);
            sim.y = sim.y.wrapping_add(mario_y as u16);
        }
    }

    /// `$4x` / `$2323`: the operand's high nibble becomes the camera's *and*
    /// Mario's Y page (`$FFC9` / `$FFC1`), its low nibble their X page (`$FFCB`
    /// / `$FFC3`). Only the high bytes are written — the low bytes keep their
    /// values, so this lands the pair on a 256-px cell boundary rather than at a
    /// pixel address. (The operand byte is read twice in the ROM, once per
    /// nibble, which is why both pairs get the same page.) The dispatcher's tail
    /// then sets `sSceneTransitionFlag = 1`, which `$21EE` later latches into
    /// `sSceneTransitionLatch`.
    ///
    /// The handler also spends a frame and, *only while a screen transition is
    /// in progress*, runs the direction-specific destination-screen streamers.
    /// Those are not modelled and need not be: they fill the BG window at
    /// `$9800`, and this port renders from the resident layout (see the module
    /// docs).
    fn op_4x(&mut self, operand: u8, sim: &mut PlayerSim, cam: &mut CameraSim) {
        let y_page = (operand >> 4) as u16;
        let x_page = (operand & 0x0F) as u16;
        cam.y = (cam.y & 0x00FF) | (y_page << 8);
        sim.y = (sim.y & 0x00FF) | (y_page << 8);
        cam.x = (cam.x & 0x00FF) | (x_page << 8);
        sim.x = (sim.x & 0x00FF) | (x_page << 8);
        cam.scene_transition_flag = 1;
    }

    fn note_unported(&mut self, op: u8) {
        if !self.unported.contains(&op) {
            self.unported.push(op);
        }
    }

    /// Opcode classes with no port implementation, in hit order (deduped).
    pub fn unported(&self) -> &[u8] {
        &self.unported
    }
}

/// Lookup boss spawn definition from the ROM bank 2 tables ($4E3F, $4E9A, $4EAC).
///
/// Returns `(count, hits, sprite, obj0_def, Vec<sub_obj_defs>)`.
///
/// DATA ONLY, AND NO CALLER YET: this is the `$8DFD`/`$8E6C` read side
/// (`sObjectDefIndex` -> the 3-byte count/hits/sprite record through the
/// 9-pointer table at `$4E3F`, `sObject0Def` through `$4E9A`, and
/// `sObjectDefCount` 18-byte images through `$4EAC`). Nothing spawns a boss
/// with it: the `$0EB0` scene branch, the `$8DA9` alternate driver and the
/// slot staging into `$AD05` are still unported (see the `scene_boss_spawn`
/// queue item). It is here because the table is ROM-verified data, not
/// because the boss exists.
///
/// `$8DFD` computes the index as `sBossFightPhase + b` with `b` per level
/// (level 5 -> 0, 9 -> 1, $0D -> 2, $10 -> 3, $13 -> 4, $17 -> 5, else 6);
/// the record table carries a 9th entry at index 8 (count 0, sprite 99 --
/// the "no boss at this phase" record) and the port clamps everything at or
/// above it to that entry rather than reading past the pointer table as the
/// ROM would.
/// `$8DFD`: `sObjectDefIndex = sBossFightPhase + b`, where the ladder `b`
/// keys the fight's def by the *level*: `$05` -> 0, `$09` -> 1, `$0D` -> 2,
/// `$10` -> 3, `$13` -> 4, `$17` -> 5, and anything else (including `$18`,
/// the Wario castle) -> 6. That index selects the 3-byte record (count, hits,
/// sprite) and the two 18-byte def images in [`lookup_boss_spawn_def`].
/// Definitions accepted through normal arena entry: 1 (level `$09`), 2
/// (`$0D`), 3 (`$10`), 4 (`$13`), and Wario's 6/7/8 (`$18`). The regular fights finish
/// through reward and level reset; Wario uses phase exits and the ending.
/// Unsupported definitions still use `CameraSim::take_blocked_scene_load`.
pub fn boss_def_portable(def_index: u8) -> bool {
    matches!(def_index, 1 | 2 | 3 | 4 | 6 | 7 | 8)
}

pub fn boss_def_index(level_id: u8, phase: u8) -> u8 {
    let b = match level_id {
        0x05 => 0,
        0x09 => 1,
        0x0D => 2,
        0x10 => 3,
        0x13 => 4,
        0x17 => 5,
        _ => 6,
    };
    phase.wrapping_add(b)
}

pub fn lookup_boss_spawn_def(def_index: u8) -> (u8, u8, u8, [u8; 18], Vec<[u8; 18]>) {
    match def_index {
        0 => (
            1, 6, 1,
            [1, 0, 0, 0, 124, 88, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![[2, 0, 0, 0, 129, 88, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
        ),
        1 => (
            3, 6, 18,
            [6, 0, 0, 0, 112, 152, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![
                [7, 0, 0, 0, 132, 56, 19, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0],
                [8, 0, 0, 0, 116, 88, 19, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0],
                [7, 0, 0, 0, 132, 120, 19, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0],
            ],
        ),
        2 => (
            2, 6, 42,
            [12, 0, 0, 0, 64, 88, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![
                [11, 0, 0, 6, 96, 136, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                [10, 0, 0, 6, 96, 40, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ],
        ),
        3 => (
            2, 6, 48,
            [14, 0, 0, 0, 144, 88, 44, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![
                [15, 0, 0, 0, 80, 208, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                [15, 0, 0, 0, 80, 208, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ],
        ),
        4 => (
            2, 6, 54,
            [16, 0, 0, 0, 48, 144, 52, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![
                [17, 0, 0, 0, 48, 144, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                [18, 0, 0, 0, 64, 152, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ],
        ),
        5 => (
            1, 6, 72,
            [19, 0, 0, 0, 64, 176, 66, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![[13, 0, 0, 0, 208, 208, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
        ),
        6 => (
            1, 6, 82,
            [20, 0, 0, 0, 104, 144, 80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![[21, 0, 0, 0, 56, 160, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
        ),
        7 => (
            1, 6, 82,
            [23, 0, 0, 0, 136, 144, 76, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![[24, 0, 0, 0, 56, 160, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]],
        ),
        // Index 8 = the record table's 9th entry ($4E69: count 0, hits 6,
        // sprite 99) with its own `sObject0Def` image from `$4E9A[8]` =
        // `$4F4C`; `sObjectDefCount` 0 means `$8E6C` returns before it ever
        // reads the `$4EAC` table, so there are no sub-objects to return.
        _ => (
            0, 6, 99,
            [26, 0, 0, 0, 136, 144, 83, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            Vec::new(),
        ),
    }
}

    /// `$8DFD`'s ladder keys the fight def by the level: `$05` -> 0, `$09` -> 1,
    /// `$0D` -> 2, `$10` -> 3, `$13` -> 4, `$17` -> 5, anything else -> 6
    /// (including `$18`, the Wario castle), plus `sBossFightPhase`.
    #[test]
    fn the_boss_def_index_follows_the_roms_level_ladder() {
        assert_eq!(boss_def_index(0x05, 0), 0);
        assert_eq!(boss_def_index(0x09, 0), 1);
        assert_eq!(boss_def_index(0x0D, 0), 2);
        assert_eq!(boss_def_index(0x10, 0), 3);
        assert_eq!(boss_def_index(0x13, 0), 4);
        assert_eq!(boss_def_index(0x17, 0), 5);
        assert_eq!(boss_def_index(0x18, 0), 6);
        assert_eq!(boss_def_index(0x01, 0), 6);
        // The ladder is checked in ROM order, so $05 stays 0 and the rest step
        // up by one; a level that is *not* in it lands on 6.
        assert_eq!(boss_def_index(0x13, 1), 5, "phase adds on top");
        assert_eq!(boss_def_index(0x18, 2), 8);
    }

#[cfg(test)]
mod tests {
    use super::*;

    /// The real blob `sml2-extract` writes (`extracted/scenes/script_table.bin`
    /// = bank 5 `$4125..$41E4`): the 32 `dw` entries followed by the scripts.
    /// Embedded so the tests need no file and still pin the ROM's own bytes —
    /// scene 6's script is at `$4177` (`40 03 FF`) and scene 7's at `$417A`
    /// (`40 16 FF`).
    const BLOB_HEX: &str = "\
        65 41 66 41 69 41 6C 41 6F 41 74 41 77 41 7A 41 \
        7D 41 7F 41 81 41 84 41 8C 41 94 41 9C 41 A4 41 \
        AC 41 B4 41 B7 41 BA 41 C2 41 C5 41 C8 41 CB 41 \
        CE 41 D1 41 D4 41 D7 41 D9 41 DC 41 DF 41 E2 41 \
        FF 40 3B FF 40 18 FF 40 29 FF B2 1E 00 6C FF 40 \
        19 FF 40 03 FF 40 16 FF 81 FF 82 FF 40 21 FF 60 \
        20 30 60 60 40 04 FF 62 20 30 58 60 40 00 FF 63 \
        20 30 58 58 40 01 FF 65 20 30 60 68 40 03 FF 64 \
        20 30 58 60 40 02 FF 61 20 30 60 68 40 05 FF 40 \
        21 FF 40 02 FF 66 20 30 C0 C8 40 06 FF 40 10 FF \
        40 20 FF 40 11 FF 40 3B FF 40 01 FF 40 3E FF 40 \
        0B FF 20 FF 40 1D FF 40 19 FF 40 19 FF 40 19 FF";

    fn scripts() -> SceneScripts {
        let bytes: Vec<u8> = BLOB_HEX
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).unwrap())
            .collect();
        SceneScripts::from_bytes(&bytes).expect("192-byte blob")
    }

    /// A camera latched onto a scene id, with Mario standing somewhere the
    /// page writes can be told apart from the low bytes.
    fn latched(cam: &mut CameraSim, vm: &mut SceneVm, id: u8) {
        cam.scene_id = id;
        vm.start();
    }

    fn step_prologue(vm: &mut SceneVm, sim: &mut PlayerSim, cam: &mut CameraSim, s: &SceneScripts) {
        for _ in 0..SceneVm::PROLOGUE_FRAMES {
            assert_eq!(vm.step(sim, cam, s), SceneStep::Entering);
        }
    }

    /// The operand lengths are validated by the data: every one of the 32
    /// retail scripts must parse into whole instructions that end *exactly* on
    /// its `$FF`. Reading `$6x` as a one-operand class (as the work was
    /// originally queued) makes the seven level-entry scripts run past their
    /// terminators and fail here.
    #[test]
    fn every_retail_script_parses_into_whole_instructions() {
        let s = scripts();
        let mut seen = 0;
        for id in 0..32u8 {
            let addr = s.script(id).expect("table entry");
            let mut pc = addr;
            loop {
                let op = s.byte_at(pc).expect("inside the blob");
                if op == 0xFF {
                    break;
                }
                let len = OPERANDS[(op >> 4) as usize]
                    .unwrap_or_else(|| panic!("scene {id}: illegal opcode ${op:02X}"));
                pc += 1 + len as u16;
            }
            seen += 1;
        }
        assert_eq!(seen, 32, "all 32 entries resolve");
        // Scene 6 and 7 are the level-3 pair, and each is exactly one $4x.
        assert_eq!(s.script(6), Some(0x4177));
        assert_eq!(s.script(7), Some(0x417A));
    }

    /// The `$4x` opcode is a teleport of Mario *and* the camera to the operand's
    /// cell: high nibble -> Y page, low nibble -> X page, low bytes untouched.
    #[test]
    fn four_x_sets_both_pages_and_keeps_the_low_bytes() {
        let s = scripts();
        let mut vm = SceneVm::new();
        let mut cam = CameraSim::new(0x1234, 0x1567);
        let mut sim = PlayerSim::new();
        sim.x = 0x12AB;
        sim.y = 0x15CD;
        latched(&mut cam, &mut vm, 6);

        // 15 frames of prologue entering; the first opcode runs on the 16th pass.
        step_prologue(&mut vm, &mut sim, &mut cam, &s);
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Ran(0x40));

        // Scene 6 = `40 03`: Y page 0, X page 3.
        assert_eq!(cam.y, 0x0067, "camera Y page 0, low byte kept");
        assert_eq!(sim.y, 0x00CD, "Mario Y page 0, low byte kept");
        assert_eq!(cam.x, 0x0334, "camera X page 3, low byte kept");
        assert_eq!(sim.x, 0x03AB, "Mario X page 3, low byte kept");
        assert_eq!(cam.scene_transition_flag, 1, "the $4x tail sets it");

        // Scene 7 = `40 16`: Y page 1, X page 6.
        let mut vm7 = SceneVm::new();
        let mut cam7 = CameraSim::new(0, 0);
        let mut sim7 = PlayerSim::new();
        latched(&mut cam7, &mut vm7, 7);
        step_prologue(&mut vm7, &mut sim7, &mut cam7, &s);
        assert_eq!(vm7.step(&mut sim7, &mut cam7, &s), SceneStep::Ran(0x40));
        assert_eq!((cam7.y >> 8, cam7.x >> 8), (1, 6));
        assert_eq!((sim7.y >> 8, sim7.x >> 8), (1, 6));
    }

    /// The arena intro end to end, on scene 11's real script
    /// (`60 20 30 60 60 40 04 FF` at `$4184`): the tileset swap, the arena arm,
    /// the camera/Mario Y set, the screen cell, and the terminator — with
    /// nothing flagged. Before `$2x`/`$3x`/`$6x` were ported this script reached
    /// its `$4x` having recorded the first three as unported and armed nothing.
    #[test]
    fn an_arena_script_runs_its_whole_intro() {
        let s = scripts();
        assert_eq!(s.script(11), Some(0x4184));
        let mut cam = CameraSim::new(0x1200, 0x1300);
        let mut sim = PlayerSim::new();
        let mut vm = SceneVm::new();
        latched(&mut cam, &mut vm, 11);

        step_prologue(&mut vm, &mut sim, &mut cam, &s);
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Ran(0x60));
        assert_eq!(sim.level_gfx_index, 0, "$6x's nibble is the tileset index");
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Ran(0x20));
        assert_eq!(cam.scene_state, 15, "$20EB's arm");
        assert_eq!(sim.fade_timer, 0x0160, "sFadeTimerHigh=1, sFadeTimerLow=96");
        assert!(
            vm.boss_arena,
            "the arm is recorded; the boss itself is not spawned"
        );
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Ran(0x30));
        assert_eq!(cam.y, 0x1360, "camera Y low byte, page kept");
        assert_eq!(sim.y, 0x0060, "and Mario's");
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Ran(0x40));
        assert_eq!(
            (cam.y, cam.x),
            (0x0060, 0x0400),
            "operand $04: Y page 0, X 4"
        );
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::End);
        assert!(vm.unported().is_empty(), "every byte of it is honoured");
    }

    /// `$3x`'s other mode: a non-zero low nibble is a 16-bit **add** of each
    /// operand to the camera and Mario Y pairs, so the carry moves between
    /// pages. No retail script uses it (all seven arena intros write `30`), which
    /// is why this is driven from a synthetic script.
    #[test]
    fn the_3x_add_mode_carries_between_pages() {
        let mut bytes: Vec<u8> = BLOB_HEX
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).unwrap())
            .collect();
        // Scene 15's script ($41A4, payload index $7F) becomes `3D 10 20 FF`.
        assert_eq!(0x41A4 - 0x4125, 0x7F);
        bytes[0x7F..0x83].copy_from_slice(&[0x3D, 0x10, 0x20, 0xFF]);
        let s = SceneScripts::from_bytes(&bytes).expect("192-byte blob");
        assert_eq!(s.script(15), Some(0x41A4));

        let mut cam = CameraSim::new(0, 0x00F8);
        let mut sim = PlayerSim::new();
        sim.y = 0x01F0;
        let mut vm = SceneVm::new();
        latched(&mut cam, &mut vm, 15);
        step_prologue(&mut vm, &mut sim, &mut cam, &s);
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Ran(0x3D));
        assert_eq!(cam.y, 0x0108, "F8 + 10 carries into the next page");
        assert_eq!(sim.y, 0x0210, "F0 + 20 likewise");
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::End);
    }

    /// `$FF` ends the script and runs the `$21EE` epilogue: the transition flag
    /// is latched and every scene byte is zeroed, so the next pass takes the
    /// no-scene path.
    #[test]
    fn end_latches_the_flag_and_clears_the_scene_bytes() {
        let s = scripts();
        let mut vm = SceneVm::new();
        let mut cam = CameraSim::new(0, 0);
        let mut sim = PlayerSim::new();
        latched(&mut cam, &mut vm, 6);
        step_prologue(&mut vm, &mut sim, &mut cam, &s);
        vm.step(&mut sim, &mut cam, &s); // the $40
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::End);

        assert_eq!(cam.scene_transition_latch, 1, "$A249 = $A24A");
        assert_eq!(cam.scene_id, 0);
        assert_eq!(cam.scene_transition_flag, 0);
        assert_eq!(cam.scene_done_flag, 0);
        assert_eq!(vm.pc, 0x417A, "one past the $FF at $4179");
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::NoScene);
    }

    /// `81 FF` / `82 FF` (level 25's scenes 8 and 9) arm `sBossDefeated` with
    /// `$F0 | nibble` — the whole of those two scripts.
    #[test]
    fn eight_x_arms_boss_defeated_from_its_own_nibble() {
        let s = scripts();
        for (id, want) in [(8u8, 0xF1u8), (9, 0xF2)] {
            let mut vm = SceneVm::new();
            let mut cam = CameraSim::new(0, 0);
            let mut sim = PlayerSim::new();
            latched(&mut cam, &mut vm, id);
            step_prologue(&mut vm, &mut sim, &mut cam, &s);
            assert_eq!(
                vm.step(&mut sim, &mut cam, &s),
                SceneStep::Ran(want & 0x0F | 0x80)
            );
            assert_eq!(cam.boss_defeated, want);
            assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::End);
        }
    }

    /// A class the ROM has no handler for halts the VM on that byte: the cursor
    /// does not advance, and the pass repeats it forever.
    #[test]
    fn an_unhandled_class_stalls_without_advancing() {
        // A synthetic table whose scene 1 script is `7F FF` (no handler).
        let mut blob = vec![0u8; SceneScripts::BLOB_LEN];
        blob[2] = 0x65; // scene 1 -> $4165
        blob[3] = 0x41;
        blob[0x40] = 0x7F;
        blob[0x41] = 0xFF;
        let s = SceneScripts::from_bytes(&blob).unwrap();

        let mut vm = SceneVm::new();
        let mut cam = CameraSim::new(0, 0);
        let mut sim = PlayerSim::new();
        latched(&mut cam, &mut vm, 1);
        step_prologue(&mut vm, &mut sim, &mut cam, &s);
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Stalled(0x7F));
        assert_eq!(vm.pc, 0x4165, "HL is unchanged on the halt path");
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Stalled(0x7F));
        assert_eq!(vm.unported(), &[0x7F]);
    }

    /// An unimplemented *but handled* class is stepped over with its real
    /// operand length, so the script still reaches its `$FF` — and the opcode is
    /// reported rather than silently ignored. `$Bx` is the case now that `$2x`,
    /// `$3x` and `$6x` are ported: it is a VRAM block transfer, and its whole
    /// effect lands in the BG window this port never draws.
    #[test]
    fn an_unported_class_is_stepped_over_by_its_real_length() {
        // Scene 1's script `B3 11 22 33 FF`: a $Bx (three operands).
        let mut blob = vec![0u8; SceneScripts::BLOB_LEN];
        blob[2] = 0x65;
        blob[3] = 0x41;
        blob[0x40..0x45].copy_from_slice(&[0xB3, 0x11, 0x22, 0x33, 0xFF]);
        let s = SceneScripts::from_bytes(&blob).unwrap();

        let mut vm = SceneVm::new();
        let mut cam = CameraSim::new(0, 0);
        let mut sim = PlayerSim::new();
        latched(&mut cam, &mut vm, 1);
        step_prologue(&mut vm, &mut sim, &mut cam, &s);
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Unported(0xB3));
        assert_eq!(vm.pc, 0x4169, "one opcode + three operands");
        assert_eq!(vm.unported(), &[0xB3]);
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::End);
    }

    /// With no table extracted the VM is inert: nothing runs and, in
    /// particular, no epilogue fires on an id it cannot resolve.
    #[test]
    fn a_missing_table_makes_the_vm_inert() {
        let empty = SceneScripts::default();
        let mut vm = SceneVm::new();
        let mut cam = CameraSim::new(0, 0);
        let mut sim = PlayerSim::new();
        cam.scene_id = 6;
        vm.start();
        assert_eq!(vm.step(&mut sim, &mut cam, &empty), SceneStep::Idle);
        assert_eq!(cam.scene_id, 6, "the id is left alone");
    }

    /// A blob of the wrong size is refused outright rather than half-read.
    #[test]
    fn a_short_blob_is_rejected() {
        assert!(SceneScripts::from_bytes(&[0u8; 191]).is_none());
        assert!(SceneScripts::from_bytes(&[0u8; 193]).is_none());
        assert!(SceneScripts::from_bytes(&[0u8; 192]).is_some());
    }

    /// Every script in the real blob is reachable and ends in `$FF`; the list
    /// also documents which scenes carry which opcodes for the queue item.
    #[test]
    fn the_real_blob_lists_all_thirty_two_scripts() {
        let all = scripts().scripts();
        assert_eq!(all.len(), 32);
        assert_eq!(all[6].1, vec![0x40, 0x03, 0xFF]);
        assert_eq!(all[7].1, vec![0x40, 0x16, 0xFF]);
        assert_eq!(all[8].1, vec![0x81, 0xFF]);
        assert_eq!(all[27].1, vec![0x20, 0xFF]);
        assert!(all.iter().all(|(_, b)| b.last() == Some(&0xFF)));
        // The classes that are absent from every retail script: implementing
        // them would add no reachable behaviour today.
        for absent in [0x0u8, 0x1, 0x5, 0xA] {
            assert!(
                all.iter().all(|(_, b)| b[0] >> 4 != absent),
                "class ${absent:x}x is unused in retail data"
            );
        }
    }

    #[test]
    fn boss_spawn_definitions_match_extracted_records() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../extracted/scenes/boss_spawns.json");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return,
        };
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 9, "9 records ($4E51..$4E6B) -- see the extractor");
        for (i, entry) in arr.iter().enumerate() {
            let (count, hits, spr, obj0, subs) = lookup_boss_spawn_def(i as u8);
            assert_eq!(entry["count"].as_u64().unwrap() as u8, count, "entry {i} count");
            assert_eq!(entry["hits"].as_u64().unwrap() as u8, hits, "entry {i} hits");
            assert_eq!(entry["sprite"].as_u64().unwrap() as u8, spr, "entry {i} sprite");
            let obj0_json: Vec<u8> = entry["obj0_def"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect();
            assert_eq!(&obj0[..], &obj0_json[..], "entry {i} obj0");
            let subs_json = entry["sub_obj_defs"].as_array().unwrap();
            assert_eq!(subs.len(), subs_json.len(), "entry {i} subs len");
            for (j, s) in subs_json.iter().enumerate() {
                let sub_bytes: Vec<u8> = s
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u8)
                    .collect();
                assert_eq!(&subs[j][..], &sub_bytes[..], "entry {i} sub {j}");
            }
        }
    }

    /// The latch must ARM the VM by itself. `$0F0B` writes `sSceneId` on the
    /// camera pass and `$20A4` runs later in that same frame, so there is no
    /// other caller to arm it — and `start()` having no call site anywhere was a
    /// real, silent bug: a latched scene stepped `byte_at(0)` -> `BadCursor`,
    /// ran nothing, and both frame loops discarded the return value. This test
    /// FAILS against that build (it expected `Entering` and got `BadCursor`).
    #[test]
    fn a_latched_scene_arms_itself_and_runs_to_its_terminator() {
        let s = scripts();
        let mut vm = SceneVm::new();
        let mut cam = CameraSim::new(0x0320, 0x01C0);
        let mut sim = PlayerSim::new();
        sim.x = 0x0311;                     // Mario's X page 3; low byte must survive
        sim.y = 0x00D4;
        cam.scene_id = 7;                   // level 3's cell i4 latches scene 7

        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Entering,
                   "the prologue claims the cursor on the pass after the latch");
        assert_ne!(vm.pc, 0, "the cursor comes from the script table, not from zero");

        let mut ran = Vec::new();
        for _ in 0..20 {
            match vm.step(&mut sim, &mut cam, &s) {
                SceneStep::Entering => {}
                SceneStep::Ran(op) => ran.push(op),
                SceneStep::End => break,
                other => panic!("unexpected {other:?} before the terminator"),
            }
        }
        assert_eq!(ran, vec![0x40], "scene 7 is `40 16 FF`: one $4x, then the end");
        assert_eq!(sim.x >> 8, 6, "$4x $16 -> Mario X page 6");
        assert_eq!(sim.y >> 8, 1, "$4x $16 -> Mario Y page 1");
        assert_eq!(sim.x & 0xFF, 0x11, "the low byte is untouched");
        assert_eq!(vm.bad_cursor_scene(), None);
    }

    /// A latched id the script table cannot resolve must be REPORTED, not
    /// silently stepped past: the harness turns this into a `note:`, which is
    /// what keeps a scene capture from replaying all-zero on top of a dead VM.
    #[test]
    fn an_unresolvable_scene_id_is_reported() {
        let s = scripts();
        let mut vm = SceneVm::new();
        let mut cam = CameraSim::new(0, 0);
        let mut sim = PlayerSim::new();
        cam.scene_id = 0x20;                // past the 32-entry table
        step_prologue(&mut vm, &mut sim, &mut cam, &s);
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::BadCursor);
        assert_eq!(vm.bad_cursor_scene(), Some(0x20));
    }

    /// On console, $20A4's prologue spends exactly 15 frames between the scene
    /// latch and the execution of the first script opcode ($2CFF -> $52E7, $2728
    /// wait opcode, call $FFA0).
    #[test]
    fn scene_vm_prologue_waits_exact_fifteen_frames_before_first_opcode() {
        let s = scripts();
        let mut cam = CameraSim::new(0, 0);
        let mut sim = PlayerSim::new();
        let mut vm = SceneVm::new();
        latched(&mut cam, &mut vm, 6);

        for i in 0..SceneVm::PROLOGUE_FRAMES {
            assert_eq!(
                vm.step(&mut sim, &mut cam, &s),
                SceneStep::Entering,
                "frame {} must be Entering",
                i
            );
        }
        assert_eq!(vm.step(&mut sim, &mut cam, &s), SceneStep::Ran(0x40));
    }
}
