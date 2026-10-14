//! The overworld map — `sGameState` `$0C`, bank 15's `$3C000` dispatcher.
//!
//! The port's in-level sims never run this state (the replay harness only ever
//! sees state `$04`), so the map is its own simulation: [`MapSim::tick`] is one
//! map frame, driven by the two joypad bytes, and it hands back the writes the
//! ROM makes *outside* the map's own RAM block (the level it entered, the state
//! it moved to, the save it wrote) as [`MapEntry`].
//!
//! ## What one frame is
//!
//! `$3C000` switches on `sMapScreenId` (`$A68B`): `0` is the hub, `1`-`9` the
//! zone screens, `$0A`+ the castle/ending screens. Every screen handler calls
//! `$3C13E` (the frame/tick bookkeeping), its own screen routine (tiles, sprites,
//! the camera latch — art, not modelled), then `$3C38C`, which is the cursor:
//! while `sMapCursorWalking` is set it advances the walk, otherwise it runs the
//! `$3E1D7` gate and either the node's own entry action or the direction input.
//! The frame ends with `$3C169`, the A/START check that starts a level. The hub
//! runs the cursor *first* (`$3ED4C`/`$3C38C`/`$3C13E`/`$3CFC8`), every other
//! screen runs its routine first (`$3C13E`/`$3D8E6`/`$3C38C`).
//!
//! ## The cursor's walk
//!
//! The cursor moves one pixel per *move frame*, and a walk's stream decides which
//! frames are move frames: `$3C7EC` rotates `sMapCursorAnim` ($A878) right and
//! skips the frame when the bit rotated out was set. That byte is not a timer —
//! it is part of the 64-byte save image, so the ROM's own value depends on the
//! last transition that wrote it (`$3E3BA` writes `$92`, `$3EA9A` writes `0`),
//! and with `0` the cursor moves *every* frame. Polarity from the ROM's
//! `rrca`/`jr nc`: `$92` gives five move frames out of every eight.
//!
//! A walk starts from `$563A + node*8 + dir*2` ([`MapWalk`]) — or, at nodes `$0C`
//! and `$1E`, from a *hardcoded* stream at `$5BF6` when A is pressed with nothing
//! else held (`$3C4E7`, the secret route). The first stream byte is the move, and
//! the cursor applies it on every move frame until `sMapWalkStepTimer` runs out
//! (9 frames for the first step, then 8 — or 4/5 when the move byte's bit 4 is
//! set), then reads the next byte; a `$00` ends the walk and the byte after it is
//! the node the walk arrives at.
//!
//! Move bytes are direction bits (`$01` R, `$02` L, `$04` U, `$08` D, and
//! combinations) applied in the ROM's order — Up/Down first, then Right/Left, and
//! within a pair the first bit wins — with the cursor bounded to the screen.
//! The bounds are the ROM's own comparisons and they are *not* symmetric: Right
//! and Down stop exactly at `$94`/`$8A` (`cp`/`adc`), while Left and Up decrement
//! whenever the coordinate is still `>= $08` (`cp $08`/`ccf`/`sbc 0`), so a cursor
//! at `$08` stepping Left or Up lands on `$07` and only then stops. Bits 5-7 only
//! reach the OAM/pose code.
//!
//! ## What is not modelled
//!
//! Everything `$3C000` does to the *screen*: the map camera latch (`$3ED4C`), the
//! tiles and sprites, the window raster, the gfx loads behind `$3E2F7`/`$3E00`,
//! the hub's 18-entry decoration list (`$3CFC8`) and the cursor's OAM. The port
//! keeps `sMapCursorPose` because the ROM computes it from the walk; it just has
//! nothing to draw with. The blocking screen transition `$3E2F7` (40 iterations
//! of a scroll) is art too, but its *effects* — the new node, the hub screen, the
//! move byte, state `$0C` — are modelled.
//!
//! ROM logic that is not ported yet is reported as [`MapEntry::unported`] rather
//! than silently skipped: the node flag helpers (`$3C2xx`), the A/START node
//! actions behind `$3C60B`, and the hub's own cursor effects (`$3CA31`-`$3CAB1`).

use crate::map::{map_enterable, MapNodes, MapWalk, WalkDir};

/// Input for one map frame: the two joypad bytes and the `$FF04` divider the ROM
/// XORs into its two pseudo-random latches.
///
/// `keys_held` is `hKeysHeld` (`$FF80`, directions in bits 4-7) and `keys_pressed`
/// is `hKeysPressed` (`$FF81`, the same direction bits plus A in bit 0 and START
/// in bit 3). The gate reads the *held* byte for directions and the *pressed*
/// byte for A/START — in this ROM `$FF80` is held and `$FF81` pressed, which is
/// the opposite of the usual convention.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapInput {
    pub keys_held: u8,
    pub keys_pressed: u8,
    /// `$FF04` — a free-running divider, read by `$3C13E` as the map's random
    /// source. Nothing mechanical reads the two latches it feeds, so any value
    /// keeps the port faithful; it is an argument so a caller can be exact.
    pub div: u8,
}

/// The ROM's writes outside the map's own RAM block, for one frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapEntry {
    /// `sCurLevel` (`$A269`) — set only by a level entry.
    pub level: Option<u8>,
    /// `sLevelProgressFlags` (`$A27D`). The entry clears it and then copies the
    /// node's `sLevelProgress` byte into it.
    pub progress_flags: u8,
    /// `sGameState` (`$FF9B`): `$16` on a level entry, `$0C` on a node
    /// transition, `0` when the map just ticked on.
    pub state: u8,
    /// `sSfxRequest0` (`$A460`).
    pub sfx: Option<u8>,
    /// `sMusicRequest` (`$A468`).
    pub music: Option<u8>,
    /// A node screen transition ran (`$3E2F7`'s blocking walk-out/walk-in), with
    /// its direction argument.
    pub transition: Option<u8>,
    /// `$2934` ran: the save image was written.
    pub saved: bool,
    /// A ROM path this frame took that the port does not model, as a stable id.
    pub unported: Option<&'static str>,
}

impl MapEntry {
    /// Keep `self`'s fields but record the unported id, if there is one and none
    /// is recorded yet.
    fn note(&mut self, id: Option<&'static str>) {
        if self.unported.is_none() {
            self.unported = id;
        }
    }
}

/// The overworld's live state: the `$A840` block (the save image's first 64 bytes)
/// plus the map's own screen/work bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSim {
    /// `sMapScreenId` (`$A68B`): 0 hub, 1-9 zones, `$0A`+ castle/ending.
    pub screen_id: u8,
    /// `sMapNode` (`$A840`).
    pub node: u8,
    /// `sMapCursorX` / `sMapCursorY` (`$A842`/`$A843`).
    pub cursor_x: u8,
    pub cursor_y: u8,
    /// `sMapCursorMove` (`$A844`): the current move byte. Bit 5 is the OAM bump
    /// offset, bit 4 the fast step, bits 0-3 the direction.
    pub cursor_move: u8,
    /// `sMapCursorWalking` (`$A690`) — nonzero while a stream is being walked.
    pub cursor_walking: u8,
    /// `sMapWalkPtrLo`/`Hi` (`$A692`/`$A693`): the stream cursor, a bank-24
    /// window address.
    pub walk_ptr: u16,
    /// `sMapWalkStepTimer` (`$A694`).
    pub walk_step_timer: u8,
    /// `sMapCursorPose` (`$A695`): 0-3 Left, 4-7 Right, 8-11 Up, 12-15 Down —
    /// and the idle pose lands in Down's block.
    pub cursor_pose: u8,
    /// `sMapCursorWalkFrame` (`$A68C`).
    pub walk_frame: u8,
    /// `sMapCursorAnim` (`$A878`) — the move-frame gate. Part of the save image,
    /// so the ROM's own value comes from the last transition (`$92`) or map load
    /// (`0`); see the module docs.
    pub cursor_anim: u8,
    /// `sLevelProgress` (`$A848`), 42 bytes — the map's copy. The save record's
    /// first 64 bytes carry this array, so [`Self::from_player`] /
    /// [`Self::to_player`] bridge it to `PlayerSim`'s image.
    pub level_progress: [u8; 42],
    /// `sMapScrollOnlyLatch` (`$A7A0`): `$FF` while B is held on the hub.
    pub scroll_only_latch: u8,
    /// `sMapRockAnimTimer` (`$A789`): the rock/gate animation countdown, set to 1
    /// by the `$3C4E7` secret-route arm.
    pub rock_anim_timer: u8,
    /// `sMapSpecialNodeLatch` (`$A7B8`) — written by `$3C201`, read by nothing.
    pub special_node_latch: u8,
    /// `sMapKeysXor` (`$A6B2`) — a census-only byte with no reader in the ROM.
    pub keys_xor: u8,
    /// `sMapFrameCounter` (`$A69A`).
    pub frame_counter: u8,
    /// `sMapTickTimer` (`$A697`) — 40-frame countdown.
    pub tick_timer: u8,
    /// `sMapRandLatch` (`$A696`).
    pub rand_latch: u8,
    /// `$2934` was called: the caller must persist the save image and clear it.
    pub pending_save: bool,
}

impl Default for MapSim {
    fn default() -> Self {
        Self::new()
    }
}

impl MapSim {
    /// A map that has just loaded: no walk, tick timer at its reload value, the
    /// anim byte at `0` (what `$3EA9A` and the hub loader write — the ROM's `$92`
    /// comes from the walk-out transition `$3E3BA`, a screen effect).
    pub fn new() -> Self {
        Self {
            screen_id: 0,
            node: 0,
            cursor_x: 0,
            cursor_y: 0,
            cursor_move: 0,
            cursor_walking: 0,
            walk_ptr: 0,
            walk_step_timer: 0,
            cursor_pose: 0,
            walk_frame: 0,
            cursor_anim: 0,
            level_progress: [0; 42],
            scroll_only_latch: 0,
            rock_anim_timer: 0,
            special_node_latch: 0,
            keys_xor: 0,
            frame_counter: 0,
            tick_timer: 40,
            rand_latch: 0,
            pending_save: false,
        }
    }

    /// Seed the map's persistent block from the player's save image.
    ///
    /// `PlayerSim` holds `sMapNode` and `sLevelProgress` because the `$14`
    /// hand-off and the save record need them; the map is where they *live*. Both
    /// structs carry a copy and these two functions are the bridge between them.
    pub fn from_player(sim: &crate::player_sim::PlayerSim) -> Self {
        let mut me = Self::new();
        me.node = sim.map_node;
        let n = me.level_progress.len().min(sim.level_progress.len());
        me.level_progress[..n].copy_from_slice(&sim.level_progress[..n]);
        me
    }

    /// Write the map's node and progress bytes back onto the player's save image.
    pub fn to_player(&self, sim: &mut crate::player_sim::PlayerSim) {
        sim.map_node = self.node;
        let n = self.level_progress.len().min(sim.level_progress.len());
        sim.level_progress[..n].copy_from_slice(&self.level_progress[..n]);
    }

    /// `$3E1D7` at the cursor's current node, in the ROM's polarity (see
    /// [`map_enterable`]): true means the node's own action runs and the cursor
    /// does not walk.
    pub fn enterable(&self, nodes: &MapNodes, keys_held: u8) -> bool {
        map_enterable(
            self.screen_id,
            self.node,
            nodes,
            &self.level_progress,
            keys_held,
        )
    }

    /// `sMapCursorAnim`'s `rrca` gate: the byte rotates right and the frame is a
    /// move frame iff the bit rotated out was clear.
    fn move_frame(&mut self) -> bool {
        let out = self.cursor_anim & 1 != 0;
        self.cursor_anim = self.cursor_anim.rotate_right(1);
        !out
    }

    /// `$3C13E`: the per-frame bookkeeping — the frame counter, the 40-frame tick
    /// and its two `$FF04`-seeded latches. `$3C13E` reads `$FF00+c` with `c = 4`,
    /// i.e. the divider, not the joypad, which is why nothing mechanical reads
    /// these bytes.
    fn bookkeeping(&mut self, div: u8) {
        self.keys_xor ^= div;
        self.frame_counter = self.frame_counter.wrapping_add(1);
        self.tick_timer = self.tick_timer.wrapping_sub(1);
        if self.tick_timer == 0 {
            self.tick_timer = 40;
            self.rand_latch ^= div;
        }
    }

    /// One map frame: `$3C000` for the frame's screen, then `$3C169`.
    pub fn tick(&mut self, input: MapInput, nodes: &MapNodes, walk: &MapWalk) -> MapEntry {
        let mut out = MapEntry::default();
        if self.screen_id == 0 {
            self.cursor_chain(input, nodes, walk, &mut out);
            self.bookkeeping(input.div);
        } else {
            self.bookkeeping(input.div);
            // The screen routine itself ($3D8E6 and friends) is tiles, sprites and
            // the camera latch: art, see the module docs.
            self.cursor_chain(input, nodes, walk, &mut out);
        }
        self.enter_check(input, nodes, &mut out);
        out
    }

    /// `$3C38C`'s tail: the cursor, `$3C4CC` onward.
    fn cursor_chain(
        &mut self,
        input: MapInput,
        nodes: &MapNodes,
        walk: &MapWalk,
        out: &mut MapEntry,
    ) {
        if self.cursor_walking != 0 {
            self.walk_step(walk);
            return;
        }
        // `$3C4D3`: 0 from the gate means the node's own action runs.
        if self.enterable(nodes, input.keys_held) {
            self.node_action(input, out);
            return;
        }
        // `$3C4E7`: the two nodes with a hardcoded secret stream — A pressed with
        // nothing else, on the node, with the gate saying "walk".
        if (self.node == 0x0C || self.node == 0x1E)
            && input.keys_pressed & 0xF6 == 0
            && input.keys_pressed & 0x09 != 0
        {
            self.rock_anim_timer = 1;
            out.music = Some(25);
            // $3C4E7 banks bc = $5BF6 in and jumps into the walk start.
            self.start_walk(0x5BF6, walk);
            return;
        }
        if self.screen_id == 0 {
            // `$3C5B2`: B (bit 1 of the held byte) turns the hub into a camera
            // pan — the cursor stays put and the four direction bits pan.
            if input.keys_held & 0x02 != 0 {
                self.scroll_only_latch = 0xFF;
                out.unported = Some("map:$3C5B2 hub B pan ($3CA37/$3CA63/$3CA8A/$3CAB1)");
                return;
            }
            // `$3C518`: the frame after B is released, re-centre the screen on the
            // cursor's node. $3EEA4/$3ED4C/$3F156 are the screen reload (art) and
            // $3EA9A clears the anim byte and asks for music 6.
            if self.scroll_only_latch != 0 {
                self.cursor_anim = 0;
                out.music = Some(6);
            }
        }
        self.scroll_only_latch = 0;
        let pressed = input.keys_pressed & 0xF0;
        if pressed == 0 {
            self.node_action(input, out);
            return;
        }
        // `$3C52B`: the direction decides the entry — Left, then Up, then Down,
        // then Right as the fallback — and $FFFF in that slot means "no path", in
        // which case the ROM falls into the same node action as no input at all.
        let d = pressed >> 4;
        let dir = if d & 0x02 != 0 {
            WalkDir::Left
        } else if d & 0x04 != 0 {
            WalkDir::Up
        } else if d & 0x08 != 0 {
            WalkDir::Down
        } else {
            WalkDir::Right
        };
        match walk.pointer(self.node, dir) {
            Some(ptr) if ptr != MapWalk::NO_PATH => self.start_walk(ptr, walk),
            _ => self.node_action(input, out),
        }
    }

    /// `$3C588`: begin a walk on the stream `ptr` (a bank-24 window address).
    fn start_walk(&mut self, ptr: u16, walk: &MapWalk) {
        let Some(first) = walk.byte_at(ptr) else {
            // The blob covers every pointer the real graph holds, so this can only
            // happen with a truncated blob; leaving the cursor still is the one
            // safe answer.
            return;
        };
        self.walk_ptr = ptr;
        self.cursor_move = first;
        self.cursor_walking = 0xFF;
        self.walk_step_timer = if first & 0xF0 == 0x10 { 5 } else { 9 };
        self.walk_step(walk);
    }

    /// `$3C7EC` onward: one frame of an in-progress walk — the anim gate, the step
    /// timer, the stream advance, then the pose and the one-pixel move.
    fn walk_step(&mut self, walk: &MapWalk) {
        if !self.move_frame() {
            return;
        }
        self.walk_step_timer = self.walk_step_timer.wrapping_sub(1);
        if self.walk_step_timer == 0 {
            self.walk_step_timer = 8;
            self.walk_ptr = self.walk_ptr.wrapping_add(1);
            let b = walk.byte_at(self.walk_ptr).unwrap_or(0);
            if b == 0 {
                // The end of the stream: the next byte is the node we arrived at.
                self.walk_ptr = self.walk_ptr.wrapping_add(1);
                self.cursor_walking = 0;
                self.node = walk.byte_at(self.walk_ptr).unwrap_or(self.node);
                return;
            }
            self.cursor_move = b;
            if b & 0x10 != 0 {
                self.walk_step_timer = 4;
            }
        }
        self.walk_frame = self.walk_frame.wrapping_add(1);
        let m = self.cursor_move;
        if m & 0x0F == 0 {
            // A move byte with no direction is a pause step: the ROM still bumps
            // the frame and lands on the idle pose.
            self.cursor_pose = ((self.walk_frame & 0x18) >> 3) + 12;
            return;
        }
        // Up/Down first — if bit 2 is set bit 3 is never tested — then Right
        // before Left. The pose blocks are 4 wide: L 0-3, R 4-7, U 8-11, D 12-15.
        if m & 0x04 != 0 {
            self.cursor_pose = ((self.walk_frame & 0x0C) >> 2) + 8;
            if self.cursor_y >= 0x08 {
                self.cursor_y -= 1;
            }
        } else if m & 0x08 != 0 {
            self.cursor_pose = ((self.walk_frame & 0x0C) >> 2) + 12;
            if self.cursor_y < 0x8A {
                self.cursor_y += 1;
            }
        }
        if m & 0x01 != 0 {
            if self.walk_frame & 0x0C == 0x0C {
                self.walk_frame = 0;
            }
            self.cursor_pose = ((self.walk_frame & 0x0C) >> 2) + 4;
            if self.cursor_x < 0x94 {
                self.cursor_x += 1;
            }
        } else if m & 0x02 != 0 {
            if self.walk_frame & 0x0C == 0x0C {
                self.walk_frame = 0;
            }
            self.cursor_pose = (self.walk_frame & 0x0C) >> 2;
            if self.cursor_x >= 0x08 {
                self.cursor_x -= 1;
            }
        }
    }

    /// `$3C5DA` — the node's own action, reached when the cursor is not walking.
    ///
    /// `$3C5F4` needs A or START *pressed* to run the node's entry action, and
    /// only on the hub or screen 8; everything else falls to `$3C66C`. On the hub
    /// that is the idle path (`$3C7D9`); on a zone screen it is the door cluster
    /// (`$3C6B0`-`$3C7A3`), which is what turns a zone screen back into the hub.
    fn node_action(&mut self, input: MapInput, out: &mut MapEntry) {
        // `$3C5DA`: nodes $15 and $0D run the same flag helper, which the port has
        // not modelled yet.
        if self.node == 0x15 || self.node == 0x0D {
            out.note(Some("map:$3C2B2 node flag helper"));
        }
        if self.node == 0x0D && self.rock_anim_timer != 0 {
            out.note(Some("map:$3C60B rock/gate node action"));
            return;
        }
        if input.keys_pressed & 0x09 != 0 && (self.screen_id == 0 || self.screen_id == 8) {
            // `$3C60B`: the A/START node action table — the castle animation
            // ($3D0D7), the zone cutscenes, the ending screens.
            out.note(Some("map:$3C60B A/START node action"));
            return;
        }
        if self.screen_id != 0 && self.door(input.keys_pressed, out) {
            return;
        }
        // `$3C7D9`: idle — the walk state is cleared, the step timer reloads, the
        // frame keeps counting and the pose goes to the idle block.
        self.cursor_walking = 0;
        self.walk_step_timer = 8;
        self.walk_frame = self.walk_frame.wrapping_add(1);
        self.cursor_pose = ((self.walk_frame & 0x18) >> 3) + 12;
    }

    /// The zone screens' door cluster (`$3C66C`-`$3C7A3`): eleven nodes walk the
    /// cursor to another node, put it back on the hub and move the game to state
    /// `$0C`. Returns whether this node was one of them.
    ///
    /// Every one of these nodes has *no* walk path in the direction it tests, so
    /// the door is reached the same way as no input at all: `$3C52B` finds `$FFFF`
    /// in the walk table and falls into `$3C5DA`. That is also why the refused
    /// directions matter — node `$46` tests Up and walks Down, node `$50` tests
    /// Left and walks Up.
    ///
    /// The tests are per node: an exact pressed byte (`$80` Down, `$40` Up, `$20`
    /// Left), a mask (`$5C` takes Right or Down), or no test at all (`$2D`, `$30`,
    /// `$40`, `$4A`). Nodes `$46`/`$4A` and `$23`/`$74` share their tails in the
    /// ROM, which is why two nodes can land on the same destination logic.
    fn door(&mut self, keys_pressed: u8, out: &mut MapEntry) -> bool {
        // node -> (destination, move byte, pressed test, transition direction).
        // The direction is `$3E2F7`'s own argument, which is *not* always the move
        // byte's low nibble: node $30 moves Down but transitions Left.
        let (dest, move_, test, dir): (u8, u8, Option<u8>, u8) = match self.node {
            0x23 => (25, 8, Some(0x80), 8),
            0x28 => (28, 8, Some(0x80), 8),
            0x2D => (121, 8, None, 8),
            0x30 => (122, 8, None, 4),
            0x32 => (27, 0x22, Some(0x20), 2),
            0x40 => (26, 2, None, 2),
            0x46 => (30, 4, Some(0x40), 4),
            0x4A => (101, 4, None, 4),
            0x50 => (29, 2, Some(0x20), 2),
            0x5C => (100, 1, Some(0x90), 1),
            0x74 => (23, 2, Some(0x20), 2),
            _ => return false,
        };
        let ok = match test {
            Some(0x90) => keys_pressed & 0x90 != 0,
            Some(k) => keys_pressed == k,
            None => true,
        };
        if !ok {
            return false;
        }
        // `$3C708` is the odd one out: it arms a walk frame — move $22 and the
        // walking byte — before the transition, then clears the walking byte.
        if self.node == 0x32 {
            self.cursor_move = 0x22;
            self.cursor_walking = 0xFF;
            self.cursor_walking = 0;
        }
        // `$3C7CA`: the cursor's new node, the hub's screen id; then `$3C7BD`:
        // `$3F156`, state $0C, `$3EA9A` (anim 0, music 6), return.
        self.node = dest;
        self.screen_id = 0;
        self.cursor_move = move_;
        self.cursor_anim = 0;
        out.transition = Some(dir);
        out.music = Some(6);
        out.state = 12;
        true
    }

    /// `$3C169` — the A/START check, and the level entry behind it (`$3C1A2`).
    fn enter_check(&mut self, input: MapInput, nodes: &MapNodes, out: &mut MapEntry) {
        if self.cursor_walking != 0 || input.keys_pressed & 0x09 == 0 {
            return;
        }
        if self.scroll_only_latch != 0 {
            return;
        }
        let slot = nodes.progress_slot(self.node);
        // The ROM reads `sLevelProgress[$650E[node]]` unconditionally, so a node
        // whose slot byte is $FE reads past the array; the port calls that "not
        // cleared" (the byte it would land on is zero in the initialised save).
        let progress = slot
            .and_then(|s| self.level_progress.get(s as usize).copied())
            .unwrap_or(0);
        // An already-cleared level is re-enterable — except at the six castle
        // slots, which refuse outright.
        if progress & 0x80 != 0 && matches!(slot, Some(0x05 | 0x09 | 0x0E | 0x17 | 0x1D | 0x11)) {
            return;
        }
        out.progress_flags = 0;
        match nodes.level_byte(self.node) {
            // `$3C201`: a node without a level — `$4218` is $FE. A alone arms a
            // latch nothing reads; from node $6E up it also clicks.
            Some(0xFE) => {
                if input.keys_pressed & 0x01 != 0 {
                    self.special_node_latch = 16;
                    if self.node >= 0x6E {
                        out.sfx = Some(2);
                    }
                }
            }
            // `$3C1A2`: the level entry. `$FF` passes through — the gate rejects
            // only $FE.
            Some(level) => {
                out.level = Some(level);
                out.progress_flags = progress;
                out.music = Some(255);
                // $2AAA is _UpdateSound (bank 4) — the requested music starts
                // here; $3E9FB is the map's exit and $2934 the save writer. The
                // state goes straight to $16 and the game's own state machine
                // takes it from there (fade, then the level load).
                out.sfx = Some(22);
                out.saved = true;
                out.state = 22;
                self.pending_save = true;
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapNodes;

    /// The ROM's own bytes, embedded so the tests need no file.
    ///
    /// `NODE1_ROW` is the walk table's node-1 entry (flat `$61642`): R `$5AA1`,
    /// L `$5AAE`, U `$5AB8`, no Down. `NODE1_R_STREAM` is what `$5AA1` holds —
    /// eleven moves (R, R, D, D, D, R×6), the `$00` terminator, and the arrival
    /// node `$13` = 19. Both are read straight out of `baserom.gb`.
    const NODE1_ROW_HEX: &str = "a1 5a ae 5a b8 5a ff ff";
    const NODE1_R_STREAM_HEX: &str = "01 01 08 08 08 01 01 01 01 01 01 00 13";
    /// Node 1's Right pointer as a bank-24 window address.
    const NODE1_R: u16 = 0x5AA1;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).unwrap())
            .collect()
    }

    /// A blob holding the real table row and the real stream, at the offsets the
    /// ROM addresses them at. Every other entry is `$FFFF` — no path — the way the
    /// ROM's own empty entries are, which is what sends a direction press into the
    /// node's own action (`$3C52B`'s `jr z, $3C5DA`).
    fn walk_blob() -> MapWalk {
        let mut blob = vec![0xFFu8; MapWalk::TABLE_LEN];
        // Node 1 at its real offset in the table.
        blob[8..16].copy_from_slice(&hex(NODE1_ROW_HEX));
        // The stream, at (its window pointer - the table's own window address).
        let at = (NODE1_R - 0x563A) as usize;
        blob.resize(at + 13, 0);
        blob[at..at + 13].copy_from_slice(&hex(NODE1_R_STREAM_HEX));
        MapWalk::from_bytes(&blob).expect("table-sized blob")
    }

    /// The node tables as `sml2-extract` writes them, trimmed to what the map tick
    /// needs: node 1 has level 0 and slot 0, node 2 has level 1 and slot 1, node 3
    /// has no level ($FE) and no slot, node $20 is a castle slot (5) and node $6E a
    /// special node. The key mask table is the ROM's (`$6591`): slot 0 `$00`,
    /// slot 1 `$10`, slot 5 `$40`.
    fn nodes() -> MapNodes {
        use crate::map::MapNodes as N;
        let mut b = vec![0xFEu8; N::BLOB_LEN];
        b[0] = 0x00; // $4218[0]: node 0 is level 1
        b[1] = 0x00; // node 1: level 1
        b[2] = 0x01; // node 2: level 2
        b[0x20] = 0x04; // castle node, level 5
        b[0x6E] = 0x1F; // a level at a special node
        b[N::NODES] = 0x00;
        b[N::NODES + 1] = 0x00;
        b[N::NODES + 2] = 0x01;
        b[N::NODES + 0x20] = 0x05;
        b[N::NODES + 0x6E] = 0x00;
        b[2 * N::NODES] = 0x00; // slot 0: no direction opens it
        b[2 * N::NODES + 1] = 0x10; // slot 1: Right
        b[2 * N::NODES + 5] = 0x40; // slot 5: Up, the castle
        N::from_bytes(&b).expect("sized blob")
    }

    /// The gate's own test sites. On the hub the ROM's `$3E1D7` answers 255 before
    /// it looks at anything, so the cursor always walks; on a zone screen the gate
    /// needs a level, the held direction and an uncleared stage.
    #[test]
    fn the_gate_is_hub_off_and_zone_conditional() {
        let n = nodes();
        let mut map = MapSim::new();
        map.node = 2;
        assert!(
            !map.enterable(&n, 0x10),
            "screen 0 always walks, whatever the node offers"
        );
        map.screen_id = 1;
        assert!(map.enterable(&n, 0x10), "node 2, Right held, uncleared");
        assert!(!map.enterable(&n, 0x40), "node 2's mask is Right only");
        assert!(!map.enterable(&n, 0x00), "no direction held");
        map.level_progress[1] = 0x80;
        assert!(
            !map.enterable(&n, 0x10),
            "a cleared level is not enterable through this gate — the cursor walks"
        );
        map.node = 3;
        assert!(
            !map.enterable(&n, 0x10),
            "node 3's $4218 is $FE: the gate rejects it even with a direction"
        );
    }

    /// A walk follows the stream and lands on the node after the terminator, with
    /// the cursor moved by the stream's own moves: node 1's Right stream is
    /// R, R, D, D, D, R×6, so from (8, 8) the cursor ends at (72, 32).
    ///
    /// The arithmetic, from the ROM's `$3C588`/`$3C7F8`: the first step lasts 9
    /// move frames and every later one 8, and the new byte is applied on the frame
    /// the pointer advances. So move frames 1-16 are Right (x = 8 + 16), 17-40 are
    /// Down (y = 8 + 24) and 41-88 are Right (x = 24 + 48 = 72); move frame 89
    /// reads the terminator.
    #[test]
    fn a_walk_follows_its_stream_to_the_arrival_node() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.cursor_anim = 0x92; // the ROM's walk-out value
        map.cursor_x = 8;
        map.cursor_y = 8;
        map.node = 1;
        map.screen_id = 1;

        let input = MapInput {
            keys_held: 0x10, // the gate reads the *held* byte for directions...
            keys_pressed: 0x10, // ...and the walk reads the pressed one
            ..Default::default()
        };
        let out = map.tick(input, &n, &walk);
        assert_eq!(map.cursor_walking, 0xFF, "the walk started");
        assert_eq!(map.cursor_move, 0x01, "the stream's first move byte");
        // The walk-start frame *is* a move frame ($3C588 jumps into $3C7EC), so
        // the 9 it loads is already one down by the time the frame ends.
        assert_eq!(map.walk_step_timer, 8, "9 frames when bit 4 is clear, less this one");
        assert_eq!(map.walk_ptr, NODE1_R);
        assert_eq!(map.cursor_x, 9, "and the frame moves the cursor too");
        assert_eq!(out.level, None, "A was not pressed");

        // Release the key: the walk keeps going on its own.
        let idle = MapInput::default();
        let mut ticks = 1;
        while map.cursor_walking != 0 && ticks < 1000 {
            map.tick(idle, &n, &walk);
            ticks += 1;
        }
        assert_eq!(map.cursor_walking, 0, "the stream ended");
        assert_eq!(map.node, 19, "the byte after the $00 terminator");
        assert_eq!(map.cursor_x, 72, "16 move frames of R, then 48 more");
        assert_eq!(map.cursor_y, 32, "24 move frames of D");
        // 89 move frames at the $92 cadence — five of every eight frames, at cycle
        // positions 1, 3, 4, 6 and 7 — is 17 whole cycles (136 frames) plus
        // positions 1, 3, 4 and 6 of the next one: 142.
        assert_eq!(ticks, 142, "the anim byte's cadence, pinned");
    }

    /// The anim byte is the frame gate, not a timer: with `0` every frame moves,
    /// with `$92` five of every eight do. Both are the ROM's own values (`$3EA9A`
    /// writes `0`, `$3E3BA` writes `$92`).
    #[test]
    fn the_anim_byte_decides_how_many_frames_move() {
        fn moves_in_8(anim: u8) -> usize {
            let mut map = MapSim::new();
            map.cursor_anim = anim;
            (0..8).filter(|_| map.move_frame()).count()
        }
        assert_eq!(moves_in_8(0x00), 8, "no skips");
        assert_eq!(moves_in_8(0x92), 5, "the ROM's $92 cycle");
        // The cycle's period is 8 frames for $92: after 8 frames the byte is back.
        let mut map = MapSim::new();
        map.cursor_anim = 0x92;
        for _ in 0..8 {
            map.move_frame();
        }
        assert_eq!(map.cursor_anim, 0x92, "8 rotations come back round");
    }

    /// Each direction's pose block, and the cursor's bounds. The ROM's own
    /// comparisons are asymmetric, so these pin the asymmetry rather than a tidy
    /// box: Right and Down stop *at* `$94`/`$8A`, Left and Up step from `$08` to
    /// `$07` and stop below that.
    #[test]
    fn the_cursor_bounds_to_the_screen_and_takes_its_pose_block() {
        fn step(dir: u8, x: u8, y: u8, frame: u8) -> MapSim {
            let mut map = MapSim::new();
            map.cursor_x = x;
            map.cursor_y = y;
            map.cursor_move = dir;
            map.walk_frame = frame;
            map.cursor_walking = 0xFF;
            map.walk_step_timer = 0xFF; // never advances the stream
            map.cursor_anim = 0; // every frame moves
            map.walk_step(&walk_blob());
            map
        }
        // Left from $08: one step too far — the ROM's `cp $08`/`ccf`/`sbc 0`
        // decrements while the coordinate is still >= $08.
        let m = step(0x02, 0x08, 0x40, 4);
        assert_eq!((m.cursor_x, m.cursor_pose), (0x07, 1), "L steps 8 -> 7");
        // And from $07 it stays.
        let m = step(0x02, 0x07, 0x40, 4);
        assert_eq!(m.cursor_x, 0x07, "L stops below 8");
        // From 9 it lands on the edge, pose 0-3.
        let m = step(0x02, 0x09, 0x40, 4);
        assert_eq!((m.cursor_x, m.cursor_pose), (0x08, 1));
        // Right stops exactly at $94.
        let m = step(0x01, 0x94, 0x40, 4);
        assert_eq!((m.cursor_x, m.cursor_pose), (0x94, 5), "R clamps at $94");
        let m = step(0x01, 0x93, 0x40, 4);
        assert_eq!(m.cursor_x, 0x94);
        // Up mirrors Left: 8 -> 7, then it stops.
        let m = step(0x04, 0x40, 0x08, 4);
        assert_eq!((m.cursor_y, m.cursor_pose), (0x07, 9), "U steps 8 -> 7");
        let m = step(0x04, 0x40, 0x07, 4);
        assert_eq!(m.cursor_y, 0x07, "U stops below 8");
        // Down stops exactly at $8A.
        let m = step(0x08, 0x40, 0x8A, 4);
        assert_eq!((m.cursor_y, m.cursor_pose), (0x8A, 13), "D clamps at $8A");
        // A move with both Up and Down is Up: the ROM never tests bit 3 when bit 2
        // is set.
        let m = step(0x0C, 0x40, 0x40, 4);
        assert_eq!((m.cursor_y, m.cursor_pose), (0x3F, 9), "$0C is Up");
        // A move with R and L is Right, and its frame counter wraps at $0C.
        let m = step(0x03, 0x40, 0x40, 0x0C);
        assert_eq!(
            (m.cursor_x, m.walk_frame, m.cursor_pose),
            (0x41, 0, 4),
            "$03 is Right and $0C wraps the walk frame"
        );
        // A move byte with no direction is a pause step: it still counts frames
        // and goes to the idle block.
        let m = step(0xA0, 0x40, 0x40, 8);
        assert_eq!(
            (m.cursor_x, m.cursor_y, m.cursor_pose),
            (0x40, 0x40, 13),
            "$A0 moves nothing, idle pose"
        );
    }

    /// `$3C4E7`: nodes `$0C` and `$1E` walk a *hardcoded* stream at `$5BF6` when A
    /// is pressed with every other bit clear, and set the rock animation counter
    /// and music 25 on the way. The stream is the ROM's: `$A0 $A0 $01 $01 $E0`,
    /// then `$41` until the `$00`, and it arrives at node `$0D`.
    #[test]
    fn the_secret_route_arm_starts_the_hardcoded_stream() {
        let n = nodes();
        // The real bytes at $5BF6, at the real offset in the blob.
        const SECRET_STREAM: &str = "a0 a0 01 01 e0 41 41 41 41 41 41 41 41 41 00 0d";
        let mut blob = vec![0xFFu8; MapWalk::TABLE_LEN];
        let at = (0x5BF6 - 0x563A) as usize;
        blob.resize(at + 16, 0);
        blob[at..at + 16].copy_from_slice(&hex(SECRET_STREAM));
        let walk = MapWalk::from_bytes(&blob).unwrap();

        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x0C;
        let out = map.tick(
            MapInput {
                keys_pressed: 0x09, // A and START together: $F6 is clear
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(map.rock_anim_timer, 1, "$3C4E7's rock counter");
        assert_eq!(out.music, Some(25));
        assert_eq!(map.cursor_walking, 0xFF, "the stream walk started");
        assert_eq!(map.walk_ptr, 0x5BF6);
        assert_eq!(map.cursor_move, 0xA0, "the hardcoded stream's first byte");
        assert_eq!(map.walk_step_timer, 8, "$A0 has no bit 4: 9, less the start frame");
        assert_eq!(out.level, None, "nothing was entered");

        // Walking it out lands on the stream's arrival node, $0D.
        let idle = MapInput::default();
        let mut guard = 0;
        while map.cursor_walking != 0 && guard < 2000 {
            map.tick(idle, &n, &walk);
            guard += 1;
        }
        assert_eq!(map.cursor_walking, 0, "the secret stream ended");
        assert_eq!(map.node, 0x0D, "the byte after its $00");

        // Any other pressed bit and the arm does not fire.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x0C;
        map.tick(
            MapInput {
                keys_pressed: 0x19, // A plus a direction
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(map.rock_anim_timer, 0);
        assert_eq!(map.cursor_walking, 0);
    }

    /// `$3C169`/`$3C1A2`: A or START on a node with a level hands back the level,
    /// the node's progress byte and state `$16`, and writes the save.
    #[test]
    fn a_level_entry_hands_back_the_level_and_moves_the_state() {
        let n = nodes();
        let walk = walk_blob();
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 2;
        map.level_progress[1] = 0x08; // the secret-route marker, bit 3 only

        let out = map.tick(
            MapInput {
                keys_pressed: 0x01, // A
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(out.level, Some(1), "$4218[2] = 1 = level 2");
        assert_eq!(out.progress_flags, 0x08, "the node's own progress byte");
        assert_eq!(out.state, 22, "sGameState = $16");
        assert_eq!(out.sfx, Some(22));
        assert_eq!(out.music, Some(255));
        assert!(out.saved, "$2934 runs on the way into a level");
        assert!(map.pending_save);
    }

    /// A node whose `$4218` is `$FE` has no level, so A arms `$3C201`'s latch
    /// instead — and clicks only from node `$6E` up.
    #[test]
    fn a_nodeless_node_arms_the_special_latch() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 3; // $4218 = $FE
        let out = map.tick(
            MapInput {
                keys_pressed: 0x01,
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(map.special_node_latch, 16);
        assert_eq!(out.level, None, "no level was entered");
        assert_eq!(out.state, 0, "the map just ticked on");
        assert_eq!(out.sfx, None, "node 3 is below $6E: no click");
        assert_eq!(out.progress_flags, 0, "$3C1A2 cleared the flags first");

        // START alone does not arm it: `$3C201` wants A (bit 0).
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 3;
        map.tick(
            MapInput {
                keys_pressed: 0x08,
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(map.special_node_latch, 0, "START is not A");
    }

    /// The six castle slots refuse re-entry once cleared; every other level
    /// re-enters. `$3C169` compares the *slot*, not the level.
    #[test]
    fn a_cleared_castle_slot_is_not_entered_again() {
        let (n, walk) = (nodes(), walk_blob());
        let press_a = MapInput {
            keys_pressed: 0x01,
            ..Default::default()
        };
        // Node $20's slot is 5 — one of the six.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x20;
        map.level_progress[5] = 0x80;
        let out = map.tick(press_a, &n, &walk);
        assert_eq!(out.level, None, "the castle slot refuses");
        assert!(!out.saved);

        // The same node uncleared enters: $4218[$20] is 4, i.e. level 5.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x20;
        let out = map.tick(press_a, &n, &walk);
        assert_eq!(out.level, Some(4));
        assert_eq!(out.progress_flags, 0);

        // Node 2's slot is 1, not one of the six: a cleared level re-enters.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 2;
        map.level_progress[1] = 0x80;
        let out = map.tick(press_a, &n, &walk);
        assert_eq!(out.level, Some(1), "a cleared level still re-enters");
        assert_eq!(out.progress_flags, 0x80, "and keeps its cleared bit");
    }

    /// `$3C5F4`: on the hub (and screen 8) A runs the node's own action, which is
    /// what loads a level from a zone screen's entry node — and the port reports
    /// that path as unported rather than pretending it ran.
    #[test]
    fn the_a_start_node_action_is_reported_as_unported() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0;
        let out = map.tick(
            MapInput {
                keys_pressed: 0x01,
                keys_held: 0x10,
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(
            out.unported,
            Some("map:$3C60B A/START node action"),
            "the hub's node action table is not ported"
        );

        // Without A, the hub falls to the idle path instead.
        let mut map = MapSim::new();
        map.node = 0;
        let out = map.tick(MapInput::default(), &n, &walk);
        assert_eq!(out.unported, None);
        assert_eq!(map.cursor_walking, 0);
        assert_eq!(map.walk_step_timer, 8, "$3C7D9 reloads the step timer");
        assert_eq!(map.walk_frame, 1, "and counts the frame");
        assert_eq!(map.cursor_pose, 12, "the idle block, 12-15");
    }

    /// The zone screens' door cluster: walk into one and the cursor lands on the
    /// hub at another node, in state `$0C`. Node `$23` wants Down exactly; node
    /// `$2D` has no key test at all.
    #[test]
    fn a_zone_door_returns_the_cursor_to_the_hub() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x23;
        map.cursor_walking = 0; // no walk in progress
        let out = map.tick(
            MapInput {
                keys_pressed: 0x80, // Down
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(map.node, 25, "the door's destination");
        assert_eq!(map.screen_id, 0, "back to the hub");
        assert_eq!(map.cursor_move, 8);
        assert_eq!(out.state, 12, "sGameState = $0C");
        assert_eq!(out.transition, Some(8), "the transition direction");
        assert_eq!(out.level, None);

        // Node $23 with any other key does nothing.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x23;
        map.tick(
            MapInput {
                keys_pressed: 0x20,
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(map.node, 0x23, "Left is not this door's key");
        assert_eq!(map.cursor_pose, 12, "the idle path instead");

        // Node $2D has no test: any frame at that node takes the door, which is
        // why the ROM's own $3C6E4 has no `cp`.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x2D;
        let out = map.tick(MapInput::default(), &n, &walk);
        assert_eq!(map.node, 121);
        assert_eq!(map.screen_id, 0);
        assert_eq!(out.transition, Some(8));
        assert_eq!(map.cursor_move, 8);
    }

    /// The hub's B button: the cursor stays put and the pan latch is set and held
    /// only while B is down.
    #[test]
    fn the_hub_b_button_sets_the_scroll_only_latch() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0;
        let out = map.tick(
            MapInput {
                keys_held: 0x02, // B
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(map.scroll_only_latch, 0xFF);
        assert!(out.unported.is_some(), "the pan itself is not ported");
        assert_eq!(map.cursor_walking, 0, "the cursor does not walk");
        assert_eq!(map.node, 0, "and does not move");

        // The frame B is released, the screen re-centres and the anim byte is
        // cleared; the latch goes with it.
        let out = map.tick(MapInput::default(), &n, &walk);
        assert_eq!(map.scroll_only_latch, 0);
        assert_eq!(out.music, Some(6), "$3EA9A asks for music 6");
        assert_eq!(map.cursor_anim, 0);
    }

    /// A press that is only A, with the cursor walking, is swallowed: `$3C169`
    /// returns while `sMapCursorWalking` is set.
    #[test]
    fn a_walk_in_progress_swallows_the_entry_check() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 2;
        map.cursor_walking = 0xFF;
        map.cursor_anim = 0;
        map.walk_step_timer = 0xFF;
        map.cursor_move = 0x08; // Down, so the stream never ends on this test
        let out = map.tick(
            MapInput {
                keys_pressed: 0x01,
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(out.level, None);
        assert_eq!(out.state, 0);
    }

    /// `$3C13E`: the frame counter always advances, the tick fires every 40 frames
    /// and the random latch only moves with it.
    #[test]
    fn the_bookkeeping_counts_frames_and_ticks_at_40() {
        let mut map = MapSim::new();
        map.tick_timer = 40;
        for i in 1..=39u8 {
            map.bookkeeping(0x55);
            assert_eq!(map.frame_counter, i);
            assert_eq!(map.keys_xor, if i % 2 == 1 { 0x55 } else { 0x00 });
        }
        assert_eq!(map.tick_timer, 1);
        assert_eq!(map.rand_latch, 0, "not yet");
        map.bookkeeping(0x55);
        assert_eq!(map.rand_latch, 0x55, "the 40th frame moves the latch");
        assert_eq!(map.tick_timer, 40, "and reloads");
    }

    /// A truncated graph starts no walk rather than reading past its blob.
    #[test]
    fn a_truncated_walk_blob_refuses_to_start() {
        let n = nodes();
        let walk = MapWalk::from_bytes(&[0u8; MapWalk::TABLE_LEN]).unwrap();
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 1;
        map.tick(
            MapInput {
                keys_pressed: 0x10,
                ..Default::default()
            },
            &n,
            &walk,
        );
        assert_eq!(map.cursor_walking, 0, "no stream, no walk");
        assert_eq!(map.cursor_x, 0);
        assert!(MapWalk::from_bytes(&[0u8; 4]).is_none(), "a short blob");
    }

    /// The map's persistent block is the player's save image; the two bridges
    /// carry node and progress both ways.
    #[test]
    fn the_map_block_bridges_to_the_players_save_image() {
        let mut sim = crate::player_sim::PlayerSim::new();
        sim.map_node = 42;
        sim.level_progress[3] = 0x88;
        let mut map = MapSim::from_player(&sim);
        assert_eq!(map.node, 42);
        assert_eq!(map.level_progress[3], 0x88);
        map.node = 7;
        map.level_progress[3] = 0x08;
        map.to_player(&mut sim);
        assert_eq!(sim.map_node, 7);
        assert_eq!(sim.level_progress[3], 0x08);
    }

    /// The graph's shape, from the ROM: the table covers 128 nodes and an entry
    /// that carries a path is a pair of in-bank pointers. (This is the embedded
    /// blob: the row and the stream are the ROM's bytes, at the ROM's offsets.)
    #[test]
    fn the_walk_table_is_pinned_to_the_roms_bytes() {
        let walk = walk_blob();
        assert_eq!(walk.pointer(1, WalkDir::Right), Some(0x5AA1));
        assert_eq!(walk.pointer(1, WalkDir::Left), Some(0x5AAE));
        assert_eq!(walk.pointer(1, WalkDir::Up), Some(0x5AB8));
        assert_eq!(walk.pointer(1, WalkDir::Down), Some(MapWalk::NO_PATH));
        assert_eq!(
            walk.pointer(0, WalkDir::Right),
            Some(MapWalk::NO_PATH),
            "an empty entry is $FFFF, not zero"
        );
        assert_eq!(walk.pointer(128, WalkDir::Right), None, "past the table");
        assert_eq!(walk.byte_at(0x5AA1), Some(0x01));
        assert_eq!(walk.byte_at(0x5AA0), Some(0x00), "one byte before it is the pad");
        assert_eq!(walk.byte_at(0x4000), None, "outside the extracted span");
    }
}
