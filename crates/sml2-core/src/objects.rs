//! Object engine — the full bank-2 `$8000` flow (`main.asm`), the bank-16
//! `$58000` per-anim handler dispatcher + the walker handlers, and the
//! Mario-vs-object interaction pass (home.asm `$2D7D`).
//!
//! Decoded pieces (ROM offsets; CPU = `$4000 + (ROM − bank base)`):
//! - Windows around the camera (`$8000`): act ±`$60`, scan ±`$70`, alive
//!   ±`$A0`, each min clamped to 0. Scan direction/edge (`$8064`): camera
//!   moved right → dir 1 (scanX max), left → dir −1 (scanX min), unchanged →
//!   dir = the camera low byte with spawnXHi = `$FF` (scan returns early).
//! - Slot loop (`$8131`, 16 slots × `$20` at `$AD00`, state byte +8):
//!   standby (1) culls against the alive X window on odd `$FFF0` frames
//!   (`$826D` → `$8285` removal via `$5B2D7`), promotes into active (2) on
//!   even frames when inside the act window (`$81F7`, which also copies the
//!   parked type `+17` → `+18`); active slots cull against alive X (odd) /
//!   alive Y (even, `$828F` → demote to standby or remove), then run the
//!   update pipeline `$818D`: copy slot → `$FFD0` staging, bank-16 handler,
//!   bank-3 draw (`$C090` screen pos + flip flags), optional flag clear,
//!   hitbox refresh (`$81CF`), copy back.
//! - Bank-16 dispatcher (`$58000`): mario-dead gate, interaction dispatch
//!   (`$85AA`), slot-parity gate vs `sObjectUpdatePhase` (skipped while
//!   `+15 == 0`), `sClearStageFlags = 1`, hit-reaction path (`$FFEB`), then
//!   `anim → handler` through the `$58040` (mode A) / `$59C07` (mode B)
//!   tables. Handlers operate on the staging copy.
//! - Walker (anim 9 → `$58201`): sprite toggle `$5B51A` (bit0 every 4th
//!   `$FFF0`), init `$5B3C6` (face Mario → state 1 left / 2 right, flip
//!   `$FFDD` = 0 / `$20`), main loop `$58215` gated to even `$FFF0`; states
//!   1 walk left / 2 walk right (wall probes `$84EA`/`$843C`, ground
//!   `$8358` → fall states 3/4 on a ledge), 3/4 fall (speed curve `$7488`
//!   stepped by `+16` through `$5B451`).
//! - Squashed variant (anim 10 → `$582A9`): flash 16, phase 32, type 0, then
//!   counts `+16` down (every 2nd frame — slot parity) and removes via
//!   `$5B2D7`, crediting the coin counter (`$5B31D`).
//! - Bobber (anim 12 → `$584E0`): pipe gate; state 0 stages `+12 = $80`
//!   until `$5B4B4`'s 24-px Mario-X distance passes, then type `$44` + twin
//!   32-step timers; states 1/2 rise/sink 1 px/call, state 3 rests 24 frames.
//! - Hover crawler (anim 8 → `$5862E`): even `$FFF0` frames walk (states 1/3
//!   left, 2/4 right, wall turns); odd frames rise through `$5B47A`'s curve
//!   until the top or the 18-px ceiling probe (`$3B7C`) arms the `$5B451`
//!   fall; the ground probe ends it and four landings re-face Mario.
//! - Item-pose family (anims 18/19/20/25 → `$58755`/`$58758`, 22 → `$587BD`,
//!   23 → `$587D7`): item pose (`type $80`, done) until touched (`+23`
//!   bit4) → rise / spin (`$5B545`) / type-from-anim; anim 23 removes via
//!   `$5B2D7` mode 2 on the secret route. Anim 15 (`$586E1`) = type `$85`.
//!   The pickup arm (anim 16 → `$586F5`) is where the `$8611`/`$868A` touch
//!   switch sends a collected item: flash, Y += 2 and `+24` += 2 per call,
//!   remove-mode-2 despawn when `+24` reaches `$20`.
//! - Interaction pass (home.asm `$2D7D`): per slot, box test against the
//!   slot's `+9/+10` screen pos (set by the previous frame's draw) and the
//!   Mario screen pos (`x − camx + 96`, `y − camy + 98`); flags at `+23`
//!   bit0 bumped / bit2 star / bit3 stood-on / bit4 touched. The response
//!   (`$2DC6`) writes the flags and either bounces Mario (`sObjectBounce` →
//!   vvel 48/75, A held → 75), sets `sMarioDead = 15` on harmful side
//!   contact, or (star) defeats the object.
//! - `$85AA` dispatch on `+23`: bit0 stomp (`$85C4`: `+15 = 0`, spin jump →
//!   defeat, type bit1 clear → defeat, else anim+1 + sprite re-derive),
//!   bit1/bit2 defeat (`$85F0`/`$85E2` → `$85FC`), bit4 touched (`$8611`:
//!   bit-7 types `$80`-`$85`/`$89` switch to the anim-16 pickup arm, with
//!   `$85` deriving its sprite before the anim write and `$80`-`$84`/`$89`
//!   after; `$86`-`$88`/`$8A`+ inert; type `$06` carry — not modelled),
//!   bit5 hit-from-below (`$86A0` → the `$86E4` cluster for bit-7 types:
//!   `$8721` holds the type across the re-derive and arms the revive
//!   reaction, `$80` alone runs `$8708`'s pose swap; see
//!   [`ObjectSim::dispatch_below`]).
//!
//! The bit5 flag itself is *written* by `$2F72` — the tail of the block-bump
//! slot alloc (`$5960`), which runs inside the player update and boxes each
//! idle object's staged screen pair against the ceiling probe's bonk point
//! ([`ObjectSim::below_bonk`]). It only runs on a frame whose ceiling bonk
//! allocated a bump slot, so the below-hit arms are reached from a brick
//! bonk, not from a plain touching contact.
//!
//! Verification compares the raw 32 slot bytes per frame, so every byte the
//! engine writes (positions, sprite/flip/type, frame state, phase counter,
//! hitbox, interaction flags) is checked against the live console.

use crate::level::{LevelLayout, SpawnRecord};
use crate::object::ObjectTables;

pub const SLOT_COUNT: usize = 16;
pub const RECORD_LEN: usize = 6;
pub const STREAM_BASE: u16 = 0xAB06;

// --- slot / staging byte offsets (ram.asm `$FFD0-$FFEF` map) ---
pub const B_XHI: usize = 0;
pub const B_XLO: usize = 1;
pub const B_MARKER: usize = 2;
pub const B_YHI: usize = 3;
pub const B_YLO: usize = 4;
pub const B_DEF: usize = 5;
pub const B_RECHI: usize = 6;
pub const B_RECLO: usize = 7;
pub const B_STATE: usize = 8;
pub const B_SCRY: usize = 9;
pub const B_SCRX: usize = 10;
pub const B_SPRITE: usize = 11;
pub const B_DEFEAT: usize = 12;
pub const B_FLIP: usize = 13;
pub const B_FLASH: usize = 14;
pub const B_FSTATE: usize = 15;
pub const B_PHASE: usize = 16;
pub const B_PARKED: usize = 17;
pub const B_TYPE: usize = 18;
pub const B_HB_TOP: usize = 19;
pub const B_HB_BOTTOM: usize = 20;
pub const B_HB_LEFT: usize = 21;
pub const B_HB_RIGHT: usize = 22;
pub const B_FLAGS: usize = 23;
pub const B_T24: usize = 24;
pub const B_T25: usize = 25;
pub const B_ANIMFRAME: usize = 26;
pub const B_HITREACT: usize = 27;
pub const B_T28: usize = 28;
pub const B_MASK: usize = 31;

/// `$5B451` fall-speed curve (ROM `$5B488`, 22 entries; `+16` indexes it).
const FALL_SPEED: [u8; 22] = [
    0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x03, 0x02, 0x03, 0x03, 0x03,
    0x02, 0x03, 0x03, 0x02, 0x03, 0x03,
];
/// `$5B481` hit-reaction bounce curve (ROM `$5B4A4`, 16 entries).
const BOUNCE_SPEED: [u8; 16] = [
    0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00,
];
/// `$5B47A` hover curve (ROM `$5B49E`, 22 entries; `+16` indexes it, cap 21)
/// — anim 8's odd-frame rise.
const HOVER_SPEED: [u8; 22] = [
    0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02,
    0x01, 0x01, 0x01, 0x00, 0x00, 0x00,
];
/// `$5B545` spin wobble (ROM `$5B57A`, 4 entries; `+26` indexes it).
const SPIN_WOBBLE: [u8; 4] = [0xFF, 0x01, 0xFF, 0x01];

#[inline]
pub fn x_of(b: &[u8; 32]) -> u16 {
    ((b[B_XHI] as u16) << 8) | b[B_XLO] as u16
}
#[inline]
pub fn set_x(b: &mut [u8; 32], v: u16) {
    b[B_XHI] = (v >> 8) as u8;
    b[B_XLO] = v as u8;
}
#[inline]
pub fn y_of(b: &[u8; 32]) -> u16 {
    ((b[B_YHI] as u16) << 8) | b[B_YLO] as u16
}
#[inline]
pub fn set_y(b: &mut [u8; 32], v: u16) {
    b[B_YHI] = (v >> 8) as u8;
    b[B_YLO] = v as u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlotState {
    #[default]
    Inactive,
    Standby,
    Active,
}

#[derive(Debug, Clone, Default)]
pub struct ObjSlot {
    /// The raw 32 slot bytes — the engine operates on bytes; every field is
    /// an offset (see the `B_*` constants).
    pub b: [u8; 32],
}

impl ObjSlot {
    pub fn x(&self) -> u16 {
        x_of(&self.b)
    }
    pub fn y(&self) -> u16 {
        y_of(&self.b)
    }
    pub fn state(&self) -> SlotState {
        match self.b[B_STATE] {
            0 => SlotState::Inactive,
            1 => SlotState::Standby,
            _ => SlotState::Active,
        }
    }
    pub fn def(&self) -> u8 {
        self.b[B_DEF]
    }
    pub fn sprite(&self) -> u8 {
        self.b[B_SPRITE]
    }
    /// Record byte offset + 2 at spawn time (`0` when never spawned).
    pub fn rec_index(&self) -> u16 {
        ((self.b[B_RECHI] as u16) << 8) | self.b[B_RECLO] as u16
    }
}

/// The `$FFD0-$FFEF` staging copy + `$FFB7-BA` probe scratch.
#[derive(Clone, Default)]
pub struct Staging {
    pub b: [u8; 32],
    pub px: u16,
    pub py: u16,
}

impl Staging {
    pub fn x(&self) -> u16 {
        x_of(&self.b)
    }
    pub fn y(&self) -> u16 {
        y_of(&self.b)
    }
}

/// Mario state the engine reads (post player-update, pre object engine).
#[derive(Debug, Clone, Copy, Default)]
pub struct MarioContact {
    pub x: u16,
    pub y: u16,
    /// `sMarioDead`.
    pub dead: u8,
    /// `sCurPowerup` (0 = small).
    pub powerup: u8,
    /// `sMarioDucking`.
    pub ducking: bool,
    /// `sSpinJump`.
    pub spin_jump: bool,
    /// `sStarTimerHigh`.
    pub star_high: u8,
    /// `sInvincibilityTimer`.
    pub invincibility: u8,
    /// `sFlutterTimer`.
    pub flutter: u8,
    /// `sVVelocityIndex`.
    pub vvel: u8,
    /// `sMarioOnGround`.
    pub on_ground: bool,
    /// `sMarioOnObject`.
    pub on_object: bool,
    /// `hKeysHeld` bit 1 (B).
    pub b_held: bool,
    /// `hKeysHeld` bit 0 (A).
    pub a_held: bool,
    /// `sSceneState`.
    pub scene_state: u8,
    /// `sMoonPhysics`.
    pub moon_physics: u8,
    /// `sCameraPanTimer` (the `$2062` scroll-Y nudge).
    pub pan: u8,
    /// `sPipeTravelDirection >= $10`: the frame loop skips `$2D7D` entirely
    /// while Mario is inside a pipe (`$05E4` line `ld a,[sPipeTravelDirection]`).
    pub travelling: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotView {
    pub state: SlotState,
    pub x: u16,
    pub y: u16,
    pub def: u8,
    pub sprite: u8,
    pub rec_index: u16,
}

impl From<&ObjSlot> for SlotView {
    fn from(s: &ObjSlot) -> Self {
        Self {
            state: s.state(),
            x: s.x(),
            y: s.y(),
            def: s.def(),
            sprite: s.sprite(),
            rec_index: s.rec_index(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectSim {
    pub slots: [ObjSlot; SLOT_COUNT],
    pub records: Vec<SpawnRecord>,
    /// Byte offset into the record stream (`sObjectStreamPtr` − `$AB06`).
    pub stream_index: usize,
    /// Spawned markers per record (the engine writes `2` into the record's
    /// mode byte; kept separately here).
    pub spawned: Vec<bool>,
    pub prev_cam_x: u16,
    /// `sObjectScanDir`: 1 forward, else backward, except that when the
    /// camera did not move the engine stores the camera's low byte there.
    pub scan_dir: u8,
    pub spawn_x: u16,
    /// `sObjectUpdatePhase` (toggled at `$8000` entry).
    pub phase: bool,
    /// `$FFF0 & 1` — the odd/even split for cull/promote gates.
    pub odd: bool,
    /// `$FFF0` itself (`& 3` gates the sprite toggle / walker state machine).
    pub fff0: u8,
    /// `sObjectUpdatePhase` as a byte (the slot-parity gate in `$58000`).
    pub update_phase: u8,
    pub standby: u8,
    pub active: u8,
    pub easy_mode: bool,
    pub sprite_map_index: u8,
    pub level_id: u8,
    /// `sObjectDone` (0 normal, 1 stop, 2 removed, 3 demoted).
    pub done: u8,
    /// `sClearStageFlags` — the `$818D` tail clears staging `+23` when set.
    pub clear_stage_flags: bool,
    /// `sPendingItemId` (`$A26C`) — what the `$2DC6` item branch staged. The
    /// frame loop picks it up at the tail of the *next* frame's `$0F71`
    /// (`$319F`, then `$3451`); the runtime hands it to the player sim.
    pub pending_item: u8,
    /// `sObjectRemoveMode`.
    pub remove_mode: u8,
    /// `sMoveFlags` (bank-2 move/probe helper result bits).
    pub move_flags: u8,
    /// `sObjectFacing` (1 = Mario right, `$FF` = left, 0 = equal).
    pub facing: u8,
    /// `sMarioRefX/Y` (`mario + 8`, `mario + 32`, set at `$8000` entry).
    pub mario_ref_x: u16,
    pub mario_ref_y: u16,
    /// `sMarioDead` (the harmful contact sets it to 15).
    pub mario_dead: u8,
    /// `sObjectBounce` — a stomp; the frame loop turns it into a jump.
    pub bounce: bool,
    /// The `$2D7D` tail: `sMarioInAir = $FF`, `sVVelocityIndex` 48/75.
    pub stomp_jump: bool,
    /// The bounce path's `$2E5C` tail (vvel 49) — `sSceneState != 0` only.
    pub stomp_bounce: bool,
    /// `sMarioOnObject`.
    pub on_object: bool,
    /// `sMarioOnGround` cleared by the response path.
    pub on_ground_cleared: bool,
    /// Direct Mario Y writes from the response (`$FFC0/C1`).
    pub mario_y_adjust: Option<u16>,
    /// `sStompBounceTimer` / `sStompChainCount` / `sMoneyBagCount`.
    pub stomp_bounce_timer: u8,
    pub stomp_chain: u8,
    pub money_bag: u8,
    /// `sObjectWindowHit` (staged-window test result, for debugging).
    pub window_hit: bool,
    /// `sSpriteFlipFlags` (recomputed by the draw: `+12 ^ +13 ^ +14`).
    pub sprite_flip_flags: u8,
    /// `sGoalBell` (objects lose their type when the goal rings).
    pub goal_bell: u8,
    /// `sSecretRouteFlag` (`$A2A0`) — the anim-23 removal gate.
    pub secret_route: bool,
    /// `sObjectSpawnAnim/Sprite/Attr` (used by the `$5B346` bonus spawn).
    pub spawn_anim: u8,
    pub spawn_sprite: u8,
    pub spawn_attr: u8,
    /// Mario contact for the current frame (read by the handlers).
    pub mario: MarioContact,
    /// ROM addresses of bank-16 handlers that are not ported yet.
    pub unported: Vec<u32>,
}

impl ObjectSim {
    pub fn new(records: Vec<SpawnRecord>, cam_x: u16) -> Self {
        let n = records.len();
        Self {
            slots: Default::default(),
            records,
            stream_index: 0,
            spawned: vec![false; n],
            prev_cam_x: cam_x,
            scan_dir: 0,
            // Level load leaves spawnX with the first scan edge's low byte
            // and the $FF high byte (the "camera unchanged" case).
            spawn_x: 0xFF00 | ((cam_x + 0x70) & 0xF8),
            phase: true,
            odd: false,
            fff0: 0,
            update_phase: 0,
            standby: 0,
            active: 0,
            easy_mode: false,
            sprite_map_index: 0,
            level_id: 0,
            done: 0,
            pending_item: 0,
            clear_stage_flags: false,
            remove_mode: 0,
            move_flags: 0,
            facing: 0,
            mario_ref_x: 0,
            mario_ref_y: 0,
            mario_dead: 0,
            bounce: false,
            stomp_jump: false,
            stomp_bounce: false,
            on_object: false,
            on_ground_cleared: false,
            mario_y_adjust: None,
            stomp_bounce_timer: 0,
            stomp_chain: 0,
            money_bag: 0,
            window_hit: false,
            sprite_flip_flags: 0,
            goal_bell: 0,
            secret_route: false,
            spawn_anim: 0,
            spawn_sprite: 0,
            spawn_attr: 0,
            mario: MarioContact::default(),
            unported: Vec::new(),
        }
    }

    /// Set the level context (sprite-map mode).
    pub fn configure(&mut self, level_id: u8) {
        self.level_id = level_id;
        self.sprite_map_index = crate::object::sprite_map_index(level_id);
    }

    fn note_unported(&mut self, addr: u32) {
        if !self.unported.contains(&addr) {
            self.unported.push(addr);
        }
    }

    /// `$8000` windows: ±`half` around `c` with the min clamped at 0.
    fn window(c: u16, half: i32) -> (u16, u16) {
        let c = c as i32;
        ((c - half).max(0) as u16, (c + half) as u16)
    }

    /// `$82D5`/`$8306`: `min <= v <= max` (inclusive).
    fn window_hit(v: u16, w: (u16, u16)) -> bool {
        v >= w.0 && v <= w.1
    }

    /// One full frame: Mario interaction pass (`$2D7D`, which the frame loop
    /// runs before the object engine) then the bank-2 `$8000` engine.
    pub fn step(
        &mut self,
        cam_x: u16,
        cam_y: u16,
        layout: &LevelLayout,
        tables: &ObjectTables,
        mario: &MarioContact,
    ) {
        self.mario = *mario;
        if !mario.travelling {
            self.collide(cam_x, cam_y, layout);
        }
        self.engine(cam_x, cam_y, layout, tables);
    }

    /// The `$EBBB` level-(re)load tail: `$EC40` clears the object scratch
    /// page ($AF00-$AFFF — spawn X, sprite map index, update phase, the
    /// previous-camera latch, move flags, facing, ...), `$EC17` re-seeds the
    /// stream buffer and `$EC31` clears the 16 slots. `$FFF0` lives in HRAM
    /// and is *not* cleared — the toggle phase carries across the reload.
    pub fn reset_level_start(&mut self) {
        for s in self.slots.iter_mut() {
            s.b = [0; 32];
        }
        for m in self.spawned.iter_mut() {
            *m = false;
        }
        self.stream_index = 0;
        self.prev_cam_x = 0;
        self.scan_dir = 0;
        self.spawn_x = 0;
        self.update_phase = 0;
        self.phase = false;
        self.standby = 0;
        self.active = 0;
        self.done = 0;
        self.pending_item = 0;
        self.remove_mode = 0;
        self.clear_stage_flags = false;
        self.move_flags = 0;
        self.facing = 0;
        self.mario_ref_x = 0;
        self.mario_ref_y = 0;
        self.mario_dead = 0;
        self.bounce = false;
        self.stomp_jump = false;
        self.stomp_bounce = false;
        self.on_object = false;
        self.on_ground_cleared = false;
        self.mario_y_adjust = None;
        self.stomp_bounce_timer = 0;
        self.stomp_chain = 0;
        self.money_bag = 0;
        self.window_hit = false;
        self.sprite_flip_flags = 0;
        self.goal_bell = 0;
        self.spawn_anim = 0;
        self.spawn_sprite = 0;
        self.spawn_attr = 0;
        self.mario = MarioContact::default();
        // $AF06: $EC40 clears it; $EC4B then sets 1 for levels >= $0A.
        self.sprite_map_index = if self.level_id >= 0x0A { 1 } else { 0 };
        self.odd = self.fff0 & 1 == 1; // fff0 kept ($FFF0 is HRAM)
    }

    // ---------------------------------------------------------------------
    // $8000 — windows, slot loop, spawn scan
    // ---------------------------------------------------------------------

    fn engine(&mut self, cam_x: u16, cam_y: u16, layout: &LevelLayout, tables: &ObjectTables) {
        let act_x = Self::window(cam_x, 0x60);
        let act_y = Self::window(cam_y, 0x60);
        let scan_x = Self::window(cam_x, 0x70);
        let alive_x = Self::window(cam_x, 0xA0);
        let alive_y = Self::window(cam_y, 0xA0);

        // $8064: scan direction/edge from the camera delta. Three-way:
        // - camera < prev: dir $FF, edge = scanX min & $F8
        // - camera > prev: dir 1, edge = scanX max & $F8
        // - camera equal: dir = the camera low byte (!), spawnXHi = $FF and
        //   the spawnX low byte stays (the scan returns early on that hi).
        let (dir, spawn_x) = if cam_x < self.prev_cam_x {
            (0xFF, scan_x.0 & 0xFFF8)
        } else if cam_x > self.prev_cam_x {
            (1, scan_x.1 & 0xFFF8)
        } else {
            (cam_x as u8, 0xFF00 | (self.spawn_x & 0xFF))
        };
        self.scan_dir = dir;
        self.spawn_x = spawn_x;

        // $80F7-$810A: Mario reference point (x + 8, y + 32).
        self.mario_ref_x = self.mario.x.wrapping_add(8);
        self.mario_ref_y = self.mario.y.wrapping_add(32);

        // $812E: phase toggle; $FFF0 increments when the phase becomes 1.
        self.phase = !self.phase;
        self.update_phase = u8::from(self.phase);
        if self.phase {
            self.fff0 = self.fff0.wrapping_add(1);
        }
        self.odd = self.fff0 & 1 == 1;

        // $8131: slot loop.
        for i in 0..SLOT_COUNT {
            self.done = 0;
            let state = self.slots[i].state();
            if state == SlotState::Inactive {
                continue;
            }
            if state == SlotState::Standby {
                if self.odd {
                    // $826D: alive-X cull → $8285 removal when outside.
                    if !Self::window_hit(self.slots[i].x(), alive_x) {
                        self.remove_slot(i, tables);
                    }
                } else if Self::window_hit(self.slots[i].x(), act_x)
                    && Self::window_hit(self.slots[i].y(), act_y)
                {
                    // $81F7: promote (type +17 → +18, slot+9 → +10 copy).
                    let s = &mut self.slots[i];
                    s.b[B_STATE] = 2;
                    s.b[B_TYPE] = s.b[B_PARKED];
                    s.b[B_SCRX] = s.b[B_SCRY];
                    self.standby = self.standby.wrapping_sub(1);
                    self.active = self.active.wrapping_add(1);
                    self.done = 1;
                }
                continue;
            }
            // Active.
            if self.odd {
                if !Self::window_hit(self.slots[i].x(), alive_x) {
                    self.remove_slot(i, tables);
                    continue;
                }
            } else if !Self::window_hit(self.slots[i].y(), alive_y) {
                // $828F: demote to standby (or remove when not stream-backed).
                if self.slots[i].b[B_RECHI] >= 0xAD {
                    self.remove_slot(i, tables);
                } else {
                    let s = &mut self.slots[i];
                    s.b[B_STATE] = 1;
                    s.b[B_PARKED] = s.b[B_TYPE];
                    s.b[B_TYPE] = 0;
                    self.done = 3;
                    self.active = self.active.wrapping_sub(1);
                    self.standby = self.standby.wrapping_add(1);
                }
                continue;
            }
            self.active_update(i, cam_x, cam_y, layout, tables);
        }

        // $817D: spawn scan + previous-camera update.
        if self.scan_dir != 0 && (self.spawn_x >> 8) != 0xFF {
            self.scan(tables);
        }
        self.prev_cam_x = cam_x;
    }

    /// `$8285`: cull-removal — stage the slot, `$5B2D7`, hitbox, copy back.
    fn remove_slot(&mut self, i: usize, tables: &ObjectTables) {
        let mut st = Staging {
            b: self.slots[i].b,
            ..Default::default()
        };
        self.remove_object(&mut st, tables);
        self.slots[i].b = st.b;
    }

    /// `$818D`: copy slot → staging, bank-16 handler, draw, flag clear,
    /// hitbox refresh, copy back.
    fn active_update(
        &mut self,
        i: usize,
        cam_x: u16,
        cam_y: u16,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        let mut st = Staging {
            b: self.slots[i].b,
            ..Default::default()
        };
        // $81A7 (slot → staging) + $3A50 (bank 16).
        self.bank16_update(i, &mut st, layout, tables);
        // $3C80 (bank 3 draw): `$C000` returns immediately unless the staged
        // state is 2, so a removed slot keeps its (zeroed) screen bytes; the
        // `$C090` screen pos + flip flags are computed first otherwise.
        if st.b[B_STATE] == 2 {
            let scroll_x = cam_x.wrapping_sub(80);
            let scroll_y = cam_y
                .wrapping_sub(72)
                .wrapping_sub((self.mario.pan & 3) as u16);
            st.b[B_SCRY] = st.y().wrapping_add(16).wrapping_sub(scroll_y) as u8;
            st.b[B_SCRX] = st.x().wrapping_add(8).wrapping_sub(scroll_x) as u8;
            self.sprite_flip_flags = st.b[B_DEFEAT] ^ st.b[B_FLIP] ^ st.b[B_FLASH];
        }
        // $818D tail: clear the interaction flags when the handler asked.
        if self.clear_stage_flags {
            st.b[B_FLAGS] = 0;
            self.clear_stage_flags = false;
        }
        self.refresh_hitbox(&mut st, tables);
        // $81BD: staging → slot.
        self.slots[i].b = st.b;
    }

    /// `$81CF`: copy the 4-byte hitbox record for the staged sprite id.
    fn refresh_hitbox(&self, st: &mut Staging, tables: &ObjectTables) {
        if let Some(hb) = tables.hitbox_for(self.level_id, st.b[B_SPRITE]) {
            st.b[B_HB_TOP] = hb[0] as u8;
            st.b[B_HB_BOTTOM] = hb[1] as u8;
            st.b[B_HB_LEFT] = hb[2] as u8;
            st.b[B_HB_RIGHT] = hb[3] as u8;
        }
    }

    /// `$58000`: the bank-16 per-slot dispatcher.
    fn bank16_update(
        &mut self,
        i: usize,
        st: &mut Staging,
        layout: &LevelLayout,
        tables: &ObjectTables,
    ) {
        if self.mario_dead == 0xFF {
            return;
        }
        if self.goal_bell != 0 {
            st.b[B_TYPE] = 0;
        }
        // $3A8C → bank-2 $85AA: interaction-flag dispatch.
        self.interaction_dispatch(st, tables);
        if self.done != 0 {
            return;
        }
        if st.b[B_FSTATE] != 0 && (i & 1) as u8 != (self.update_phase & 1) {
            return;
        }
        self.clear_stage_flags = true; // $5802B
        if st.b[B_HITREACT] != 0 {
            self.hit_reaction(st, layout, tables);
            return;
        }
        self.run_handler(st, layout, tables);
    }

    /// `anim → handler` via the extracted profile tables (`$58040`/`$59C07`).
    fn run_handler(&mut self, st: &mut Staging, layout: &LevelLayout, tables: &ObjectTables) {
        let anim = st.b[B_DEF];
        let handler = tables
            .profile_for(self.level_id, anim)
            .map(|p| p.handler)
            .unwrap_or(0);
        match handler {
            0x58201 => self.handler_walker(st, layout),
            0x582A9 => self.handler_squashed(st, tables),
            0x582D1 => self.handler_fallwalker(st, layout),
            0x5834F => self.handler_stun(st, layout, tables),
            0x584E0 => self.handler_bobber(st, tables),
            0x5862E => self.handler_crawler(st, layout),
            0x586E1 => self.handler_marker85(st),
            0x586F5 => self.handler_pickup16(st, tables),
            0x58712 => self.handler_hopper(st, layout, tables),
            0x58755 => self.handler_itempose20(st),
            0x58758 => self.itempose_body(st),
            0x587BD => self.handler_pose86(st),
            0x587D7 => self.handler_pose87(st, tables),
            // $58100 — a bare `ret` (ROM bytes: the single byte `$C9` right after
            // the 192-byte mode-A table at $58040). Nineteen mode-A indices and
            // eleven mode-B ones dispatch here, and it is a *real* target, not a
            // missing one: the $58000 prelude still runs (interaction, bounce,
            // star), only the per-anim body does nothing. Dispatching it
            // explicitly keeps it out of `note_unported`, where it would look
            // like a gap in the port rather than a fixture of the ROM.
            0x58100 => {}
            h => self.note_unported(h),
        }
    }

    // ---------------------------------------------------------------------
    // Shared bank-16 helpers
    // ---------------------------------------------------------------------

    /// `$1F2F`: the level tile at ((x >> 4), (y >> 4)).
    fn probe(layout: &LevelLayout, x: u16, y: u16) -> u8 {
        layout
            .get(((x >> 4) & 0xFF) as usize, ((y >> 4) & 0xFF) as usize)
            .unwrap_or(0)
    }

    /// `$1EFA`: the level tile at ((x − 8) >> 4, (y − 16) >> 4).
    fn probe_bias(layout: &LevelLayout, x: u16, y: u16) -> u8 {
        Self::probe(layout, x.wrapping_sub(8), y.wrapping_sub(16))
    }

    /// `$8240`: sprite = anim→sprite map, `+14..+30` cleared.
    fn derive_sprite(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_RECHI] >= 0xAD {
            return;
        }
        let sprite = tables.sprite_for(self.level_id, st.b[B_DEF]).unwrap_or(0);
        st.b[B_SPRITE] = sprite;
        // $863C stages $FFDD → sObjectSpawnAttr and $8240 writes it back to
        // +13, so the flip byte is preserved; +14..+30 are zeroed.
        for b in st.b.iter_mut().take(31).skip(14) {
            *b = 0;
        }
    }

    /// `$863C`/`$3AA0`: sprite re-derive + object done this frame.
    fn finish_sprite_swap(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.derive_sprite(st, tables);
        self.done = 1;
    }

    /// The `$AB06` stream-record index a staging's `+6/+7` pointer aims at.
    /// The pointer is the record's `+2` word, and `hi >= $AD` marks a slot
    /// that is not stream-backed (`$8240`/`$5B2F0` take the same gate).
    fn record_index(&self, st: &Staging) -> Option<usize> {
        let hi = st.b[B_RECHI];
        if hi >= 0xAD {
            return None;
        }
        let addr = ((hi as u16) << 8) | st.b[B_RECLO] as u16;
        let ri = addr.wrapping_sub(STREAM_BASE) as usize / RECORD_LEN;
        (ri < self.records.len()).then_some(ri)
    }

    /// `$86E4`'s record write: the spawn record's anim byte (`ptr + 3` =
    /// record+5) takes the new anim, so a later re-spawn of that record is
    /// the swapped pose. The port's records are the parsed stream, so this
    /// edits `anim_index` in place.
    fn record_set_anim(&mut self, st: &Staging, anim: u8) {
        if let Some(ri) = self.record_index(st) {
            self.records[ri].anim_index = anim;
        }
    }

    /// `$5B2D7`: removal — clear the record marker when not in remove-mode,
    /// adjust the counters, zero the staging (+ hitbox for sprite 0), done 2.
    fn remove_object(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if self.remove_mode == 0 {
            if let Some(ri) = self.record_index(st) {
                // [$FFD2] == $10 would write $10 back (spent marker).
                self.spawned[ri] = false;
            }
        }
        if st.b[B_STATE] == 1 {
            self.standby = self.standby.wrapping_sub(1);
        } else {
            self.active = self.active.wrapping_sub(1);
            if self.remove_mode == 1 {
                self.money_bag_add();
            }
        }
        st.b = [0; 32];
        self.refresh_hitbox(st, tables);
        self.done = 2;
        self.remove_mode = 0;
    }

    /// `$5B31D`: BCD coin counter; on overflow spawn the bonus sprite.
    fn money_bag_add(&mut self) {
        let (v, overflow) = bcd_add(self.money_bag, 1);
        self.money_bag = v;
        if overflow {
            self.bonus_spawn();
        }
    }

    /// `$5B346`: spawn sprite 28 / anim 26 into the first free slot at the
    /// camera position (`$5B3AE` clears the spawn fields when full).
    fn bonus_spawn(&mut self) {
        let free = (0..SLOT_COUNT).find(|&i| self.slots[i].b[B_STATE] == 0);
        let Some(i) = free else {
            self.spawn_anim = 0;
            self.spawn_sprite = 0;
            self.spawn_attr = 0;
            return;
        };
        let slot_hi = 0xAD + (i as u8 >> 1);
        let slot_lo = (i as u8 & 1) * 0x20;
        let cam = self.prev_cam_x; // the frame's camera is stored at $817D
        let camy = 0u16; // camera Y is not needed for the walker captures
        let s = &mut self.slots[i];
        set_x(&mut s.b, cam);
        set_y(&mut s.b, camy);
        s.b[B_DEF] = self.spawn_anim;
        s.b[B_RECHI] = slot_hi;
        s.b[B_RECLO] = slot_lo & 0xF0;
        s.b[B_STATE] = 1;
        s.b[B_SPRITE] = self.spawn_sprite;
        s.b[B_FLIP] = self.spawn_attr;
        self.standby = self.standby.wrapping_add(1);
        self.spawn_anim = 0;
        self.spawn_sprite = 0;
        self.spawn_attr = 0;
    }

    /// `$5B51A`: toggle the staged sprite's bit 0 every 4th `$FFF0`.
    fn sprite_toggle(&self, st: &mut Staging) {
        if self.fff0 & 3 == 0 {
            st.b[B_SPRITE] ^= 1;
        }
    }

    /// `$5B3C6`: face Mario — `$FFDF` = 1 (left) / 2 (right), `$FFDD` = 0 / `$20`.
    fn face_mario(&mut self, st: &mut Staging) {
        // $5B3E0: compare sMarioRefX with the object X.
        let obj_x = st.x();
        self.facing = if self.mario_ref_x > obj_x {
            1
        } else if self.mario_ref_x < obj_x {
            0xFF
        } else {
            0
        };
        if self.facing == 1 {
            st.b[B_FSTATE] = 2;
            st.b[B_FLIP] = 0x20;
        } else {
            st.b[B_FSTATE] = 1;
            st.b[B_FLIP] = 0;
        }
    }

    /// `$5B451`: step `+16` through `FALL_SPEED` (cap 21); returns the speed.
    fn fall_step(&self, st: &mut Staging) -> u8 {
        let idx = (st.b[B_PHASE] as usize).min(21);
        let b = FALL_SPEED[idx];
        if st.b[B_PHASE] != 21 {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        b
    }

    /// `$5B481`: step `+16` through `BOUNCE_SPEED` (cap 15); returns speed.
    fn bounce_step(&self, st: &mut Staging) -> u8 {
        let idx = (st.b[B_PHASE] as usize).min(15);
        let b = BOUNCE_SPEED[idx];
        if st.b[B_PHASE] != 15 {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        b
    }

    /// `$5B521`: sprite bit0 toggle on even `$FFF0` (anim 8's faster variant
    /// of `$5B51A`).
    fn sprite_toggle1(&self, st: &mut Staging) {
        if self.fff0 & 1 == 0 {
            st.b[B_SPRITE] ^= 0x01;
        }
    }

    /// `$5B68B`: `+14` bit4 toggle on even `$FFF0` (the anim-20-family flash).
    fn flash_toggle(&self, st: &mut Staging) {
        if self.fff0 & 1 == 0 {
            st.b[B_FLASH] ^= 0x10;
        }
    }

    /// `$5B47A`: step `+16` through `HOVER_SPEED` (cap 21); returns the speed.
    fn hover_step(&self, st: &mut Staging) -> u8 {
        let idx = (st.b[B_PHASE] as usize).min(21);
        let b = HOVER_SPEED[idx];
        if st.b[B_PHASE] != 21 {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        b
    }

    /// `$5B545`: `+26` cycles 0..3 every 4th `$FFF0`, adds `SPIN_WOBBLE[+26]`
    /// to the sprite; sets `+13` bit5 at phase 3 and clears it on the wrap.
    fn spin_step(&self, st: &mut Staging) {
        if self.fff0 & 3 != 0 {
            return;
        }
        st.b[B_ANIMFRAME] = st.b[B_ANIMFRAME].wrapping_add(1);
        if st.b[B_ANIMFRAME] == 4 {
            st.b[B_ANIMFRAME] = 0;
            st.b[B_FLIP] &= !0x20;
        }
        st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_add(SPIN_WOBBLE[st.b[B_ANIMFRAME] as usize]);
        if st.b[B_ANIMFRAME] == 3 {
            st.b[B_FLIP] |= 0x20;
        }
    }

    /// `$5B4B4`: low-byte `|sMarioRefX − X|` (`bit 7` set → negate).
    fn dist_mario_x(&self, st: &Staging) -> u8 {
        let d = (self.mario_ref_x as u8).wrapping_sub(st.b[B_XLO]);
        if d & 0x80 != 0 {
            d.wrapping_neg()
        } else {
            d
        }
    }

    /// `$58101`: the shared hit-reaction path (`$FFEB` set by the dispatch).
    fn hit_reaction(&mut self, st: &mut Staging, layout: &LevelLayout, tables: &ObjectTables) {
        if st.b[B_FSTATE] == 0 {
            self.face_mario(st);
            st.b[B_FLIP] ^= 0x20;
            return;
        }
        match st.b[B_FSTATE].wrapping_sub(1) {
            // $411A: knocked right — X+1, wall probe; bounce up; the state
            // advances +2 every 15 phases.
            0 => {
                self.mv_x_add(st, 1);
                self.m_right_wall(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                }
                let b = self.bounce_step(st);
                self.mv_y_sub(st, b);
                if st.b[B_PHASE] == 15 {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(2);
                }
            }
            // $4142: knocked left.
            1 => {
                self.mv_x_sub(st, 1);
                self.m_left_wall(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                }
                let b = self.bounce_step(st);
                self.mv_y_sub(st, b);
                if st.b[B_PHASE] == 15 {
                    st.b[B_PHASE] = 0;
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(2);
                }
            }
            // $4158: airborne right — fall; land (revive or remove).
            2 => {
                self.mv_x_add(st, 1);
                self.m_right_wall(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                }
                self.land_or_remove(st, layout, tables);
            }
            // $4185: airborne left.
            _ => {
                self.mv_x_sub(st, 1);
                self.m_left_wall(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                }
                self.land_or_remove(st, layout, tables);
            }
        }
    }

    /// `$5816C`/`$5819B`: the airborne hit-reaction tail.
    fn land_or_remove(&mut self, st: &mut Staging, layout: &LevelLayout, tables: &ObjectTables) {
        let b = self.fall_step(st);
        self.mv_y_add(st, b);
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 == 0 {
            return;
        }
        if st.b[B_HITREACT] == 2 {
            // $5819B: revive — sprite re-derive, then the frame state is set
            // from the flip byte ($3CB1: 1/2) and advanced.
            let saved_type = st.b[B_TYPE];
            let d = self.done;
            self.finish_sprite_swap(st, tables);
            self.done = d;
            st.b[B_TYPE] = saved_type;
            st.b[B_FSTATE] = if st.b[B_FLIP] & 0x20 != 0 { 3 } else { 2 };
        } else {
            let d = self.done;
            self.finish_sprite_swap(st, tables);
            self.done = d;
        }
    }

    // ---------------------------------------------------------------------
    // Anim 9 — the walker ($58201)
    // ---------------------------------------------------------------------

    /// `$58201` + `$58215`: sprite toggle, init, then the four-state walker.
    fn handler_walker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.sprite_toggle(st);
        if st.b[B_FSTATE] == 0 {
            // First run: face Mario, set flash + type, done.
            self.face_mario(st);
            st.b[B_FLASH] = 16;
            st.b[B_TYPE] = 0x4E;
            return;
        }
        // $58215: main loop, gated to even $FFF0.
        if self.fff0 & 1 != 0 {
            return;
        }
        self.walker_state(st, layout, st.b[B_FSTATE]);
    }

    /// The four walker states (`$4226`/`$4253`/`$4271`/`$4282`) — gate-free;
    /// anim 17/21 `jp`s straight into them from their own state numbers.
    fn walker_state(&mut self, st: &mut Staging, layout: &LevelLayout, state: u8) {
        match state {
            // $4226: walk left; wall on the left → turn; no ground → fall.
            1 => {
                self.m_left_wall(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                    st.b[B_FLIP] ^= 0x20;
                    return;
                }
                self.mv_x_sub(st, 1);
                self.walk_ground_check(st, layout);
            }
            // $4253: walk right; wall on the right → turn.
            2 => {
                self.m_right_wall(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                    st.b[B_FLIP] ^= 0x20;
                    return;
                }
                self.mv_x_add(st, 1);
                self.walk_ground_check(st, layout);
            }
            // $4271: falling left.
            3 => {
                self.m_left_wall(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                    st.b[B_FLIP] ^= 0x20;
                    return;
                }
                self.mv_x_sub(st, 1);
                self.fall_step_and_land(st, layout);
            }
            // $4282: falling right.
            4 => {
                self.m_right_wall(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                    st.b[B_FLIP] ^= 0x20;
                    return;
                }
                self.mv_x_add(st, 1);
                self.fall_step_and_land(st, layout);
            }
            _ => {}
        }
    }

    /// `$582D1` (anim 1): fall to the ground, then walk (states 1/2) with
    /// wall + ledge turns; type `$56`.
    fn handler_fallwalker(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.sprite_bob(st); // $5B585
        match st.b[B_FSTATE] {
            0 => {
                st.b[B_TYPE] = 0x56;
                // $582DD: fall until grounded, then face Mario ($582EE).
                self.m_ground_own(st, layout);
                if self.move_flags & 0x08 != 0 {
                    st.b[B_PHASE] = 0;
                    self.face_mario(st);
                    return;
                }
                let b = self.fall_step(st);
                self.mv_y_add(st, b);
            }
            // $582FB (state 1, walk left): move + wall probe on even $FFF0;
            // ground check (ledge reverse) on odd.
            1 => {
                if self.fff0 & 1 == 0 {
                    self.mv_x_sub(st, 1);
                    self.m_left_wall(st, layout);
                    if self.move_flags & 0x02 != 0 {
                        st.b[B_FSTATE] = 2;
                        st.b[B_FLIP] ^= 0x20;
                    }
                } else {
                    self.ledge_reverse(st, layout);
                }
            }
            // $5831A (state 2, walk right).
            2 => {
                if self.fff0 & 1 == 0 {
                    self.mv_x_add(st, 1);
                    self.m_right_wall(st, layout);
                    if self.move_flags & 0x01 != 0 {
                        st.b[B_FSTATE] = 1;
                        st.b[B_FLIP] ^= 0x20;
                    }
                } else {
                    self.ledge_reverse(st, layout);
                }
            }
            _ => {}
        }
    }

    /// `$58339`: ground check; walking off a ledge reverses the state (1↔2).
    fn ledge_reverse(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 != 0 {
            return;
        }
        st.b[B_FSTATE] ^= 3;
        st.b[B_FLIP] ^= 0x20;
    }

    /// `$5834F` (anim 2): anim 1's stunned variant — type `$06`, a 192-tick
    /// countdown (falling while it runs), then an 8-flicker spin
    /// (`sprite ^= $0B` every 4th `$FFF0`) and back to anim 1.
    fn handler_stun(&mut self, st: &mut Staging, layout: &LevelLayout, tables: &ObjectTables) {
        match st.b[B_FSTATE] {
            // $58364: count down; fall when airborne.
            1 => {
                st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                if st.b[B_T28] == 0 {
                    st.b[B_FSTATE] = 2;
                    return;
                }
                self.m_ground_own(st, layout);
                if self.move_flags & 0x08 != 0 {
                    return;
                }
                let b = self.fall_step(st);
                self.mv_y_add(st, b);
            }
            // $58380: spin, then revive to anim 1 ($3AA0 sprite re-derive).
            2 => {
                if self.fff0 & 3 != 0 {
                    return;
                }
                st.b[B_SPRITE] ^= 0x0B;
                st.b[B_T28] = st.b[B_T28].wrapping_add(1);
                if st.b[B_T28] != 8 {
                    return;
                }
                st.b[B_DEF] = 1;
                self.finish_sprite_swap(st, tables);
            }
            // fstate 0: init.
            _ => {
                st.b[B_TYPE] = 0x06;
                st.b[B_FSTATE] = 1;
                st.b[B_T28] = 192;
            }
        }
    }

    /// `$58712` (anims 17/21): a walker variant — state 0 poses as an item
    /// (type `$80`, `+12 = $80`, done), state 1 pops up 8 px, states 2-5
    /// jump straight into the walker's states 1-4.
    fn handler_hopper(&mut self, st: &mut Staging, layout: &LevelLayout, tables: &ObjectTables) {
        let _ = tables;
        match st.b[B_FSTATE] {
            1 => {
                // $58734: rise 1 px per call; at 8 → state 3, clear +12.
                self.mv_y_sub(st, 1);
                st.b[B_T24] = st.b[B_T24].wrapping_add(1);
                if st.b[B_T24] != 8 {
                    return;
                }
                st.b[B_FSTATE] = 3;
                st.b[B_DEFEAT] = 0;
            }
            2 => self.walker_state(st, layout, 1),
            3 => self.walker_state(st, layout, 2),
            4 => self.walker_state(st, layout, 3),
            5 => self.walker_state(st, layout, 4),
            _ => {
                // state 0: item setup.
                st.b[B_TYPE] = 0x80;
                st.b[B_T24] = 0;
                st.b[B_DEFEAT] = 0x80;
                self.done = 1;
            }
        }
    }

    // ---------------------------------------------------------------------
    // Anim 12 — the bobber ($584E0)
    // ---------------------------------------------------------------------

    /// `$584E0`: pipe travel removes the object; state 0 (`$584F3`) stages the
    /// `+12 = $80` pose every call until Mario's low-byte X distance reaches
    /// `$18`, then sets type `$44` and two 32-step timers; states 1/2 rise /
    /// sink one px per call for `+24` steps; state 3 waits `+28` frames out
    /// and re-arms. (Anim 7 `$584CB` is the mirrored variant — not in level 0.)
    fn handler_bobber(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if self.mario.travelling {
            self.remove_object(st, tables);
            return;
        }
        match st.b[B_FSTATE] {
            1 => {
                // $5850C — rise.
                self.sprite_toggle(st);
                self.mv_y_sub(st, 1);
                st.b[B_T24] = st.b[B_T24].wrapping_sub(1);
                if st.b[B_T24] != 0 {
                    return;
                }
                self.bobber_reload(st);
            }
            2 => {
                // $5851C — sink.
                self.sprite_toggle(st);
                self.mv_y_add(st, 1);
                st.b[B_T24] = st.b[B_T24].wrapping_sub(1);
                if st.b[B_T24] != 0 {
                    return;
                }
                self.bobber_reload(st);
            }
            3 => {
                // $58537 — rest, then state 0.
                st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                if st.b[B_T28] != 0 {
                    return;
                }
                st.b[B_FSTATE] = 0;
            }
            _ => {
                // $584F3 — the wait-for-Mario setup.
                st.b[B_DEFEAT] = 0x80;
                if self.dist_mario_x(st) < 0x18 {
                    return;
                }
                st.b[B_FSTATE] = 1;
                st.b[B_T24] = 32;
                st.b[B_T25] = 32;
                st.b[B_TYPE] = 0x44;
            }
        }
    }

    /// `$5852A`: reload `+24` from `+25`, advance the state, `+28` = 24.
    fn bobber_reload(&self, st: &mut Staging) {
        st.b[B_T24] = st.b[B_T25];
        st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
        st.b[B_T28] = 24;
    }

    // ---------------------------------------------------------------------
    // Anim 8 — the hover crawler ($5862E)
    // ---------------------------------------------------------------------

    /// `$5862E`: sprite bit0 toggle; state 0 faces Mario (type `$56`, flash
    /// 16); states 1/3 walk left, 2/4 walk right on even `$FFF0` (wall turns
    /// the state); every odd `$FFF0` runs the `$5868C` hover/fall half.
    fn handler_crawler(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.sprite_toggle1(st);
        match st.b[B_FSTATE] {
            0 => {
                self.face_mario(st);
                st.b[B_FLASH] = 16;
                st.b[B_TYPE] = 0x56;
            }
            1 | 3 => {
                // $5864E
                if self.fff0 & 1 == 0 {
                    self.mv_x_sub(st, 1);
                    self.m_left_wall(st, layout);
                    if self.move_flags & 0x02 != 0 {
                        st.b[B_FSTATE] = 2;
                        st.b[B_FLIP] ^= 0x20;
                    }
                    return;
                }
                self.crawler_vertical(st, layout);
            }
            2 | 4 => {
                // $5866D
                if self.fff0 & 1 == 0 {
                    self.mv_x_add(st, 1);
                    self.m_right_wall(st, layout);
                    if self.move_flags & 0x01 != 0 {
                        st.b[B_FSTATE] = 1;
                        st.b[B_FLIP] ^= 0x20;
                    }
                    return;
                }
                self.crawler_vertical(st, layout);
            }
            _ => {}
        }
    }

    /// `$5868C`/`$586AD`: the odd-frame vertical half — hover up through
    /// `HOVER_SPEED` until the curve tops out (index 21) or the 18-px ceiling
    /// probe hits, either of which arms `+24 = 1`; then fall through
    /// `FALL_SPEED` to the ground; four landings reset at Mario (`$586D6`).
    fn crawler_vertical(&mut self, st: &mut Staging, layout: &LevelLayout) {
        if st.b[B_T24] == 1 {
            // $586AD — falling.
            let b = self.fall_step(st);
            self.mv_y_add(st, b);
            self.m_ground_own(st, layout);
            if self.move_flags & 0x08 == 0 {
                return;
            }
            st.b[B_T24] = 0;
            st.b[B_T25] = st.b[B_T25].wrapping_add(1);
            if st.b[B_T25] == 4 {
                // $586D6: restarted cycle, face Mario.
                st.b[B_PHASE] = 0;
                st.b[B_T24] = 0;
                st.b[B_T25] = 0;
                self.face_mario(st);
                return;
            }
            st.b[B_PHASE] = 10;
            if st.b[B_FSTATE] >= 3 {
                return;
            }
            st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(2);
            return;
        }
        // $5868C — hovering.
        let b = self.hover_step(st);
        if st.b[B_PHASE] == 21 {
            st.b[B_PHASE] = 0;
            st.b[B_T24] = 1;
            return;
        }
        self.mv_y_sub(st, b);
        self.m_ceil_own(st, layout, 18);
        if self.move_flags & 0x04 == 0 {
            return;
        }
        st.b[B_PHASE] = 0;
        st.b[B_T24] = 1;
    }

    // ---------------------------------------------------------------------
    // Anims 18/19/20/25 — the item-pose family ($58755 / $58758)
    // ---------------------------------------------------------------------

    /// `$58755` (anim 20): the `$5B68B` flash toggle, then the `$58758` body.
    fn handler_itempose20(&mut self, st: &mut Staging) {
        self.flash_toggle(st);
        self.itempose_body(st);
    }

    /// `$58758` (anims 18/19/25; anim 20 falls into it): state 0 poses as an
    /// item (`type $80`, `+12 = $80`, done) — unless the marker byte reads
    /// `$10`, when it takes the activated pose (type = anim + `$70`, drop
    /// 8 px) into state 2. State 1 rises 1 px per even `$FFF0` (`+24` → 8)
    /// into state 2; state 2 idles.
    fn itempose_body(&mut self, st: &mut Staging) {
        match st.b[B_FSTATE] {
            1 => {
                // $58787
                if self.fff0 & 1 != 0 {
                    return;
                }
                self.mv_y_sub(st, 1);
                st.b[B_T24] = st.b[B_T24].wrapping_add(1);
                if st.b[B_T24] != 8 {
                    return;
                }
                st.b[B_FSTATE] = 2;
                st.b[B_DEFEAT] = 0;
            }
            2 => {}
            _ => {
                st.b[B_TYPE] = 0x80;
                if st.b[B_MARKER] == 0x10 {
                    // $58777 — the activated pose.
                    st.b[B_TYPE] = st.b[B_DEF].wrapping_add(0x70);
                    st.b[B_FSTATE] = 2;
                    self.mv_y_add(st, 8);
                    return;
                }
                st.b[B_DEFEAT] = 0x80;
                self.done = 1;
                st.b[B_FSTATE] = 2;
            }
        }
    }

    /// `$587BD` (anim 22): the `$86` pose until `+23` bit4 (touched) → state 1
    /// + type 0; state 1 is the shared `$587D3` spin.
    fn handler_pose86(&mut self, st: &mut Staging) {
        if st.b[B_FSTATE] == 1 {
            self.spin_step(st);
            return;
        }
        st.b[B_TYPE] = 0x86;
        if st.b[B_FLAGS] & 0x10 == 0 {
            return;
        }
        st.b[B_FSTATE] = 1;
        st.b[B_TYPE] = 0;
    }

    /// `$587D7` (anim 23): state 1 = shared spin (`$587D3`); state 2 =
    /// `+23` bit4 → state 1 + type 0; state 0 = remove via `$5B2D7`
    /// (remove-mode 2) on the secret route, else the `$87` pose + state 2.
    fn handler_pose87(&mut self, st: &mut Staging, tables: &ObjectTables) {
        match st.b[B_FSTATE] {
            1 => self.spin_step(st),
            2 => {
                if st.b[B_FLAGS] & 0x10 == 0 {
                    return;
                }
                st.b[B_FSTATE] = 1;
                st.b[B_TYPE] = 0;
            }
            _ => {
                if self.secret_route {
                    // $587FB
                    self.remove_mode = 2;
                    self.remove_object(st, tables);
                    return;
                }
                st.b[B_TYPE] = 0x87;
                st.b[B_FSTATE] = 2;
            }
        }
    }

    /// `$586E1` (anim 15): a static type-`$85` marker.
    fn handler_marker85(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = 0x85;
    }

    /// `$586F5` (anim 16): the picked-up item's rise-and-vanish arm — the body
    /// the `$8611`/`$868A` touch cluster switches a collectible into. Sets
    /// `+15 = 1` (which arms the slot-parity gate from the *next* frame, so the
    /// rise runs on every other call), runs the `$5B68B` flash, lifts Y by 2 and
    /// steps `+24` by 2; on the 16th call (`+24` reaching `$20`) it removes the
    /// slot in remove-mode 2. Mode 2 is the item despawn: `$5B2D7` skips the
    /// record-marker clear and the coin/money-bag credit (unlike mode 0/1), so
    /// the slot zeroes, `sActiveObjectCount` drops and `sObjectDone = 2`.
    fn handler_pickup16(&mut self, st: &mut Staging, tables: &ObjectTables) {
        st.b[B_FSTATE] = 1;
        self.flash_toggle(st);
        self.mv_y_add(st, 2);
        st.b[B_T24] = st.b[B_T24].wrapping_add(2);
        if st.b[B_T24] != 0x20 {
            return;
        }
        self.remove_mode = 2;
        self.remove_object(st, tables);
    }

    /// `$5B585`: sprite bob — `+26` cycles 0..3 on even `$FFF0` frames and
    /// adds `[FE, 01, FF, 02]` to the staged sprite.
    fn sprite_bob(&mut self, st: &mut Staging) {
        if self.fff0 & 1 != 0 {
            return;
        }
        st.b[B_ANIMFRAME] = st.b[B_ANIMFRAME].wrapping_add(1);
        if st.b[B_ANIMFRAME] == 4 {
            st.b[B_ANIMFRAME] = 0;
        }
        const BOB: [u8; 4] = [0xFE, 0x01, 0xFF, 0x02];
        let d = BOB[st.b[B_ANIMFRAME] as usize];
        st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_add(d);
    }

    /// `$58243`: ground check — no ground under the object → state += 2.
    fn walk_ground_check(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 == 0 {
            st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(2);
        }
    }

    /// `$58293`: fall (Y += curve) then ground check → state −= 2 on landing.
    fn fall_step_and_land(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let b = self.fall_step(st);
        self.mv_y_add(st, b);
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 != 0 {
            st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(2);
        }
    }

    // ---------------------------------------------------------------------
    // Anim 10 — squashed walker ($582A9)
    // ---------------------------------------------------------------------

    /// `$582A9`: flash/phase/type init, then count down and remove.
    fn handler_squashed(&mut self, st: &mut Staging, tables: &ObjectTables) {
        if st.b[B_FSTATE] == 1 {
            // $582C3: count +16 down; at 0 remove via $5B2D7.
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_sub(1);
            if st.b[B_PHASE] != 0 {
                return;
            }
            self.remove_mode = 1;
            self.remove_object(st, tables);
            return;
        }
        st.b[B_FLASH] = 16;
        st.b[B_FSTATE] = 1;
        st.b[B_PHASE] = 32;
        st.b[B_TYPE] = 0;
    }

    // ---------------------------------------------------------------------
    // Bank-2 move/probe helpers ($8318-$8554): stage a probe point in
    // $FFB7-BA, probe the tile and clear the sMoveFlags bit on failure.
    // ---------------------------------------------------------------------

    fn mv_x_add(&mut self, st: &mut Staging, b: u8) {
        let v = st.x().wrapping_add(b as u16);
        set_x(&mut st.b, v);
    }
    fn mv_x_sub(&mut self, st: &mut Staging, b: u8) {
        let v = st.x().wrapping_sub(b as u16);
        set_x(&mut st.b, v);
    }
    fn mv_y_add(&mut self, st: &mut Staging, b: u8) {
        let v = st.y().wrapping_add(b as u16);
        set_y(&mut st.b, v);
    }
    fn mv_y_sub(&mut self, st: &mut Staging, b: u8) {
        let v = st.y().wrapping_sub(b as u16);
        set_y(&mut st.b, v);
    }

    /// Probe at the staged point; the flag survives when the tile is `< limit`.
    fn probe_keep(&mut self, st: &Staging, layout: &LevelLayout, limit: u8, flag: u8) {
        if Self::probe(layout, st.px, st.py) < limit {
            return;
        }
        self.move_flags &= !flag;
    }

    // $8318 (home $3B18): tile < $48 16 left; the Y & $0C gate first.
    #[allow(dead_code)]
    fn m_ground_left16(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.move_flags = 0x88;
        st.py = st.y();
        if st.b[B_YLO] & 0x0C != 0 {
            self.move_flags &= !0x08;
            return;
        }
        st.px = st.x().wrapping_sub(16);
        self.probe_keep(st, layout, 0x48, 0x08);
    }

    // $8338 (home $3B2C): tile < $48 16 right.
    #[allow(dead_code)]
    fn m_ground_right16(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.move_flags = 0x88;
        st.py = st.y();
        if st.b[B_YLO] & 0x0C != 0 {
            self.move_flags &= !0x08;
            return;
        }
        st.px = st.x().wrapping_add(16);
        self.probe_keep(st, layout, 0x48, 0x08);
    }

    // $8358 (home $3B40): ground at own position; tile < $48 keeps bit3.
    fn m_ground_own(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.move_flags = 0x88;
        st.py = st.y();
        if st.b[B_YLO] & 0x0C != 0 {
            self.move_flags &= !0x08;
            return;
        }
        st.px = st.x();
        self.probe_keep(st, layout, 0x48, 0x08);
    }

    // $8381 (home $3B54): ground 16 below; tile < $48 keeps bit3.
    #[allow(dead_code)]
    fn m_ground_below16(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.move_flags = 0x88;
        st.py = st.y().wrapping_add(16);
        if (st.py as u8) & 0x0C != 0 {
            self.move_flags &= !0x08;
            return;
        }
        st.px = st.x();
        self.probe_keep(st, layout, 0x48, 0x08);
    }

    // $8394/$83B0/$83D0 (home $3B68/$3B7C/$3B90): ceiling above (own X).
    fn m_ceil_own(&mut self, st: &mut Staging, layout: &LevelLayout, dist: u8) {
        self.move_flags = 0x44;
        st.py = st.y().wrapping_sub(dist as u16);
        st.px = st.x();
        self.probe_keep(st, layout, 0x38, 0x04);
    }

    // $83EC/$8403 (home $3AF0/$3B04): ceiling above at X ∓ 16.
    #[allow(dead_code)]
    fn m_ceil_side(&mut self, st: &mut Staging, layout: &LevelLayout, dx: i16) {
        self.move_flags = 0x44;
        st.py = st.y().wrapping_sub(18);
        st.px = if dx < 0 {
            st.x().wrapping_sub(16)
        } else {
            st.x().wrapping_add(16)
        };
        self.probe_keep(st, layout, 0x38, 0x04);
    }

    /// `$841A`-family / `$84C8`-family: wall probes (two probe heights).
    fn wall_probe(&mut self, st: &mut Staging, layout: &LevelLayout, dy: u8, dx: i16, flag: u8) {
        let flags = if flag == 0x01 { 0x11 } else { 0x22 };
        self.move_flags = flags;
        st.py = st.y().wrapping_sub(dy as u16);
        st.px = if dx < 0 {
            st.x().wrapping_sub((-dx) as u16)
        } else {
            st.x().wrapping_add(dx as u16)
        };
        if Self::probe(layout, st.px, st.py) < 0x38 {
            return;
        }
        st.py = st.y().wrapping_sub(5);
        if Self::probe(layout, st.px, st.py) < 0x38 {
            return;
        }
        self.move_flags &= !flag;
    }

    // $841A (home $3BA4): wall right, probes at Y−8 / Y−5, X+7.
    #[allow(dead_code)]
    fn m_right_wall_low(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 8, 7, 0x01);
    }

    // $843C (home $3BB8): wall right, probes at Y−11 / Y−5, X+7.
    fn m_right_wall(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, 7, 0x01);
    }

    // $8462/$8484/$84A6 (home $3BCC/$3BE0/$3BF4).
    #[allow(dead_code)]
    fn m_right_wall_mid(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, 11, 0x01);
    }
    #[allow(dead_code)]
    fn m_right_wall_far(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, 19, 0x01);
    }
    #[allow(dead_code)]
    fn m_right_wall_high(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 19, 11, 0x01);
    }

    // $84C8 (home $3C08): wall left, probes at Y−8 / Y−5, X−7.
    #[allow(dead_code)]
    fn m_left_wall_low(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 8, -7, 0x02);
    }

    // $84EA (home $3C1C): wall left, probes at Y−11 / Y−5, X−7.
    fn m_left_wall(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, -7, 0x02);
    }

    // $8510/$8532/$8554 (home $3C30/$3C44/$3C58).
    #[allow(dead_code)]
    fn m_left_wall_mid(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, -11, 0x02);
    }
    #[allow(dead_code)]
    fn m_left_wall_far(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 11, -19, 0x02);
    }
    #[allow(dead_code)]
    fn m_left_wall_high(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.wall_probe(st, layout, 19, -11, 0x02);
    }

    // ---------------------------------------------------------------------
    // $85AA — interaction-flag dispatch
    // ---------------------------------------------------------------------

    fn interaction_dispatch(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let f = st.b[B_FLAGS];
        if f == 0 {
            return;
        }
        if f & 0x01 != 0 {
            self.dispatch_stomp(st, tables);
            return;
        }
        if f & 0x02 != 0 {
            self.defeat(st);
            return;
        }
        if f & 0x04 != 0 {
            self.defeat(st);
            return;
        }
        if f & 0x10 != 0 {
            // $8611: touched. A bit-7 type goes to the `$868A` cluster, which is
            // a switch on the type: `$86`-`$88` and `$8A`+ do **nothing at all**
            // (`cp $86 / ret nc`), while `$80`-`$85` and `$89` switch the anim
            // (`$FFD5` = slot +5) to 16, clear the flip byte and run `$863C`
            // (`$8240` sprite re-derive + `sObjectDone = 1`, i.e. the object
            // stops for this frame) — the slot is now an anim-16 pickup and the
            // next frames' `$586F5` handler plays the rise-and-despawn. The
            // order differs by type: `$85` (`$86DA`) writes the anim *before*
            // `$863C`, so its sprite is the anim-16 one; the rest (`$8695`) run
            // `$863C` first and write the anim after, so their sprite stays the
            // **old** anim's — the anim-16 handler never re-derives it. Either
            // way `$8240` clears `+14..+30` (the rise counter, the state gate
            // byte and the interaction flags) before the copy-back. The type
            // `$06` carry path has its own arm.
            let ty = st.b[B_TYPE];
            if ty & 0x80 != 0 {
                if (0x86..=0x88).contains(&ty) || ty >= 0x8A {
                    return;
                }
                st.b[B_FLIP] = 0;
                if ty == 0x85 {
                    st.b[B_DEF] = 16;
                    self.finish_sprite_swap(st, tables);
                } else {
                    self.derive_sprite(st, tables);
                    st.b[B_DEF] = 16;
                    self.done = 1;
                }
                return;
            }
            if ty != 0x06 {
                return;
            }
            self.note_unported(0x8611);
            return;
        }
        if f & 0x20 != 0 {
            self.dispatch_below(st, tables);
        }
    }

    /// `$85C4`: stomp (bit0) — `+15 = 0`, then defeat / anim swap.
    fn dispatch_stomp(&mut self, st: &mut Staging, tables: &ObjectTables) {
        st.b[B_FSTATE] = 0;
        if self.mario.spin_jump {
            self.defeat(st);
            return;
        }
        let ty = st.b[B_TYPE];
        if ty & 0x02 == 0 {
            self.defeat(st);
            return;
        }
        if ty & 0x01 == 0 {
            // $8633: sfx 3; anim + 1; sprite re-derive.
            st.b[B_DEF] = st.b[B_DEF].wrapping_add(1);
            self.finish_sprite_swap(st, tables);
        } else {
            // $85D7/$85DC: sfx 3; anim − 1.
            st.b[B_DEF] = st.b[B_DEF].wrapping_sub(1);
            self.finish_sprite_swap(st, tables);
        }
    }

    /// `$85F0`/`$85FC`: defeat — coin credit, clear anim/state/flags/type.
    fn defeat(&mut self, st: &mut Staging) {
        self.money_bag_add();
        st.b[B_DEF] = 0;
        st.b[B_FSTATE] = 0;
        st.b[B_DEFEAT] = 0;
        st.b[B_TYPE] = 0;
        self.done = 1;
        self.clear_stage_flags = true;
    }

    /// `$86A0`: hit from below (bit5) — the flag `$2F72` sets from the ceiling
    /// probe's bonk point ([`ObjectSim::below_bonk`]).
    ///
    /// The type's high bit splits the arms. A bit-7 type does **not** touch
    /// `+15` here: `$86E4` re-checks the low seven bits and either takes
    /// `$8721` (`$7F` non-zero — the bell `$86`, the `$87`/`$88` poses, the
    /// `$85` marker) or, for `$80` alone, `$8708`. `$8721` holds the type in
    /// `sSavedObjectType` across `$863C`, because `$8240` clears `+14..+30`
    /// (the type's own byte included), then arms the *revive* reaction
    /// (`+27 = 2`): `$5816C`'s landing branch re-derives the sprite and drops
    /// the object into frame state 2/3. `$8708` is the item pose's collect
    /// arm — anim `$11` with a powerup first rewrites the spawn record's anim
    /// byte to 18 (`ptr+3` = record+5, so a later re-spawn is the anim-18
    /// pose) and stages anim 18 / sprite 26 — then type = anim + `$70`,
    /// `+15 = 1`, Y += 16 and `$FFD2 = $10` (sfx 8 is audio-only).
    ///
    /// The plain types take `+15 = 0`, then `$86C6`'s anim bump for the `$10`
    /// class (bit3 set → anim − 1 via `$85D7`/`$85DC`, else anim + 1 via
    /// `$8633`) or the `$85F0` defeat for the `$08` class; the rest just
    /// re-derive the sprite (`$863C`, sfx 6).
    fn dispatch_below(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.clear_stage_flags = true;
        let ty = st.b[B_TYPE];
        if ty & 0x80 != 0 {
            // $86E4.
            if ty & 0x7F != 0 {
                // $8721: the type survives the re-derive, then reaction 2.
                let saved_type = ty;
                self.finish_sprite_swap(st, tables);
                st.b[B_TYPE] = saved_type;
                st.b[B_HITREACT] = 2;
                return;
            }
            // $86E4's fall-through — the `$80` item pose.
            if st.b[B_DEF] == 0x11 && self.mario.powerup != 0 {
                self.record_set_anim(st, 18);
                st.b[B_DEF] = 18;
                st.b[B_SPRITE] = 26;
            }
            // $8708: type = anim + $70, frame state 1, Y += 16, marker $10.
            st.b[B_TYPE] = st.b[B_DEF].wrapping_add(0x70);
            st.b[B_FSTATE] = 1;
            self.mv_y_add(st, 16);
            st.b[B_MARKER] = 0x10;
            return;
        }
        st.b[B_FSTATE] = 0;
        if ty & 0x10 != 0 {
            // $86C6.
            if ty & 0x08 != 0 {
                // $86D2 → $85D7/$85DC: sfx 3, anim − 1, sprite re-derive.
                st.b[B_DEF] = st.b[B_DEF].wrapping_sub(1);
                self.finish_sprite_swap(st, tables);
                st.b[B_HITREACT] = 1;
                return;
            }
            st.b[B_DEF] = st.b[B_DEF].wrapping_add(1);
            self.finish_sprite_swap(st, tables);
            st.b[B_HITREACT] = 1;
            return;
        }
        if ty & 0x08 != 0 {
            self.defeat(st);
            return;
        }
        self.finish_sprite_swap(st, tables);
        st.b[B_HITREACT] = 1;
    }

    // ---------------------------------------------------------------------
    // $2F72 — the below-hit pass (the block-bump slot alloc's tail)
    // ---------------------------------------------------------------------

    /// `$2F72`: after a ceiling bonk allocated a bump slot (`$5960`), flag
    /// every object whose staged screen box contains the bonk point as hit
    /// from below (`+23` bit5) — the flag `$85AA` turns into `$86A0` later in
    /// the same frame's object pass, so this runs during the player update,
    /// before the object engine.
    ///
    /// `$1DFE` left `$FFB7`/`$FFB9` at the probe's *world* point; the pass
    /// first rewrites them into screen coords — `(Y & $F0) + 8 − sScrollY −
    /// 16` and `((X + 8) & $F0) − sScrollX − 8`, both 8-bit — and boxes each
    /// slot's own staged screen pair `+9/+10` against them: |dy| < `$10`,
    /// |dx| < `$14` (modular differences, like the interaction pass). Slots
    /// with any `+23` flag set, or a zero type, are skipped — a touched
    /// object has already run its own `$8611` arm.
    ///
    /// When the hit object is a type-`$80` pose the pass also raises
    /// `sObjectBumpSpecial`, which `$19D2` reads to *withhold* the `$01`
    /// bonk's coin and its type-4 slot. The flag is returned here rather than
    /// stored: the port resolves those two halves across its module seam
    /// (`PlayerSim::resolve_bump_coin` owns the credit, this owns the slots).
    ///
    /// `sObjectBumpHit` (`$A247`, read only by the fireball's `$2FED` pass)
    /// is not modelled.
    pub fn below_bonk(&mut self, bonk_y: u8, bonk_x: u8, scroll_y: u8, scroll_x: u8) -> bool {
        let by = (bonk_y & 0xF0)
            .wrapping_add(8)
            .wrapping_sub(scroll_y)
            .wrapping_sub(16);
        let bx = bonk_x.wrapping_add(8) & 0xF0;
        let bx = bx.wrapping_sub(scroll_x).wrapping_sub(8);
        let mut special = false;
        for s in self.slots.iter_mut() {
            if s.b[B_FLAGS] != 0 {
                continue;
            }
            let ty = s.b[B_TYPE];
            if ty == 0 {
                continue;
            }
            if s.b[B_SCRY].wrapping_sub(by) >= 0x10 {
                continue;
            }
            if s.b[B_SCRX].wrapping_sub(bx) >= 0x14 {
                continue;
            }
            s.b[B_FLAGS] |= 0x20;
            if ty == 0x80 {
                special = true;
            }
        }
        special
    }

    // ---------------------------------------------------------------------
    // home.asm $2D7D — the Mario interaction pass
    // ---------------------------------------------------------------------

    /// `$2D7D`: per-slot box test + response (runs before the object engine).
    fn collide(&mut self, cam_x: u16, cam_y: u16, layout: &LevelLayout) {
        self.bounce = false;
        self.stomp_jump = false;
        self.stomp_bounce = false;
        self.on_object = false;
        self.on_ground_cleared = false;
        self.mario_y_adjust = None;
        if self.mario_dead != 0 {
            return;
        }
        let mario_sx = self.mario.x.wrapping_sub(cam_x).wrapping_add(96) as u8;
        let mario_sy = self.mario.y.wrapping_sub(cam_y).wrapping_add(98) as u8;
        for i in 0..SLOT_COUNT {
            if self.slots[i].b[B_FLAGS] & 0xF7 != 0 {
                continue;
            }
            // $2EDD: type gate.
            let ty = self.slots[i].b[B_TYPE];
            if ty & 0x80 != 0 {
                if ty & 0x7F == 0 {
                    continue;
                }
            } else if ty & 0x63 == 0 {
                continue;
            }
            // $2EF5: box test against the slot's screen position.
            let b = &self.slots[i].b;
            let obj_y = b[B_SCRY];
            let obj_x = b[B_SCRX];
            let mut e = if self.mario.powerup != 0 { 8u8 } else { 0 };
            if self.mario.ducking {
                e = if self.mario.powerup != 0 { 0 } else { 0xF8 };
            }
            let d = b[B_HB_TOP].wrapping_add(obj_y).wrapping_sub(20);
            let span = b[B_HB_BOTTOM]
                .wrapping_add(obj_y)
                .wrapping_add(e)
                .wrapping_sub(d);
            let y_delta = mario_sy.wrapping_sub(d);
            if y_delta >= span {
                continue;
            }
            let d = b[B_HB_LEFT]
                .wrapping_add(obj_x)
                .wrapping_add(11)
                .wrapping_sub(16);
            let span = b[B_HB_RIGHT]
                .wrapping_add(obj_x)
                .wrapping_add(20)
                .wrapping_sub(16)
                .wrapping_sub(d);
            if mario_sx.wrapping_sub(d) >= span {
                continue;
            }
            self.respond(i, y_delta, layout);
        }
        if self.bounce {
            // $2D7D tail: turn the stomp into a jump (75 with A held).
            self.stomp_jump = true;
        }
    }

    /// `$2DC6`: write the interaction flags and apply the response.
    fn respond(&mut self, i: usize, y_delta: u8, layout: &LevelLayout) {
        self.on_ground_cleared = true;
        self.slots[i].b[B_FLAGS] |= 0x10;
        let ty = self.slots[i].b[B_TYPE];
        if ty & 0x80 != 0 {
            // $2DC6 tail: `and $7F; ld [sPendingItemId], a; ret` — the id is
            // consumed by `$3451` at the tail of the next frame's player update.
            // (Note the engine does *not* clear the byte here, and `$3451`
            // itself returns without clearing for id 0, so a `$80` object — the
            // levitating item pose, the bobber at rest — has no effect at all.)
            self.pending_item = ty & 0x7F;
            return;
        }
        if y_delta >= 0x0C || (self.mario.flutter == 0 && self.mario.vvel >= 0x18) {
            self.harm_or_star(i);
            return;
        }
        let a = ty & 0x23;
        if a == 0 {
            self.harm_or_star(i);
            return;
        }
        if a & 0x20 != 0 {
            // Stompable-on-top: stand on it (nudge Mario up 1 px).
            self.on_object = true;
            self.slots[i].b[B_FLAGS] = 8;
            if y_delta < 3 {
                return;
            }
            self.mario_y_adjust = Some(self.mario.y.wrapping_sub(1));
            return;
        }
        // $2E23: bounce path.
        if self.mario.star_high != 0 {
            self.harm_or_star(i);
            return;
        }
        self.slots[i].b[B_FLAGS] = 1;
        let b = y_delta.wrapping_add(6);
        let y_new = self.mario.y.wrapping_sub(b as u16);
        self.mario_y_adjust = Some(y_new);
        self.bounce = true;
        // $1DFE ceiling check: it probes $FFC0, which the `y -= b` above has
        // already rewritten — so the probe runs from the *shifted* Y, and a
        // hit adds b back.
        let px = self.mario.x.wrapping_add(16);
        let py = y_new.wrapping_add(if self.mario.powerup != 0 { 18 } else { 28 });
        let t = Self::probe_bias(layout, px, py);
        if t == 0x7E || t < 0x38 {
            self.mario_y_adjust = Some(self.mario.y);
        }
        if self.mario.scene_state != 0 {
            self.stomp_bounce = true;
        }
    }

    /// `$2E6F`/`$2EAA`: harmful contact or star defeat.
    fn harm_or_star(&mut self, i: usize) {
        if self.mario.star_high != 0 {
            // $2EAA: star defeat (type bit5 → immune).
            if self.slots[i].b[B_TYPE] & 0x20 != 0 {
                return;
            }
            self.slots[i].b[B_FLAGS] = 4;
            self.stomp_bounce_timer = 32;
            let (v, _) = bcd_add(self.stomp_chain, 1);
            self.stomp_chain = v;
            return;
        }
        if self.slots[i].b[B_TYPE] & 0x40 == 0 {
            return;
        }
        if self.mario.moon_physics & 0x04 != 0 {
            self.note_unported(0x2E8E);
            return;
        }
        if self.mario.invincibility != 0 {
            return;
        }
        self.mario_dead = 15;
    }

    // ---------------------------------------------------------------------
    // $8C31 — spawn scan
    // ---------------------------------------------------------------------

    /// `$8C56`: step the stream pointer to the next record, or refuse.
    ///
    /// The stream is terminated by a `$FF` first byte one record past the last
    /// one (`$8C56`: `add hl, de / ld a, [hl] / cp $FF / ret z` — the `ret`
    /// happens BEFORE the pointer is stored), so consuming the LAST record
    /// leaves `sObjectStreamPtr` parked on it, not one record further. Measured
    /// on `traj12`: the console holds `$AB8A` (the last record, x=4008) for the
    /// rest of the level after that record's spawn, while advancing made the
    /// port read `$AB90` (the terminator slot) and diverge from the spawn frame
    /// on. Returns false when the scan is finished (the caller must stop).
    fn advance_stream(&mut self) -> bool {
        if self.stream_index + RECORD_LEN < self.records.len() * RECORD_LEN {
            self.stream_index += RECORD_LEN;
            return true;
        }
        false
    }

    /// Walk the stream for records at the 8-aligned spawn edge.
    fn scan(&mut self, tables: &ObjectTables) {
        let n = self.records.len();
        if self.stream_index >= n * RECORD_LEN {
            return;
        }
        let forward = self.scan_dir == 1;
        // Bounded (the engine's exits are the same; this only guards the
        // port against a malformed stream).
        for _ in 0..(n * 2 + 8) {
            let ri = self.stream_index / RECORD_LEN;
            if ri >= n {
                return;
            }
            let rec_x = self.records[ri].x;
            let edge = self.spawn_x;
            if forward {
                if rec_x >> 8 != edge >> 8 {
                    if rec_x > edge {
                        return; // past the edge
                    }
                    if !self.advance_stream() {
                        return;
                    }
                    continue;
                }
                if (rec_x & 0xFF) < (edge & 0xFF) {
                    if !self.advance_stream() {
                        return;
                    }
                    continue;
                }
                if (rec_x & 0xFF) != (edge & 0xFF) {
                    return; // between records
                }
            } else {
                // backward mirror: the engine walks the pointer down
                if rec_x >> 8 != edge >> 8 {
                    if rec_x < edge {
                        return;
                    }
                    if self.stream_index == 0 {
                        return;
                    }
                    self.stream_index -= RECORD_LEN;
                    continue;
                }
                if (rec_x & 0xFF) > (edge & 0xFF) {
                    if self.stream_index == 0 {
                        return;
                    }
                    self.stream_index -= RECORD_LEN;
                    continue;
                }
                if (rec_x & 0xFF) != (edge & 0xFF) {
                    return;
                }
            }
            // Record at the edge: the mode byte gates (and the engine's
            // post-spawn marker 2 blocks re-spawns on a later pass).
            let m = self.records[ri].mode_flag;
            let skip = if self.easy_mode {
                m & 0x0F != 0
            } else {
                m & 0x0E != 0
            } || self.spawned[ri];
            if skip {
                if !self.advance_stream() {
                    return;
                }
                continue;
            }
            let free = self.slots.iter().position(|s| s.b[B_STATE] == 0);
            let Some(slot) = free else {
                return; // no free slot: scan stops ($8C93 → $AF)
            };
            let r = self.records[ri];
            // $8CA8: 6 record bytes, record pointer (+2), state = 1, clear
            // +9..+31, marker 2, $8240 sprite derive, $81CF hitbox.
            let mut s = Staging {
                b: [0; 32],
                ..Default::default()
            };
            s.b[B_XHI] = (r.x >> 8) as u8;
            s.b[B_XLO] = r.x as u8;
            s.b[B_MARKER] = r.mode_flag;
            s.b[B_YHI] = (r.y >> 8) as u8;
            s.b[B_YLO] = r.y as u8;
            s.b[B_DEF] = r.anim_index;
            // The engine stores (streamPtrHi, streamPtrLo + 2) with no carry
            // into the hi byte.
            let a = self.stream_index + STREAM_BASE as usize;
            s.b[B_RECHI] = (a >> 8) as u8;
            s.b[B_RECLO] = (a as u8).wrapping_add(2);
            s.b[B_STATE] = 1;
            self.derive_sprite(&mut s, tables);
            self.refresh_hitbox(&mut s, tables);
            self.slots[slot].b = s.b;
            self.spawned[ri] = true;
            self.standby = self.standby.wrapping_add(1);
            // The last record leaves the pointer parked on itself (see
            // `advance_stream`): the engine's `$FF` terminator check refuses
            // the store, and the re-scan then walks into the same record with
            // the post-spawn marker set and stops there every frame.
            self.advance_stream();
        }
    }
}

/// BCD add (the engine uses `add`/`inc` + `daa`).
fn bcd_add(v: u8, add: u8) -> (u8, bool) {
    let mut lo = (v & 0x0F) + (add & 0x0F);
    let mut hi = (v >> 4) + (add >> 4);
    if lo > 9 {
        lo -= 10;
        hi += 1;
    }
    if hi > 9 {
        hi -= 10;
        return ((hi << 4) | lo, true);
    }
    ((hi << 4) | lo, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{SpawnRecord, LAYOUT_SIZE};
    use crate::object::AnimProfile;

    fn prof(handler: u32) -> AnimProfile {
        AnimProfile {
            handler,
            type_sets: Vec::new(),
            states: Vec::new(),
        }
    }

    /// Minimal mode-A tables with level 0's item wiring: anim 16 is the pickup
    /// handler, 18/20 are the item poses; each anim gets its own sprite id so
    /// the touch arm's derive order is observable. Anims 17 (the `$11` hopper)
    /// and 22 (the `$86` pose) carry their real handlers/sprites for the
    /// below-hit arm.
    fn tables() -> ObjectTables {
        let mut profiles = vec![prof(0); 96];
        profiles[15] = prof(0x586E1);
        profiles[16] = prof(0x586F5);
        profiles[17] = prof(0x58712);
        profiles[18] = prof(0x58758);
        profiles[20] = prof(0x58755);
        profiles[22] = prof(0x587BD); // the `$86` pose (anim 22)
        let mut sprites = vec![0u8; 96];
        sprites[16] = 30; // the anim-16 pickup sprite
        sprites[17] = 25; // the `$11` hopper's sprite
        sprites[18] = 26; // the item pose's sprite
        sprites[22] = 31; // the `$86` pose's sprite
        ObjectTables {
            schema: String::new(),
            notes: Vec::new(),
            anim_to_sprite_a: sprites,
            anim_to_sprite_b: Vec::new(),
            hitbox_records_a: vec![[0; 4]],
            hitbox_records_b: Vec::new(),
            hitbox_index_a: vec![0; 96],
            hitbox_index_b: Vec::new(),
            draw_records: Vec::new(),
            draw_index: Vec::new(),
            sprite_mappings_a: Vec::new(),
            sprite_mappings_b: Vec::new(),
            anim_profiles_a: profiles,
            anim_profiles_b: Vec::new(),
            player_mappings: Vec::new(),
        }
    }

    fn blank_layout() -> LevelLayout {
        LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap()
    }

    /// A staged copy carrying the `$2DC6` touch flag (bit 4) and the given
    /// type/anim.
    fn touched(anim: u8, ty: u8) -> Staging {
        let mut st = Staging::default();
        st.b[B_DEF] = anim;
        st.b[B_TYPE] = ty;
        st.b[B_FLAGS] = 0x10;
        st
    }

    /// `$8695` order: for `$80`-`$84`/`$89` the flip clears and `$863C` runs
    /// while the anim is still the old one — the sprite stays the **old**
    /// anim's (the anim-16 handler never re-derives it) — and only then does
    /// `$FFD5` become 16. `$8240` wipes `+14..+30`, which is what consumes the
    /// touch flag and resets the rise counter; `sObjectDone = 1` stops the slot
    /// for this frame.
    #[test]
    fn touch_switches_to_anim16_keeping_the_old_sprite() {
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        let mut st = touched(18, 0x80);
        st.b[B_T24] = 0x0A;
        st.b[B_FLASH] = 0x20;
        st.b[B_FSTATE] = 2;
        obj.interaction_dispatch(&mut st, &t);
        assert_eq!(st.b[B_DEF], 16, "the anim switched");
        assert_eq!(st.b[B_SPRITE], 26, "the sprite is the old anim's");
        assert_eq!(st.b[B_FLIP], 0);
        assert_eq!(st.b[B_T24], 0, "$8240 cleared +14..+30");
        assert_eq!(st.b[B_FLASH], 0);
        assert_eq!(st.b[B_FSTATE], 0);
        assert_eq!(st.b[B_FLAGS], 0, "the touch flag was consumed");
        assert_eq!(obj.done, 1, "$863C sets sObjectDone");
        assert!(obj.unported.is_empty());
        // $89 takes the same fall-through as $80-$84.
        let mut st = touched(18, 0x89);
        obj.interaction_dispatch(&mut st, &t);
        assert_eq!((st.b[B_DEF], st.b[B_SPRITE]), (16, 26));
    }

    /// `$86DA` order: type `$85` writes the anim *before* `$863C`, so its
    /// sprite is the anim-16 one.
    #[test]
    fn touch_type85_derives_the_sprite_from_anim16() {
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        let mut st = touched(15, 0x85);
        obj.interaction_dispatch(&mut st, &t);
        assert_eq!(st.b[B_DEF], 16);
        assert_eq!(st.b[B_SPRITE], 30, "map[16]: the anim was written first");
        assert_eq!(st.b[B_FLIP], 0);
        assert_eq!(obj.done, 1);
    }

    /// The `$868A` switch's inert arm: `$86`-`$88` and `$8A`+ return without
    /// touching the object (their pickup/pose behavior lives in their own
    /// handlers). The bit-7-clear type-`$06` carry path is still unported.
    #[test]
    fn touched_86_to_88_and_8a_plus_are_inert() {
        let t = tables();
        for ty in [0x86u8, 0x87, 0x88, 0x8A, 0x90, 0xC0, 0xFF] {
            let mut obj = ObjectSim::new(Vec::new(), 0);
            let mut st = touched(22, ty);
            st.b[B_T24] = 0x0A;
            obj.interaction_dispatch(&mut st, &t);
            assert_eq!(st.b[B_DEF], 22, "type ${ty:02X} keeps its anim");
            assert_eq!(st.b[B_T24], 0x0A, "and its scratch");
            assert_eq!(obj.done, 0);
            assert!(obj.unported.is_empty());
        }
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut st = touched(1, 0x06);
        obj.interaction_dispatch(&mut st, &t);
        assert_eq!(obj.unported, vec![0x8611]);
        assert_eq!(st.b[B_DEF], 1);
    }

    /// `$586F5` per call: `+15 = 1`, the `$5B68B` flash on even `$FFF0`, Y += 2
    /// and `+24` += 2; the first 15 calls never remove.
    #[test]
    fn pickup16_rises_steps_the_counter_and_flashes_on_even_frames() {
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.fff0 = 0; // even: the flash toggles
        let mut st = Staging::default();
        st.b[B_STATE] = 2;
        set_y(&mut st.b, 100);
        obj.handler_pickup16(&mut st, &t);
        assert_eq!(st.b[B_FSTATE], 1);
        assert_eq!(y_of(&st.b), 102);
        assert_eq!(st.b[B_T24], 2);
        assert_eq!(st.b[B_FLASH], 0x10);
        assert_eq!(obj.done, 0);
        obj.fff0 = 1; // odd: no flash
        obj.handler_pickup16(&mut st, &t);
        assert_eq!(st.b[B_FLASH], 0x10);
        assert_eq!(y_of(&st.b), 104);
        assert_eq!(st.b[B_T24], 4);
        assert_eq!(obj.done, 0);
    }

    /// The 16th call (`+24` reaching `$20`) removes the slot in remove-mode 2:
    /// the slot zeroes and the active count drops, but — unlike mode 0/1 — the
    /// record marker survives and no coin/money-bag credit runs.
    #[test]
    fn pickup16_sixteenth_call_despawns_without_credit_or_marker_clear() {
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.active = 1;
        obj.spawned = vec![true]; // a stream-backed record
        let mut st = Staging::default();
        st.b[B_STATE] = 2;
        st.b[B_RECHI] = 0xAB; // $AB06: record 0 of the buffer
        st.b[B_RECLO] = 0x06;
        st.b[B_T24] = 0x1E; // one more call reaches $20
        obj.handler_pickup16(&mut st, &t);
        assert_eq!(obj.done, 2);
        assert_eq!(st.b, [0u8; 32], "the slot zeroes");
        assert_eq!(obj.active, 0);
        assert_eq!(obj.money_bag, 0, "no coin credit in remove-mode 2");
        assert!(obj.spawned[0], "mode 2 keeps the record marker");
        assert_eq!(obj.remove_mode, 0, "$5B2D7 resets the mode");
    }

    /// The full handshake through the real entry points: frame 1's `$2DC6`
    /// stages the mushroom id (type `$81` → `$01`) off the touch and the object
    /// pass switches the slot to the anim-16 pickup; the following frames run
    /// `$586F5` and the item despawns 31 engine frames after the touch.
    #[test]
    fn stepped_mushroom_touch_stages_the_id_and_despawns() {
        let t = tables();
        let layout = blank_layout();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.active = 1;
        obj.spawned = vec![true];
        {
            let s = &mut obj.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_DEF] = 18;
            s.b[B_TYPE] = 0x81; // bit 7 set, low 7 bits = id 1 (mushroom)
            s.b[B_RECHI] = 0xAB;
            s.b[B_RECLO] = 0x06;
            set_x(&mut s.b, 100);
            set_y(&mut s.b, 100);
            s.b[B_SCRY] = 99; // the staged screen pos the box test reads
            s.b[B_SCRX] = 96;
        }
        let mario = MarioContact {
            x: 0,
            y: 0,
            ..Default::default()
        };
        obj.step(0, 0, &layout, &t, &mario);
        assert_eq!(obj.pending_item, 1, "$2DC6 stages type & $7F");
        assert_eq!(obj.slots[0].b[B_FLAGS], 0, "the touch flag was consumed");
        assert_eq!(obj.slots[0].b[B_DEF], 16, "the touch arm switched anim 16");
        assert_eq!(obj.slots[0].b[B_SPRITE], 26);
        // The next frame is the pickup handler's first call.
        obj.step(0, 0, &layout, &t, &mario);
        assert_eq!(obj.slots[0].b[B_T24], 2);
        assert_eq!(y_of(&obj.slots[0].b), 102);
        assert_eq!(obj.slots[0].b[B_FSTATE], 1);
        // The parity gate halves the cadence; by 31 engine frames it is gone.
        for _ in 0..40 {
            obj.step(0, 0, &layout, &t, &mario);
        }
        assert_eq!(obj.slots[0].b[B_STATE], 0, "the item despawned");
        assert_eq!(obj.active, 0);
        assert_eq!(obj.money_bag, 0, "a pickup is not a coin");
        assert!(obj.spawned[0], "remove-mode 2 keeps the record marker");
        assert!(obj.unported.is_empty(), "no unported handler was reached");
    }

    /// Pin the extraction wiring: level 0's anim 16 resolves to `$586F5` in the
    /// committed `extracted/objects.json`, in both modes, and `run_handler`
    /// routes it to a ported body (no `unported` note). Skips when the
    /// extraction is not built.
    #[test]
    fn extracted_anim16_profile_points_at_the_pickup_handler() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json");
        let Ok(txt) = std::fs::read_to_string(path) else {
            eprintln!("skip: {path} not built");
            return;
        };
        let t: ObjectTables = serde_json::from_str(&txt).expect("objects.json parses");
        assert_eq!(t.behavior_for(0, 16), Some(0x586F5));
        assert_eq!(t.behavior_for(0x0B, 16), Some(0x586F5), "mode B too");
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        let mut st = Staging::default();
        st.b[B_DEF] = 16;
        obj.run_handler(&mut st, &blank_layout(), &t);
        assert!(obj.unported.is_empty(), "anim 16 must have a ported body");
    }

    /// `$EC40`'s sprite-map mode rule — `ld a,[sCurLevel] / cp $0A / ret c`, so
    /// level `$0A` itself already uses the "B" pair. The port had `> 0x0A`, which
    /// put level `$0A` in mode A: one level's worth of objects and anim profiles
    /// read from the wrong table.
    #[test]
    fn the_sprite_map_mode_rule_is_inclusive_at_0a() {
        assert_eq!(crate::object::sprite_map_index(0x00), 0);
        assert_eq!(crate::object::sprite_map_index(0x09), 0);
        assert_eq!(
            crate::object::sprite_map_index(0x0A),
            1,
            "$EC52: cp $0A / ret c — $0A is mode B"
        );
        assert_eq!(crate::object::sprite_map_index(0x1F), 1);
    }

    /// `$58100` is a bare `ret` — the ROM byte is `$C9`, immediately after the
    /// 192-byte mode-A table at `$58040` — and nineteen mode-A anims plus eleven
    /// mode-B ones dispatch to it. It is a *fixture*, not a gap: the `$8000`
    /// prelude still runs, only the per-anim body does nothing. Routing it
    /// explicitly is what keeps it out of `unported`, where it would read as
    /// port debt and could turn into a spurious gate `note:`.
    #[test]
    fn the_58100_no_op_is_dispatched_and_not_flagged() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json");
        let Ok(txt) = std::fs::read_to_string(path) else {
            eprintln!("skip: {path} not built");
            return;
        };
        let t: ObjectTables = serde_json::from_str(&txt).expect("objects.json parses");
        // One of the nineteen from each mode's index list.
        assert_eq!(t.behavior_for(0, 5), Some(0x58100), "mode A anim 5");
        assert_eq!(t.behavior_for(0x0B, 40), Some(0x58100), "mode B anim 40");

        for (level, anim) in [(0u8, 5u8), (0x0B, 40)] {
            let mut obj = ObjectSim::new(Vec::new(), 0);
            obj.configure(level);
            let mut st = Staging::default();
            st.b[B_DEF] = anim;
            obj.run_handler(&mut st, &blank_layout(), &t);
            assert!(
                obj.unported.is_empty(),
                "level {level} anim {anim}: the no-op must not be flagged"
            );
        }
    }

    // -----------------------------------------------------------------
    // $2F72 / $86A0 — the below-hit system
    // -----------------------------------------------------------------

    /// A stream-backed staging (record pointer `+6/+7` = `$AB06`, record 0).
    fn streamed(anim: u8, ty: u8, flags: u8) -> Staging {
        let mut st = Staging::default();
        st.b[B_STATE] = 2;
        st.b[B_DEF] = anim;
        st.b[B_TYPE] = ty;
        st.b[B_FLAGS] = flags;
        st.b[B_RECHI] = 0xAB;
        st.b[B_RECLO] = 0x06;
        st
    }

    /// One spawn record with a distinctive anim, so the `$86E4` record write
    /// is observable.
    fn records() -> Vec<SpawnRecord> {
        vec![SpawnRecord {
            x: 352,
            y: 448,
            anim_index: 17,
            mode_flag: 0,
            sprite_id: 25,
            hitbox: [0; 4],
            raw: [0x01, 0x60, 0x00, 0x01, 0xC0, 0x11],
        }]
    }

    /// `$86E4` → `$8721`: a bit-7 type with non-zero low bits (the `$86` bell,
    /// the `$87`/`$88` poses, `$8A`+) re-derives its sprite with the type held
    /// across `$8240`'s `+14..+30` clear, then arms the *revive* reaction
    /// (`+27 = 2`) that `$5816C`'s landing branch turns back into a live
    /// object. No anim swap, no frame state of its own — and no `unported`
    /// note any more.
    #[test]
    fn below_bit7_types_re_derive_and_arm_the_revive_reaction() {
        let t = tables();
        for ty in [0x86u8, 0x87, 0x88, 0x8A, 0x90, 0xC0, 0xFF] {
            let mut obj = ObjectSim::new(records(), 0);
            obj.configure(0);
            let mut st = streamed(22, ty, 0x20);
            st.b[B_FSTATE] = 1;
            st.b[B_FLASH] = 0x10;
            set_x(&mut st.b, 352);
            set_y(&mut st.b, 448);
            obj.dispatch_below(&mut st, &t);
            assert!(obj.clear_stage_flags, "sClearStageFlags = 1");
            assert_eq!(st.b[B_TYPE], ty, "type ${ty:02X} survives the re-derive");
            assert_eq!(st.b[B_HITREACT], 2, "reaction 2 = $5819B's revive");
            assert_eq!(st.b[B_FSTATE], 0, "$8240 cleared +14..+30");
            assert_eq!(st.b[B_FLAGS], 0, "the below-hit flag was consumed");
            assert_eq!(st.b[B_FLASH], 0);
            assert_eq!(st.b[B_DEF], 22, "the anim is untouched");
            assert_eq!(st.b[B_SPRITE], 31, "map[22] re-derived");
            assert_eq!(obj.done, 1, "$863C sets sObjectDone");
            assert!(obj.unported.is_empty(), "the $86E4 cluster is ported");
        }
    }

    /// `$86E4`'s fall-through (type `$80` alone): the item pose. Anim `$11`
    /// (the hopper's pose) with a powerup takes the collect arm — the spawn
    /// record's anim byte is rewritten to 18 (`ptr+3` = record+5, so a later
    /// re-spawn is the anim-18 pose) and the staged anim/sprite follow — then
    /// `$8708` sets type = anim + `$70`, `+15 = 1`, Y += 16 and `+2 = $10`.
    #[test]
    fn below_pose80_with_a_powerup_collects_the_pose_and_rewrites_the_record() {
        let t = tables();
        let mut obj = ObjectSim::new(records(), 0);
        obj.configure(0);
        obj.mario.powerup = 1;
        let mut st = streamed(0x11, 0x80, 0);
        set_y(&mut st.b, 448);
        obj.dispatch_below(&mut st, &t);
        assert_eq!(obj.records[0].anim_index, 18, "the record's anim byte");
        assert_eq!(st.b[B_DEF], 18, "$FFD5 = 18");
        assert_eq!(st.b[B_SPRITE], 26, "$86E4 stages sprite 26");
        assert_eq!(st.b[B_TYPE], 0x82, "anim 18 + $70");
        assert_eq!(st.b[B_FSTATE], 1);
        assert_eq!(y_of(&st.b), 464, "$3A64: Y += 16");
        assert_eq!(st.b[B_MARKER], 0x10, "$FFD2 = $10");
        assert_eq!(st.b[B_HITREACT], 0, "this arm arms no reaction");
        assert_eq!(obj.done, 0, "no $863C on this arm");
        assert!(obj.unported.is_empty());
    }

    /// The same bit-7 type without a powerup (and any anim other than `$11`):
    /// the collect arm is skipped — record, anim and sprite stay — and only
    /// `$8708`'s pose swap runs.
    #[test]
    fn below_pose80_without_the_powered_11_pose_just_swaps_the_type() {
        let t = tables();
        for (anim, powerup) in [(0x11u8, 0u8), (18, 1), (18, 0)] {
            let mut obj = ObjectSim::new(records(), 0);
            obj.configure(0);
            obj.mario.powerup = powerup;
            let mut st = streamed(anim, 0x80, 0);
            obj.dispatch_below(&mut st, &t);
            assert_eq!(obj.records[0].anim_index, 17, "the record is untouched");
            assert_eq!(st.b[B_DEF], anim);
            assert_eq!(st.b[B_SPRITE], 0, "no sprite staged");
            assert_eq!(st.b[B_TYPE], anim.wrapping_add(0x70));
            assert_eq!(st.b[B_FSTATE], 1);
            assert_eq!(y_of(&st.b), 16, "the stock y=0 + 16");
            assert_eq!(st.b[B_MARKER], 0x10);
            assert_eq!(obj.done, 0);
        }
    }

    /// The plain (bit-7-clear) arms: `+15 = 0` first, then `$86C6`'s anim bump
    /// for the `$10` class — bit3 clear → `$8633` (anim + 1), bit3 set →
    /// `$85D7`/`$85DC` (anim − 1, sprite re-derived) — and the `$85F0` defeat
    /// for the `$08` class; anything else just re-derives (`$863C`) and takes
    /// reaction 1.
    #[test]
    fn below_plain_types_bump_the_anim_or_defeat() {
        let t = tables();
        // $10 class, bit3 clear: anim + 1, re-derive, reaction 1.
        let mut obj = ObjectSim::new(records(), 0);
        let mut st = streamed(18, 0x10, 0x20);
        st.b[B_FSTATE] = 3;
        obj.dispatch_below(&mut st, &t);
        assert_eq!((st.b[B_DEF], st.b[B_SPRITE]), (19, 0));
        assert_eq!(st.b[B_FSTATE], 0);
        assert_eq!(st.b[B_HITREACT], 1);
        assert_eq!(obj.done, 1);
        // $10 class with bit3: $86D2's anim − 1 (and the sprite re-derive).
        let mut obj = ObjectSim::new(records(), 0);
        let mut st = streamed(20, 0x18, 0x20);
        obj.dispatch_below(&mut st, &t);
        assert_eq!((st.b[B_DEF], st.b[B_SPRITE]), (19, 0));
        assert_eq!(st.b[B_HITREACT], 1);
        assert_eq!(obj.done, 1);
        // $08 class: the defeat — coin credit, type/anim/state cleared.
        let mut obj = ObjectSim::new(records(), 0);
        let mut st = streamed(20, 0x08, 0x20);
        obj.dispatch_below(&mut st, &t);
        assert_eq!(st.b[B_TYPE], 0);
        assert_eq!(st.b[B_DEF], 0);
        assert_eq!(obj.money_bag, 1, "the defeat credits the coin counter");
        assert_eq!(obj.done, 1);
        // Neither bit: a plain re-derive + reaction 1 (sfx 6, audio-only).
        let mut obj = ObjectSim::new(records(), 0);
        let mut st = streamed(22, 0x02, 0x20);
        obj.dispatch_below(&mut st, &t);
        assert_eq!((st.b[B_DEF], st.b[B_SPRITE]), (22, 31));
        assert_eq!(st.b[B_HITREACT], 1);
        assert_eq!(obj.done, 1);
    }

    /// `$2F72`: the pass rewrites `$1DFE`'s world probe point into screen
    /// coords — `(Y & $F0) + 8 − sScrollY − 16`, `((X + 8) & $F0) − sScrollX −
    /// 8` — and boxes each slot's staged `+9/+10` against it (|dy| < `$10`,
    /// |dx| < `$14`).
    ///
    /// The traj15 geometry as the numbers: the `$01` block's cell is at world
    /// (352-367, 448-463), Mario's probe (360, 479), the camera (448, 377) →
    /// sScroll (368, 305) → the block's centre in screen bytes (248, 151) =
    /// `($F8, $97)`. Slots with a flag already set, or a zero type, are
    /// skipped; a type-`$80` hit raises `sObjectBumpSpecial`.
    #[test]
    fn below_bonk_marks_idle_objects_in_the_tile_window() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let put = |obj: &mut ObjectSim, i: usize, ty: u8, flags: u8, scry: u8, scrx: u8| {
            obj.slots[i].b[B_TYPE] = ty;
            obj.slots[i].b[B_FLAGS] = flags;
            obj.slots[i].b[B_SCRY] = scry;
            obj.slots[i].b[B_SCRX] = scrx;
        };
        put(&mut obj, 0, 0x86, 0, 0x97, 0xF8); // the bell pose on the tile
        put(&mut obj, 1, 0x80, 0, 0x9C, 0x04); // a pose, in the window
        put(&mut obj, 2, 0x4E, 0, 0x97, 0x60); // the walker, 100 px east
        put(&mut obj, 3, 0x81, 0x10, 0x97, 0xF8); // touched: owns its arm
        put(&mut obj, 4, 0, 0, 0x97, 0xF8); // no type
        put(&mut obj, 5, 0x4E, 0, 0xA7, 0xF8); // 16 px off in Y
        let special = obj.below_bonk(0xDF, 0x68, 0x31, 0x70);
        assert!(special, "the $80 pose raises sObjectBumpSpecial");
        assert_eq!(obj.slots[0].b[B_FLAGS], 0x20);
        assert_eq!(obj.slots[1].b[B_FLAGS], 0x20);
        assert_eq!(obj.slots[2].b[B_FLAGS], 0);
        assert_eq!(obj.slots[3].b[B_FLAGS], 0x10, "already flagged");
        assert_eq!(obj.slots[4].b[B_FLAGS], 0, "zero type");
        assert_eq!(obj.slots[5].b[B_FLAGS], 0, "$A7 is one window step out");
        // With no pose at the point the flag stays low.
        obj.slots[1].b[B_TYPE] = 0x86;
        obj.slots[1].b[B_FLAGS] = 0;
        obj.slots[0].b[B_FLAGS] = 0;
        assert!(!obj.below_bonk(0xDF, 0x68, 0x31, 0x70));
        assert_eq!(obj.slots[0].b[B_FLAGS], 0x20);
        assert_eq!(obj.slots[1].b[B_FLAGS], 0x20);
    }

    /// The whole handshake through the real entry points: `$2F72`'s pass
    /// flags the bell pose, the same frame's object engine turns the flag into
    /// `$8721` (sprite re-derived from anim 22, type held, `+27 = 2`), and the
    /// next calls run `$58101`'s frame-state-0 arm — face Mario, then the
    /// `$20` flip toggle — and the knockback (`$411A`: X + 1, Y − the bounce
    /// curve's first step 3, `+16` 0 → 1) on the slot-parity cadence.
    ///
    /// Measured on the console (`captures/probe_below_hit.py` pokes the same
    /// state into slot 0 and logs the 32 bytes per frame): sprite `$7F` →
    /// `$1F`, type `$86` kept, `+23` `$20` → 0, `+27` → 2, `+15` → 0, then
    /// `+15` 1 / `+13` `$20`, then (one parity frame later) (100,468) →
    /// (101,465) → (102,462).
    #[test]
    fn stepped_below_hit_on_the_bell_re_derives_and_faces_mario() {
        let t = tables();
        let layout = blank_layout();
        let mut obj = ObjectSim::new(records(), 0);
        obj.configure(0);
        obj.active = 1;
        {
            let s = &mut obj.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_DEF] = 22;
            s.b[B_TYPE] = 0x86;
            s.b[B_SPRITE] = 0x7F; // bogus on purpose: the re-derive shows
            s.b[B_RECHI] = 0xAB;
            s.b[B_RECLO] = 0x06;
            set_x(&mut s.b, 352);
            set_y(&mut s.b, 448);
            s.b[B_SCRY] = 0x97;
            s.b[B_SCRX] = 0xF8;
        }
        // Mario just west of the block, standing below it — the camera holds
        // the traj15 bonk point's frame.
        let mario = MarioContact {
            x: 336,
            y: 448,
            ..Default::default()
        };
        let special = obj.below_bonk(0xDF, 0x68, 0x31, 0x70);
        assert!(!special);
        assert_eq!(obj.slots[0].b[B_FLAGS], 0x20, "hit from below");
        obj.step(448, 377, &layout, &t, &mario);
        let s = &obj.slots[0];
        assert_eq!(s.b[B_TYPE], 0x86, "the bell keeps its type");
        assert_eq!(s.b[B_HITREACT], 2);
        assert_eq!(s.b[B_SPRITE], 31, "map[22]");
        assert_eq!(s.b[B_FLAGS], 0, "$818D cleared the flag");
        assert_eq!(s.b[B_FSTATE], 0, "$8240 cleared +14..+30");
        // The reaction's first arm: face Mario (west → state 1, flip 0) and
        // then the toggle → flip $20.
        obj.step(448, 377, &layout, &t, &mario);
        let s = &obj.slots[0];
        assert_eq!(s.b[B_FSTATE], 1);
        assert_eq!(s.b[B_FLIP], 0x20);
        assert_eq!(s.b[B_HITREACT], 2, "still owed: the revive clears it");
        // The $411A knockback arm — the console's (100,468) → (101,465),
        // i.e. X + 1 and Y − BOUNCE_SPEED[0] = 3 with +16 0 → 1.
        obj.step(448, 377, &layout, &t, &mario);
        let s = &obj.slots[0];
        assert_eq!((s.x(), s.y()), (353, 445));
        assert_eq!(s.b[B_PHASE], 1);
        assert_eq!(s.b[B_FSTATE], 1);
        assert!(obj.unported.is_empty());
    }
}
