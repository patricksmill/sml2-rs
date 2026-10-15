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
//! - Level-3 trio (`$59BC5` anim 88, `$594EB` anim 68, `$58DD1` anim 46): the
//!   ground walker (the `$7585` bob head + a 3-state walker: state 0 faces
//!   Mario with flash `$10`/type `$4E`, states 1/2 run a 16-idle + 48-step
//!   `+28`/`+25` counter machine and flip on the side wall probe through
//!   `$5D2D`); the `$582D1` body with type `$52`; and the carrying platform
//!   (type `$20`, waits for `+23` bit 3, then falls the `$7488` curve
//!   carrying Mario through `$3CDF` and removes itself on ground).
//! - Item-pose family (anims 18/19/20/25 → `$58755`/`$58758`, 22 → `$587BD`,
//!   23 → `$587D7`): item pose (`type $80`, done) until touched (`+23`
//!   bit4) → rise / spin (`$5B545`) / type-from-anim; anim 23 removes via
//!   `$5B2D7` mode 2 on the secret route. Anim 15 (`$586E1`) = type `$85`.
//!   The pickup arm (anim 16 → `$586F5`) is where the `$8611`/`$868A` touch
//!   switch sends a collected item: flash, Y += 2 and `+24` += 2 per call,
//!   remove-mode-2 despawn when `+24` reaches `$20`.
//! - Marker (`$586EE`, mode-B anims 27/28/29): type = anim + `$66` and
//!   return. The stream bytes are `$1B`/`$1C`/`$1D` — *not* `$27`-style
//!   values — so the types are `$81`/`$82`/`$83`, i.e. bit-7 item poses whose
//!   `$2DC6` tail stages powerup ids 1/2/3.
//! - Grower (anim 92 → `$5B1C8`): state 0 arms type `$4C` + counters
//!   `+24`/`+25`/`+26` = 3; state 1 walks 3 segments of three 1-px steps (up
//!   while `+16 == 0`), reloading `+24` from `+25` and flipping `+16` per
//!   segment, then drops the type to 0 into state 2; states 2/3 cycle the
//!   sprite id 122 → 124 on every 4th `$FFF0` frame, hold `+28` = 32 calls
//!   and walk it back to 122, jumping to the state-0 body to re-arm.
//! - Floaters (anims 94/95 → `$5B241`/`$5B28C`): the shared `$724A` state 0
//!   (type `$48`, state 1) and `$7266` tail; states 1/2 take one step per
//!   call — Y for 94, X for 95 — through the profile `+24` selects from the
//!   `$744D`/`$7473` pickers (`$7488`/`$74A1`, sentinel `$12`; 18 calls and
//!   32 px per leg), and the tail restarts `+16` at the sentinel, flipping
//!   the direction on every other leg's `+24` 0 → 1 edge (`$5D33`).
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

/// The fireball-vs-object pass's verdict (`$3007`): the ROM keeps it in
/// `sObjectHitResult`, and a non-zero value kills the ball.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireHit {
    /// No live object covered the ball's staged point — the ball flies on.
    None,
    /// A bit-2 type: the pass set `B_FLAGS` bit 1, which the interaction
    /// dispatch turns into the object's `defeat()`. No sfx of its own.
    Defeat,
    /// Any other live type: `sSfxRequest0 = 2` (the caller writes it) and the
    /// ball dies.
    Die,
}

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
/// `$74A1` — the `+24 == 0` step profile of the anim-94/95 floaters (`$7473`,
/// sentinel `$12`). One leg is 18 picker calls whose steps sum to 32 px; the
/// 19th byte is never reached (the caller's tail resets `+16` at the
/// sentinel). The `+24 != 0` side is `$7488`, whose base is `FALL_SPEED`
/// (its c=`$15` fall consumer reads 22 of the same bytes) — shared, so the
/// two consumers cannot drift.
const STEP_74A1: [u8; 19] = [
    0x03, 0x02, 0x03, 0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x01,
    0x00, 0x00, 0x00,
];

/// `$74AB` (`$5B465`'s table, c = 8): the defeated enemy's rise curve, 2× per
/// frame in `$581CE`. `$74A1`/`$74A7` are its neighbours, used by the floaters.
const DEATH_RISE_74AB: [u8; 8] = [0x02, 0x01, 0x02, 0x01, 0x01, 0x01, 0x00, 0x00];

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
    pub fn active(&self) -> bool {
        self.state() == SlotState::Active
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
    /// `sMarioDirection` ($A22B): `$01` right / `$FF` left. The shell
    /// placement tails (`$864A`/`$584A9`) test it, not the position.
    pub dir_right: bool,
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
    /// `sObjectDone` ($AF3E) per slot: the ROM keeps the byte in the scratch
    /// page ($AF00) while both the handler loop and the bank-3 draw (`$C000`)
    /// read it, and `$C014` returns before drawing when it is set. During a
    /// slot's frame it is the *dispatch's* own `done`, so a defeated object —
    /// `$85FC` sets it — is not drawn on the frame it dies. The port's sim only
    /// needs the per-slot record: [`Self::done`] still carries the byte.
    pub slot_done: [bool; SLOT_COUNT],
    /// `sActionLatch` ($AF3A): the `$8611` pickup's one-shot. Set when B-held
    /// contact turns a shell into the carried anim, and cleared by the
    /// carried/kicked-state exits; while it is set a second contact cannot
    /// re-trigger the pickup.
    pub action_latch: u8,
    /// `sObjectHitFlag` ($AF2E): `$5B697`'s "the carried shell hit another
    /// object this frame". Cleared per slot per frame like `$8000` does and
    /// read by the carried state's exit arm.
    pub object_hit_flag: u8,
    /// `$8611`'s B-not-held tail (`$8633` -> `$864A`): the anim bump happens in
    /// the flag dispatch, but the `±16` placement needs the level layout, so the
    /// dispatcher raises this and [`Self::engine`] runs the tail on the same
    /// slot before the handler.
    pub pending_kick: bool,
    /// $AFC1: `sBossHitsRemaining`
    pub boss_hits_remaining: u8,
    /// $AFC2: `sBossSprite`
    pub boss_sprite: u8,
    /// $A2A4: `sTimerReload`
    pub boss_timer_reload: u8,
    /// $A255: `sBossDefeated`
    pub boss_defeated: bool,
    /// Raw 512-byte slot staging buffer ($AD00-$AEFF) for boss driver exact staging
    pub boss_slots_raw: [u8; 512],
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
            slot_done: [false; SLOT_COUNT],
            action_latch: 0,
            object_hit_flag: 0,
            pending_kick: false,
            boss_hits_remaining: 0,
            boss_sprite: 0,
            boss_timer_reload: 0,
            boss_defeated: false,
            boss_slots_raw: [0; 512],
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

    /// Bank 2 `$8DFD`/`$8E6C`: stage boss slots and metadata from `lookup_boss_spawn_def`.
    ///
    /// Implements byte-for-byte Bank 2 staging:
    /// 1. Clears both slot pages ($AD00-$AEFF, 512 bytes) and resets slot array.
    /// 2. Sets `boss_hits_remaining = hits`, `boss_sprite = sprite`, `boss_timer_reload = 80`.
    /// 3. Copies 18-byte `obj0` to `$AD05` (offset 5 in slot 0).
    /// 4. If `count > 0`, loops over sub-objects starting destination at `$AD25` (offset 0x25)
    ///    with literal `+$10` stride per sub-object (causing overlap on count >= 2).
    /// 5. Reflects raw staged bytes into `slots[0..4]` for slot-level consumers.
    pub fn spawn_boss(&mut self, def_index: u8) {
        let (count, hits, sprite, obj0, sub_objs) = crate::scene::lookup_boss_spawn_def(def_index);
        self.boss_slots_raw = [0; 512];
        for slot in &mut self.slots {
            slot.b = [0; 32];
        }
        self.boss_hits_remaining = hits;
        self.boss_sprite = sprite;
        self.boss_timer_reload = 80;

        // obj0 at $AD05 (offset 5 in slot 0)
        self.boss_slots_raw[5..5 + 18].copy_from_slice(&obj0);

        // Sub-objects start at $AD25 (offset 0x25 = 37) with literal +$10 (16 bytes) stride
        if count > 0 {
            let mut offset = 0x25usize;
            for sub in sub_objs.iter().take(count as usize) {
                if offset + 18 <= self.boss_slots_raw.len() {
                    self.boss_slots_raw[offset..offset + 18].copy_from_slice(sub);
                }
                offset += 0x10;
            }
        }

        // Copy raw staged slot pages into slots 0..4 (32 bytes each)
        for i in 0..4 {
            self.slots[i].b.copy_from_slice(&self.boss_slots_raw[i * 32..(i + 1) * 32]);
        }
    }

    /// Helper for testing interaction dispatch directly.
    pub fn post_touch(&mut self, st: &mut Staging, _layout: &[u8; 48], tables: &ObjectTables, contact: &MarioContact) {
        self.mario = *contact;
        self.interaction_dispatch(st, tables);
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
                    self.slot_done[i] = true; // promoted this frame: not drawn yet
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
        // $8000 clears `sObjectHitFlag` per slot per frame, before the handler.
        self.object_hit_flag = 0;
        self.pending_kick = false;
        // $81A7 (slot → staging) + $3A50 (bank 16).
        self.bank16_update(i, &mut st, layout, tables);
        // `$C014`: the draw is skipped while this slot's `sObjectDone` is set.
        self.slot_done[i] = self.done != 0;
        // `$8611`'s B-not-held tail, which needs the layout.
        if self.pending_kick {
            self.pending_kick = false;
            self.kick_placement(&mut st, layout);
        }
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
            0x586EE => self.handler_marker66(st),
            0x586F5 => self.handler_pickup16(st, tables),
            0x58712 => self.handler_hopper(st, layout, tables),
            0x58755 => self.handler_itempose20(st),
            0x58758 => self.itempose_body(st),
            0x587BD => self.handler_pose86(st),
            0x587D7 => self.handler_pose87(st, tables),
            0x5839B => self.handler_kicked(st, layout),
            0x58444 => self.handler_carried(st, layout, tables),
            0x58DD1 => self.handler_carrier46(st, layout, tables),
            0x594EB => self.handler_fallwalker52(st, layout),
            0x59BC5 => self.handler_groundwalker88(st, layout),
            0x5B1C8 => self.handler_grower92(st),
            0x5B241 => self.handler_floater94(st),
            0x5B28C => self.handler_floater95(st),
            // $58100 — a bare `ret` (ROM bytes: the single byte `$C9` right after
            // the 192-byte mode-A table at $58040). Nineteen mode-A indices and
            // eleven mode-B ones dispatch here, and it is a *real* target, not a
            // missing one: the $58000 prelude still runs (interaction, bounce,
            // star), only the per-anim body does nothing. Dispatching it
            // explicitly keeps it out of `note_unported`, where it would look
            // like a gap in the port rather than a fixture of the ROM.
            0x58100 => {}
            0x581B1 => self.handler_death(st, layout, tables),
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
        self.fallwalker_body(st, layout, 0x56);
    }

    /// `$594EB` (anim 68): the same handler with `$54F4`'s `$52` in place of
    /// `$582D9`'s `$56` — the identical `$7585` bob head, the identical jump
    /// into the shared `$42F5` state dispatch and the identical `$42DD`
    /// ground arm (the ROM `jp`s where the original `jr`s; the code is the
    /// same). Only the type byte differs.
    fn handler_fallwalker52(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.fallwalker_body(st, layout, 0x52);
    }

    /// The `$42DD`/`$42F5`/`$42FB`/`$431A`/`$4339` bodies both handlers run;
    /// `ty` is the one immediate that differs between them.
    fn fallwalker_body(&mut self, st: &mut Staging, layout: &LevelLayout, ty: u8) {
        self.sprite_bob(st); // $5B585
        match st.b[B_FSTATE] {
            0 => {
                st.b[B_TYPE] = ty;
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

    // ---------------------------------------------------------------------
    // Anim 88 — the ground walker ($59BC5)
    // ---------------------------------------------------------------------

    /// `$59BC5` (anim 88): the `$7585` sprite-bob head, then a 3-state
    /// walker. State 0 (`$5BD1`) is the one-shot init: flash `$10`, type
    /// `$4E`, `face_mario` — `$73C6` picks state 1/2 and the flip byte — and
    /// `+28` = 1 (so the first walking call arms the step budget). States 1/2
    /// (`$5D05`/`$5D41`) probe the wall on their moving side on *every* call
    /// (`$3C1C` left / `$3BB8` right, X∓7 at Y−11/Y−5); a hit flips through
    /// `$5D2D` (`+13 ^= $20`, `+15 ^= 3` — 1 <-> 2) and leaves both counters
    /// alone. Otherwise they run the shared `$5D0F`/`$5D4B` pause/step
    /// counter (`$5D22`): 16 calls of `+28` counting down, then 48 one-pixel
    /// steps on `+25`, then `+28` = `$10` again.
    fn handler_groundwalker88(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.sprite_bob(st); // $5B585
        match st.b[B_FSTATE] {
            // $5BD1: the one-shot init. (The state byte itself is written by
            // `$73C6` inside `face_mario`.)
            0 => {
                st.b[B_FLASH] = 0x10;
                st.b[B_TYPE] = 0x4E;
                self.face_mario(st); // $73C6
                st.b[B_T28] = 1;
            }
            // $5D05: walk left.
            1 => {
                self.m_left_wall(st, layout); // $3C1C
                if self.move_flags & 0x02 != 0 {
                    self.walker88_flip(st); // $5D2D
                    return;
                }
                self.walker88_pause_step(st, true); // $5D1D -> $5D22
            }
            // $5D41: walk right.
            2 => {
                self.m_right_wall(st, layout); // $3BB8
                if self.move_flags & 0x01 != 0 {
                    self.walker88_flip(st); // $5D2D
                    return;
                }
                self.walker88_pause_step(st, false); // $5D3A -> $5D22
            }
            _ => {}
        }
    }

    /// `$5D0F`/`$5D4B` (`$5D22` tail): the walker's two-phase pause/step
    /// counter, shared by the two walk states. `+28` counts down first —
    /// none of those 16 calls move; the call that takes it to 0 arms the
    /// `+25` = `$30` step budget and *still* does not move. Then each call
    /// takes one 1-px step (`$3D22` X−=1 left, `$3D32` X+=1 right) and
    /// decrements `+25`; its exhaustion re-arms `+28` = `$10`.
    fn walker88_pause_step(&mut self, st: &mut Staging, left: bool) {
        if st.b[B_T28] != 0 {
            st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
            if st.b[B_T28] != 0 {
                return;
            }
            st.b[B_T25] = 0x30;
            return;
        }
        if left {
            self.mv_x_sub(st, 1); // $5D1D: `ld b,1 / call $3D22`
        } else {
            self.mv_x_add(st, 1); // $5D3A: `ld b,1 / call $3D32 / jr $5D22`
        }
        st.b[B_T25] = st.b[B_T25].wrapping_sub(1);
        if st.b[B_T25] != 0 {
            return;
        }
        st.b[B_T28] = 0x10;
    }

    /// `$5D2D`: the shared wall flip — `+13 ^= $20`, then the `$5D33` tail
    /// `+15 ^= 3` (walk-left 1 <-> walk-right 2).
    fn walker88_flip(&mut self, st: &mut Staging) {
        st.b[B_FLIP] ^= 0x20;
        st.b[B_FSTATE] ^= 3;
    }

    // ---------------------------------------------------------------------
    // Anim 46 — the carrying platform ($58DD1)
    // ---------------------------------------------------------------------

    /// `$58DD1` (anim 46): a carrying platform. State 0 (`$4DD1`, one shot):
    /// type `$20` — the on-object arm, Mario stands on it — and state 2.
    /// State 2 (`$4E04`) waits for `+23` bit 3 (`$FFE7` bit 3, raised by the
    /// `$2DF7` on-object arm the port calls `respond`, cleared by `$818D`)
    /// and then arms state 1 with `+28` = 28, without moving that call.
    /// State 1 (`$4DE5`) counts `+28` down — the call that takes it to 0
    /// falls *through* to the move, as does an already-0 counter — and then,
    /// every call: step the `$7488` curve (`$7451`), move down (`$3D3D`),
    /// carry Mario (`$3CDF`), ground-probe (`$3B40`) and, on ground, remove
    /// itself through `$72D7`.
    fn handler_carrier46(&mut self, st: &mut Staging, layout: &LevelLayout, tables: &ObjectTables) {
        match st.b[B_FSTATE] {
            // $4DE5: the countdown, then the shared move chain at $4DEE.
            1 => {
                if st.b[B_T28] != 0 {
                    st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                    if st.b[B_T28] != 0 {
                        return;
                    }
                }
                let b = self.fall_step(st); // $7451: c = $15 / $7488
                self.mv_y_add(st, b); // $3D3D
                self.carry_mario(st, b); // $3CDF
                self.m_ground_own(st, layout); // $3B40
                if self.move_flags & 0x08 == 0 {
                    return;
                }
                self.remove_object(st, tables); // $72D7
            }
            // $4E04: wait until Mario stands on the platform.
            2 => {
                if st.b[B_FLAGS] & 0x08 == 0 {
                    return;
                }
                st.b[B_FSTATE] = 1;
                st.b[B_T28] = 0x1C;
            }
            // $4DD1: the one-shot init.
            _ => {
                st.b[B_TYPE] = 0x20;
                st.b[B_FSTATE] = 2;
            }
        }
    }

    /// `$3CDF` (bank 0, reached by the anim-46 move chain): the carry. While
    /// `$FFE7` bit 3 is set (the staging copy of `+23` — the port's
    /// `B_FLAGS`) it adds `b` to Mario's 16-bit Y at `$3CD7` (`[hl] += b`,
    /// `adc $00` into the high byte — `wrapping_add` here). The
    /// `sScrollDeltaDown` write before the jump is dead: the camera writes
    /// `$A238` before reading it and no other path reads it.
    ///
    /// The ROM adds onto the *live* `$FFC0`, so a Y already staged this frame
    /// (`$2DC6`'s bounce / stand-nudge) forms the base — which also keeps two
    /// carriers in one frame from dropping an add.
    fn carry_mario(&mut self, st: &mut Staging, b: u8) {
        if st.b[B_FLAGS] & 0x08 == 0 {
            return;
        }
        let y = self.mario_y_adjust.unwrap_or(self.mario.y);
        self.mario_y_adjust = Some(y.wrapping_add(b as u16));
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

    /// `$586EE` (mode-B anims 27/28/29; the mode-A table shares the address):
    /// type = anim + `$66` and return — the ROM stream's anim bytes are
    /// `$1B`/`$1C`/`$1D`, so the types are `$81`/`$82`/`$83`. Being bit-7
    /// types with a nonzero low 7 bits they pass the `$2EDD` gate and the
    /// `$2EF5` box test, and `$2DC6`'s tail stages item ids 1/2/3 — the
    /// object is a collectible-shaped item that the `$3451` powerup arms
    /// consume, not a dead id. (The anim-30 entry just before it, `$586EB`'s
    /// `call $7684` — the 4-frame `$FFDE` toggle — is not reached by level 20
    /// and stays unported.)
    fn handler_marker66(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = st.b[B_DEF].wrapping_add(0x66);
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
    // Anims 92/94/95 — the grower ($5B1C8) and the floaters ($5B241/$5B28C)
    // ---------------------------------------------------------------------

    /// `$5B1C8` (mode-B anim 92): a 4-state grower/bobber. State 0 (`$71D3`,
    /// also the `jr` target of state 3) arms type `$4C`, state 1 and the
    /// counters `+24`/`+25`/`+26` = 3 — with no step on its own frame. State 1
    /// (`$71E4`) takes one 1-px step per call — up while `+16 == 0`, down
    /// otherwise — and at every exhausted `+24` reloads it from `+25` and
    /// flips `+16`; on the third segment (`+26` reaching 0) it instead drops
    /// the type to 0 and enters state 2. State 2 (`$7213`) steps `+11` up to
    /// `$7C` on every 4th `$FFF0` frame and arms `+28` = 32 into state 3;
    /// state 3 (`$7229`) counts `+28` down, then walks `+11` back down to
    /// `$7A` and jumps to the state-0 body.
    fn handler_grower92(&mut self, st: &mut Staging) {
        match st.b[B_FSTATE] {
            1 => {
                // $71E4: b = 1; `+16` picks the direction.
                let b = 1;
                if st.b[B_PHASE] == 0 {
                    self.mv_y_sub(st, b);
                } else {
                    self.mv_y_add(st, b);
                }
                st.b[B_T24] = st.b[B_T24].wrapping_sub(b);
                if st.b[B_T24] != 0 {
                    return;
                }
                // $71F3: a 3-px segment ended.
                st.b[B_ANIMFRAME] = st.b[B_ANIMFRAME].wrapping_sub(1);
                if st.b[B_ANIMFRAME] == 0 {
                    // $720B: the last segment — harmless from here on.
                    st.b[B_TYPE] = 0;
                    st.b[B_FSTATE] = 2;
                    return;
                }
                st.b[B_T24] = st.b[B_T25];
                st.b[B_PHASE] ^= 1;
            }
            2 => {
                // $7213: sprite id 122 -> 124, one step every 4th frame.
                if self.fff0 & 3 != 0 {
                    return;
                }
                st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_add(1);
                if st.b[B_SPRITE] != 0x7C {
                    return;
                }
                st.b[B_T28] = 0x20;
                st.b[B_FSTATE] = 3;
            }
            3 => {
                // $7229: a 32-call hold, then the id walks back to 122.
                if st.b[B_T28] != 0 {
                    st.b[B_T28] = st.b[B_T28].wrapping_sub(1);
                    return;
                }
                if self.fff0 & 3 != 0 {
                    return;
                }
                st.b[B_SPRITE] = st.b[B_SPRITE].wrapping_sub(1);
                if st.b[B_SPRITE] != 0x7A {
                    return;
                }
                self.grower92_init(st); // $18 $92: jr $71D3
            }
            _ => self.grower92_init(st),
        }
    }

    /// `$71D3`: the grower's state 0 — type `$4C` (harmful: `$4C & $63` =
    /// `$40`), state 1, and the segment counter `+24` with its reload `+25`
    /// and count `+26`.
    fn grower92_init(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = 0x4C;
        st.b[B_FSTATE] = 1;
        st.b[B_T24] = 3;
        st.b[B_T25] = 3;
        st.b[B_ANIMFRAME] = 3;
    }

    /// `$744D` (c = `$12`, profile `$7488`) / `$7473` (c = `$12`, profile
    /// `$74A1`): the shared `$74xx` step picker. `b` is the profile byte at
    /// `+16`; `+16` advances unless it already reads the entry's sentinel.
    /// `c` lives in a register that crosses into the caller's shared tail
    /// (`cp c`), so the sentinel is returned with the step instead of being
    /// folded into a caller. The other entries (`$7441`/`$7445`/`$7449`/
    /// `$7451`/`$7465`/`$746C`/`$747A`/`$7481`, i.e. the fall/hover/bounce
    /// curves) are reached by handlers outside level 20 and are not modelled
    /// here.
    fn step_picker(&self, st: &mut Staging, table: &[u8], c: u8) -> (u8, u8) {
        // Bound-safe: this consumer's `+16` never exceeds the sentinel, and a
        // past-the-end read cannot answer from another table's bytes.
        let b = table.get(st.b[B_PHASE] as usize).copied().unwrap_or(0);
        if st.b[B_PHASE] != c {
            st.b[B_PHASE] = st.b[B_PHASE].wrapping_add(1);
        }
        (b, c)
    }

    /// `$7253`/`$7277` (and the anim-95 twins) head: `+24` selects the profile
    /// — 0 -> `$7473`'s `$74A1`, else `$744D`'s `$7488`; both sentinel `$12`.
    fn floater_pick(&self, st: &mut Staging) -> (u8, u8) {
        if st.b[B_T24] == 0 {
            self.step_picker(st, &STEP_74A1, 0x12)
        } else {
            self.step_picker(st, &FALL_SPEED, 0x12)
        }
    }

    /// `$7266`: the anim-94/95 shared tail — when `+16` reads the picker's
    /// sentinel (the picker increments on a leg's last call, so the compare
    /// sees `c` exactly then), restart the profile and toggle `+24`; the
    /// 0 -> 1 edge flips the movement direction through `$5D33` (`$FFDF ^=
    /// $03` maps state 1 <-> 2), the 1 -> 0 edge just returns.
    fn floater_tail(&mut self, st: &mut Staging, c: u8) {
        if st.b[B_PHASE] != c {
            return;
        }
        st.b[B_PHASE] = 0;
        st.b[B_T24] ^= 1;
        if st.b[B_T24] == 0 {
            return;
        }
        st.b[B_FSTATE] ^= 3; // $5D33
    }

    /// `$724A`: the anim-94/95 shared state 0 — type `$48` (harmful), state 1;
    /// no movement on its own frame.
    fn floater_init(&mut self, st: &mut Staging) {
        st.b[B_TYPE] = 0x48;
        st.b[B_FSTATE] = 1;
    }

    /// `$5B241` (mode-B anim 94): the vertical floater — states 1/2 walk the
    /// `+24`-selected profile (18 calls per leg, the steps summing to 32 px)
    /// with Y -= b (`$3D2D`) up / Y += b (`$3D3D`) down, sharing state 0 and
    /// the `$7266` tail, which flips the direction every other leg.
    fn handler_floater94(&mut self, st: &mut Staging) {
        match st.b[B_FSTATE] {
            1 => {
                let (b, c) = self.floater_pick(st);
                self.mv_y_sub(st, b);
                self.floater_tail(st, c);
            }
            2 => {
                let (b, c) = self.floater_pick(st);
                self.mv_y_add(st, b);
                self.floater_tail(st, c);
            }
            0 => self.floater_init(st),
            _ => {}
        }
    }

    /// `$5B28C` (mode-B anim 95): the same machine on X (`$3D22` X -= b left /
    /// `$3D32` X += b right), sharing anim 94's state-0 body and `$7266` tail.
    fn handler_floater95(&mut self, st: &mut Staging) {
        match st.b[B_FSTATE] {
            1 => {
                let (b, c) = self.floater_pick(st);
                self.mv_x_sub(st, b);
                self.floater_tail(st, c);
            }
            2 => {
                let (b, c) = self.floater_pick(st);
                self.mv_x_add(st, b);
                self.floater_tail(st, c);
            }
            0 => self.floater_init(st),
            _ => {}
        }
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
    // Anim 0 — the defeated enemy's death animation ($581B1)
    // ---------------------------------------------------------------------

    /// `$581B1` (mode-A anim 0): the death animation every defeat runs into.
    /// `$85FC` leaves the slot as (anim 0, type 0) with `sObjectDone` set, and
    /// from the next frame this handler plays: state 0 hops it 20 px **up** and
    /// flips it upside down (`+13` bit 6), state 1 rises on the `$74AB` 8-entry
    /// curve at 2 b a frame until the curve's sentinel, and state 2 falls on the
    /// `$5B451` curve (also 2 b) and removes the slot once its staged screen Y
    /// passes `$A0` — the flip-and-fall-off-the-screen death. This is what a
    /// fireball or a stomp defeat *looks like*, and without it a defeated enemy
    /// simply froze on screen.
    fn handler_death(&mut self, st: &mut Staging, layout: &LevelLayout, tables: &ObjectTables) {
        let _ = layout;
        match st.b[B_FSTATE] {
            // $581CE: rise on 2x the $74AB curve.
            1 => {
                let (b, c) = self.step_picker(st, &DEATH_RISE_74AB, 8);
                self.mv_y_sub(st, b.wrapping_mul(2));
                if st.b[B_PHASE] != c {
                    return;
                }
                st.b[B_FSTATE] = 2;
                st.b[B_PHASE] = 0;
            }
            // $581E2: fall on 2x the $5B451 curve; off the bottom of the
            // screen -> `$5B2D7` with remove-mode 2.
            2 => {
                let b = self.fall_step(st);
                self.mv_y_add(st, b.wrapping_mul(2));
                if st.b[B_SCRY] < 0xA0 {
                    return;
                }
                self.remove_mode = 2;
                self.remove_object(st, tables);
                // $581E2's tail also counts `sBossDefeated` down; the port
                // models that byte as a bool (the boss item), so there is
                // nothing to decrement here yet.
            }
            // $581B9: the hop + the flip.
            _ => {
                self.mv_y_sub(st, 20);
                st.b[B_FLIP] ^= 0x40;
                st.b[B_PHASE] = 0;
                st.b[B_TYPE] = 0;
                st.b[B_FSTATE] = 1;
            }
        }
    }

    // ---------------------------------------------------------------------
    // Anims 3/4 — the kicked shell and the carried shell ($5839B / $58444)
    // ---------------------------------------------------------------------

    /// `$5839B` (anim 3): the kicked shell / walker. Type `$5F` on its first
    /// call, then a four-state walk shared with anim 9's shape: 1/2 walk (3 px
    /// steps, wall -> turn) and 3/4 are the ledge falls, the `$5B451` curve
    /// with a landing that subtracts 2 from the state and clears the fall
    /// index. Every call also runs `$5B607` (the sprite wobble, the same
    /// routine anim 8's spin uses) and, while the type has bit 6, `$5B697` —
    /// the KO overlap that defeats other enemies this shell runs into.
    fn handler_kicked(&mut self, st: &mut Staging, layout: &LevelLayout) {
        if st.b[B_TYPE] & 0x40 != 0 {
            self.ko_overlap(st);
        }
        self.spin_step(st); // $5B607 - byte-identical to $5B545's wobble
        match st.b[B_FSTATE] {
            0 => {
                // $5B3C6 + type $5F.
                self.face_mario(st);
                st.b[B_TYPE] = 0x5F;
            }
            // $83BC / $83E6: walk. The probes are the *low* pair ($3BA4/$3C08),
            // not the anim-9 walker's Y-11 ones.
            1 => {
                self.m_right_wall_low(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1); // $583DB
                    return;
                }
                self.mv_x_add(st, 3);
                self.walk_ground_check(st, layout); // $583CB: no ground -> state += 2
            }
            2 => {
                self.m_left_wall_low(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1); // $583F7
                    return;
                }
                self.mv_x_sub(st, 3);
                self.walk_ground_check(st, layout);
            }
            // $8402 / $8433: the falls.
            3 => {
                self.m_right_wall_low(st, layout);
                if self.move_flags & 0x01 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_add(1);
                    return;
                }
                self.mv_x_add(st, 3);
                self.kick_fall(st, layout);
            }
            4 => {
                self.m_left_wall_low(st, layout);
                if self.move_flags & 0x02 != 0 {
                    st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(1);
                    return;
                }
                self.mv_x_sub(st, 3);
                self.kick_fall(st, layout);
            }
            _ => {}
        }
    }

    /// `$58411`: one fall step — no ground under the shell → `$5B451` + 1 px,
    /// ground → `$58423` (land: clear the fall index, state −= 2, snap Y's low
    /// nibble).
    fn kick_fall(&mut self, st: &mut Staging, layout: &LevelLayout) {
        self.m_ground_own(st, layout);
        if self.move_flags & 0x08 != 0 {
            st.b[B_PHASE] = 0;
            st.b[B_FSTATE] = st.b[B_FSTATE].wrapping_sub(2);
            st.b[B_YLO] &= 0xF0;
            return;
        }
        let b = self.fall_step(st);
        self.mv_y_add(st, b.wrapping_add(1));
    }

    /// `$58444` (anim 4): the carried shell. Pipe travel removes it; state 0
    /// sets the type to 0 and enters state 1 (`$58452`), which every frame
    /// follows Mario (`$5B4C1`), runs the KO overlap (`$5B697`), and exits
    /// through `$584C3` (defeat itself via `$3ADC`) when it hit something or
    /// found a ceiling 8 px above in the `$3B68` probe. While B is held it
    /// stays carried; on release it drops one anim (4 → 3) and runs the
    /// placement tail.
    fn handler_carried(&mut self, st: &mut Staging, layout: &LevelLayout, tables: &ObjectTables) {
        if self.mario.travelling {
            // $58444: pipe travel → sActionLatch = 0, `$5B2D7` removal.
            self.action_latch = 0;
            self.remove_object(st, tables);
            return;
        }
        if st.b[B_FSTATE] == 0 {
            // $58452: type 0, state 1.
            st.b[B_TYPE] = 0;
            st.b[B_FSTATE] = 1;
            return;
        }
        self.follow_mario(st);
        self.ko_overlap(st);
        if self.object_hit_flag != 0 {
            self.release_and_defeat(st);
            return;
        }
        self.m_ceil_own(st, layout, 8); // $3B68
        if self.move_flags & 0x04 != 0 {
            self.release_and_defeat(st);
            return;
        }
        if self.mario.b_held {
            return;
        }
        // B released: `$3AB4` (`$85DC`) drops the anim, then the placement.
        self.action_latch = 0;
        st.b[B_DEF] = st.b[B_DEF].wrapping_sub(1);
        self.kick_placement(st, layout);
    }

    /// `$584C3`: the carried shell ran into something — clear the latch and
    /// defeat *itself* (`$3ADC` is `$85FC`).
    fn release_and_defeat(&mut self, st: &mut Staging) {
        self.action_latch = 0;
        self.defeat(st);
    }

    /// `$5B4C1`: follow Mario — his 16-bit X/Y, `+8` on X, then the Y offset by
    /// his size and ducking (5 / 14 / 20).
    fn follow_mario(&mut self, st: &mut Staging) {
        set_x(&mut st.b, self.mario.x);
        set_y(&mut st.b, self.mario.y);
        self.mv_x_add(st, 8);
        let b = match (self.mario.powerup != 0, self.mario.ducking) {
            (true, false) => 5,
            (false, true) => 20,
            _ => 14,
        };
        self.mv_y_add(st, b);
    }

    /// `$5B697` → `$5B6A9`: the carried/kicked shell's overlap test against
    /// every slot's own hitbox (`+3`/`+10` on Y, `±5` on X), against the
    /// shell's *staged* screen pair. Type 0 slots are skipped by the caller's
    /// scan, and `$5F` (itself, another shell) and anything with bit 7 (item
    /// poses) or bit 5 (immune) are skipped here. A hit sets the target's
    /// `+17` bit 1 — the defeat flag the interaction dispatch consumes — and
    /// raises `sObjectHitFlag`.
    fn ko_overlap(&mut self, st: &Staging) {
        let sy = st.b[B_SCRY];
        let sx = st.b[B_SCRX];
        for j in 0..SLOT_COUNT {
            let ty = self.slots[j].b[B_TYPE];
            if ty == 0 || ty == 0x5F || ty & 0xA0 != 0 {
                continue;
            }
            let b = &self.slots[j].b;
            let oy = b[B_SCRY];
            let ox = b[B_SCRX];
            let d = b[B_HB_TOP].wrapping_add(3).wrapping_add(oy);
            let e = b[B_HB_BOTTOM]
                .wrapping_add(10)
                .wrapping_add(oy)
                .wrapping_sub(d);
            if sy.wrapping_sub(d) >= e {
                continue;
            }
            let d = b[B_HB_LEFT].wrapping_sub(5).wrapping_add(ox);
            let e = b[B_HB_RIGHT]
                .wrapping_add(5)
                .wrapping_add(ox)
                .wrapping_sub(d);
            if sx.wrapping_sub(d) >= e {
                continue;
            }
            self.slots[j].b[B_FLAGS] |= 0x02;
            self.object_hit_flag = 1;
        }
    }

    /// `$864A` / `$584A9`: the shared placement tail once the anim is 3. The
    /// shell is nudged 16 px to Mario's facing side and probed there with the
    /// *low* wall pair: a wall turns it into a live kicked shell (type `$5F`,
    /// state walking away from the wall); open air leaves it anim 3 with the
    /// state it already had, i.e. walking away from Mario.
    fn kick_placement(&mut self, st: &mut Staging, layout: &LevelLayout) {
        if self.mario.dir_right {
            self.mv_x_add(st, 16);
            self.m_right_wall_low(st, layout);
            if self.move_flags & 0x01 == 0 {
                return;
            }
            self.mv_x_sub(st, 16);
            st.b[B_TYPE] = 0x5F;
            st.b[B_FSTATE] = 1;
        } else {
            self.mv_x_sub(st, 16);
            self.m_left_wall_low(st, layout);
            if self.move_flags & 0x02 == 0 {
                return;
            }
            self.mv_x_add(st, 16);
            st.b[B_TYPE] = 0x5F;
            st.b[B_FSTATE] = 2;
        }
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
            // `$8611`'s type-`$06` arm — the carry/kick entry.
            if self.mario.b_held {
                // B held: the one-shot latch, then anim += 2 (2 -> 4 = carried)
                // and `$863C`. `$8240` inside it clears `+14..+30`, the type byte
                // included, so the carried object leaves here as (anim 4, type 0,
                // state 0) and the anim-4 handler takes it from the next frame.
                if self.action_latch != 0 {
                    return;
                }
                self.action_latch = 1;
                st.b[B_DEF] = st.b[B_DEF].wrapping_add(2);
                self.finish_sprite_swap(st, tables);
                return;
            }
            // B not held: `$8633` (anim += 1 -> the kicked walker) then the
            // placement tail, which needs the layout the dispatcher does not
            // carry; the caller runs it (see `pending_kick`).
            st.b[B_DEF] = st.b[B_DEF].wrapping_add(1);
            self.pending_kick = true;
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

    /// `$2FED` → `$3007`: the fireball-vs-object pass, run once per live ball
    /// before the object engine (the call sits inside `$3261`, i.e. inside
    /// `$0F71`). `sy`/`sx` are the ball's staged screen pair (`$FFC4`/
    /// `$FFC5`), the same pair the player's contact math uses.
    ///
    /// The scan gate is `B_FLAGS == 0` — bit 3 included, so unlike `$2D7D`
    /// (`and $F7`) a slot Mario is standing on is not hittable by a ball. Then
    /// type 0 is an empty slot and a bit-7 type is an item pose: both
    /// transparent. The box is `B_HB_*` expanded by 6 px against the slot's
    /// stored screen pair, tested byte-wise as `(pair - origin) < extent`.
    ///
    /// A bit-2 type is *defeated* here — `B_FLAGS` bit 1, which `$85AA`'s
    /// dispatch (`interaction_dispatch`) turns into [`Self::defeat`] next
    /// frame; everything else takes the sfx arm, which the caller applies
    /// (`sSfxRequest0 = 2` is a player-sim byte). Both leave the ROM's
    /// `sObjectHitResult` non-zero, which is what kills the ball. The scan
    /// keeps walking after a hit, so the last hit's verdict is the one that
    /// survives, exactly like the shared `sObjectHitResult`.
    pub fn fire_hit(&mut self, sy: u8, sx: u8) -> FireHit {
        let mut result = FireHit::None;
        for s in self.slots.iter_mut() {
            if s.b[B_FLAGS] != 0 {
                continue;
            }
            let ty = s.b[B_TYPE];
            if ty == 0 || ty & 0x80 != 0 {
                continue;
            }
            let oy = s.b[B_SCRY];
            let y0 = s.b[B_HB_TOP].wrapping_sub(6).wrapping_add(oy);
            let h = s.b[B_HB_BOTTOM]
                .wrapping_add(6)
                .wrapping_add(oy)
                .wrapping_sub(y0);
            if sy.wrapping_sub(y0) >= h {
                continue;
            }
            let ox = s.b[B_SCRX];
            let x0 = s.b[B_HB_LEFT].wrapping_sub(6).wrapping_add(ox);
            let w = s.b[B_HB_RIGHT]
                .wrapping_add(6)
                .wrapping_add(ox)
                .wrapping_sub(x0);
            if sx.wrapping_sub(x0) >= w {
                continue;
            }
            if ty & 0x04 != 0 {
                s.b[B_FLAGS] |= 0x02;
                result = FireHit::Defeat;
            } else {
                result = FireHit::Die;
            }
        }
        result
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
        // $2E7C: moon item shield (bit 2 in sMoonPhysics): consume and grant invincibility
        if self.mario.moon_physics & 0x04 != 0 {
            self.mario.moon_physics &= !0x04;
            self.mario.invincibility = 0x80;
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
        assert!(obj.unported.is_empty(), "type 06 is ported at $8611");
        assert_eq!(st.b[B_DEF], 2, "kicked without B held: anim += 1");
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

    /// Anim 22 — the goal bell's own script (`$587BD`) and its ring tick
    /// (`$5B545`). Un-touched it re-stages type `$86` every frame (item id 6, the
    /// strike); one touch — `+$17` bit 4, which `$2DC6` sets — flips it to frame
    /// state 1 with the type cleared, and from then on the tick walks the
    /// 4-phase ping-pong with the X mirror on phase 3, one phase per `$FFF0 & 3`.
    ///
    /// The same tick routine serves anim 23 and *is* capture-verified through it
    /// (traj13's touch shows the 8-frame steps and the mirror at phase 3), but no
    /// capture ever touches anim 22: level 1's bell sits at anchor y 360, the
    /// measured jump apex on that floor is y 399, and the touch window is
    /// `marioY - objectY` in `[-43, -13)` — 44 px short. This test is that gap's
    /// only coverage, and the cadence it pins is the console-measured one.
    #[test]
    fn the_goal_bells_own_script_arms_then_ping_pongs() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json");
        let Ok(txt) = std::fs::read_to_string(path) else {
            eprintln!("skip: {path} not built");
            return;
        };
        let t: ObjectTables = serde_json::from_str(&txt).expect("objects.json parses");
        assert_eq!(
            t.behavior_for(0, 22),
            Some(0x587BD),
            "anim 22 is the bell's script"
        );

        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        let mut st = Staging::default();
        st.b[B_DEF] = 22;
        st.b[B_SPRITE] = 0x1F; // anim_to_sprite_a[22], what $8240 stages

        // Un-rung: the type is re-staged every frame, which is what lets the
        // strike happen at all.
        obj.fff0 = 0;
        obj.run_handler(&mut st, &blank_layout(), &t);
        assert!(obj.unported.is_empty(), "the bell's script is ported");
        assert_eq!(st.b[B_TYPE], 0x86, "un-rung type = item id 6");
        assert_eq!(st.b[B_FSTATE], 0, "frame state 0");
        assert_eq!(st.b[B_SPRITE], 0x1F, "and the sprite is untouched");

        // One touch.
        st.b[B_FLAGS] |= 0x10; // +$17 bit 4, set by $2DC6
        obj.run_handler(&mut st, &blank_layout(), &t);
        assert_eq!(st.b[B_FSTATE], 1, "ringing");
        assert_eq!(st.b[B_TYPE], 0, "and no longer collectible");
        assert_eq!(st.b[B_SPRITE], 0x1F, "the touch frame does not tick");

        // The tick: `$FFF0 & 3` gates it, so one phase per 4 engine frames = 8
        // game frames, which is the cadence the console showed.
        st.b[B_FLAGS] &= !0x10;
        for (i, (sprite, mirror)) in [(0x20u8, 0u8), (0x1F, 0), (0x20, 0x20), (0x1F, 0)]
            .into_iter()
            .enumerate()
        {
            obj.fff0 = 4; // == 0 mod 4: the gate opens
            obj.run_handler(&mut st, &blank_layout(), &t);
            let n = i + 1;
            assert_eq!(st.b[B_ANIMFRAME], (n % 4) as u8, "counter after tick {n}");
            assert_eq!(st.b[B_SPRITE], sprite, "sprite after tick {n}");
            assert_eq!(st.b[B_FLIP] & 0x20, mirror, "mirror after tick {n}");
        }

        // A frame whose gate is closed changes nothing at all.
        obj.fff0 = 5;
        let before = (st.b[B_SPRITE], st.b[B_ANIMFRAME], st.b[B_FLIP]);
        obj.run_handler(&mut st, &blank_layout(), &t);
        assert_eq!(
            (st.b[B_SPRITE], st.b[B_ANIMFRAME], st.b[B_FLIP]),
            before,
            "$5B545 returns without ticking when $FFF0 & 3 != 0"
        );
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
    // $586EE / $5B1C8 / $5B241 / $5B28C — level 19's four new classes
    // -----------------------------------------------------------------

    /// `$586EE` (mode-B anims 27/28/29): type = anim + `$66` and nothing else.
    /// The ROM's stream bytes for these records are `$1B`/`$1C`/`$1D` (the
    /// extractor's `r5` IS the staged anim byte), so the types come out
    /// `$81`/`$82`/`$83` — the spec's `$8D`-`$8F` reading added `$66` to
    /// `$27`-style values the stream never contains.
    #[test]
    fn marker66_sets_the_type_from_the_anim() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        for (anim, ty) in [(27u8, 0x81u8), (28, 0x82), (29, 0x83)] {
            let mut st = Staging::default();
            st.b[B_DEF] = anim;
            st.b[B_T24] = 0x0A;
            obj.handler_marker66(&mut st);
            assert_eq!(st.b[B_TYPE], ty, "anim {anim} -> type ${ty:02X}");
            assert_eq!(st.b[B_T24], 0x0A, "the marker touches nothing else");
            assert_eq!(obj.done, 0);
        }
        assert!(obj.unported.is_empty());
    }

    /// The four handlers the level-19 stream's cells reach are the class
    /// claims: `$586EE`'s anims are the literal record bytes `$1B`-`$1D`, which
    /// is why its type arithmetic lands on `$81`-`$83`.
    #[test]
    fn level19_record_anims_are_the_staged_bytes() {
        // From the level-19 packed stream (`baserom.gb` file offset $E7C8, 91
        // bytes): pack `64 61 06` -> anim 27, `75 DB 0E` -> 27, and the anim-92
        // packs `77 9B 98`/`4D F3 14`... decode to 92.
        let decode = |c: u8, b2: u8, _b3: u8| ((c >> 5) & 7) * 8 + if c & 0x10 != 0 { 0x40 } else { 0 } + ((b2 >> 5) & 7);
        assert_eq!(decode(0x64, 0x61, 0x06), 27, "the anim-27 record");
        assert_eq!(decode(0x77, 0x9B, 0x98), 92, "an anim-92 record");
        assert_eq!(decode(0x72, 0xC5, 0x96), 94, "the first anim-94 record");
        assert_eq!(decode(0x72, 0xFB, 0x9E), 95, "the anim-95 record");
        // anim byte 27 + $66 = $81: the `$3451` powerup arm's id 1.
        assert_eq!(27u8 + 0x66, 0x81);
    }

    /// `$5B1C8` states 0/1: the arm frame takes no step; the nine state-1
    /// calls are three segments of three 1-px steps — up while `+16 == 0`,
    /// down while `+16 == 1` — with `+24` reloading from `+25` and `+16`
    /// flipping after the first two segments; the third exhausts `+26` into
    /// type 0 and state 2.
    #[test]
    fn grower92_walks_three_segments_then_goes_harmless() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut st = Staging::default();
        st.b[B_SPRITE] = 122;
        set_y(&mut st.b, 100);
        obj.handler_grower92(&mut st);
        assert_eq!(
            (
                st.b[B_TYPE],
                st.b[B_FSTATE],
                st.b[B_T24],
                st.b[B_T25],
                st.b[B_ANIMFRAME]
            ),
            (0x4C, 1, 3, 3, 3)
        );
        assert_eq!(y_of(&st.b), 100, "state 0 takes no step");
        for expect in [99u16, 98, 97] {
            obj.handler_grower92(&mut st);
            assert_eq!(y_of(&st.b), expect, "segment 1 rises");
        }
        assert_eq!(
            (st.b[B_PHASE], st.b[B_T24], st.b[B_ANIMFRAME]),
            (1, 3, 2),
            "reload + flip after the first segment"
        );
        for expect in [98u16, 99, 100] {
            obj.handler_grower92(&mut st);
            assert_eq!(y_of(&st.b), expect, "segment 2 sinks");
        }
        assert_eq!((st.b[B_PHASE], st.b[B_ANIMFRAME]), (0, 1));
        for expect in [99u16, 98, 97] {
            obj.handler_grower92(&mut st);
            assert_eq!(y_of(&st.b), expect, "segment 3 rises");
        }
        assert_eq!((st.b[B_TYPE], st.b[B_FSTATE], st.b[B_ANIMFRAME]), (0, 2, 0));
    }

    /// `$5B1C8` states 2/3: the sprite id walks 122 → 124 on every 4th `$FFF0`
    /// frame, `+28` = 32 is armed with the state change, 32 calls count it down
    /// without touching the id, and the id walks back to 122 (also on 4th
    /// frames), which jumps to the state-0 body and re-arms it.
    #[test]
    fn grower92_sprite_cycle_holds_32_calls_and_rearms() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut st = Staging::default();
        st.b[B_FSTATE] = 2;
        st.b[B_SPRITE] = 122;
        obj.fff0 = 1;
        obj.handler_grower92(&mut st);
        assert_eq!(st.b[B_SPRITE], 122, "only every 4th $FFF0 frame steps");
        obj.fff0 = 0;
        obj.handler_grower92(&mut st);
        assert_eq!(st.b[B_SPRITE], 123);
        obj.fff0 = 4;
        obj.handler_grower92(&mut st);
        assert_eq!((st.b[B_SPRITE], st.b[B_T28], st.b[B_FSTATE]), (124, 0x20, 3));
        for _ in 0..32 {
            obj.handler_grower92(&mut st);
        }
        assert_eq!(
            (st.b[B_SPRITE], st.b[B_T28]),
            (124, 0),
            "the hold is 32 calls"
        );
        obj.fff0 = 5;
        obj.handler_grower92(&mut st);
        assert_eq!(st.b[B_SPRITE], 124, "not a 4th frame");
        obj.fff0 = 8;
        obj.handler_grower92(&mut st);
        assert_eq!(st.b[B_SPRITE], 123);
        obj.fff0 = 12;
        obj.handler_grower92(&mut st);
        assert_eq!(st.b[B_SPRITE], 122);
        assert_eq!(
            (st.b[B_TYPE], st.b[B_FSTATE], st.b[B_T24], st.b[B_ANIMFRAME]),
            (0x4C, 1, 3, 3),
            "the jr $71D3 re-arms state 0"
        );
    }

    /// `$5B241` (anim 94) + the `$7266` tail: state 0 arms type `$48` and state
    /// 1 without stepping; a leg is 18 picker calls summing to 32 px, the last
    /// of which resets `+16` at the `$12` sentinel and toggles `+24` — the
    /// 0 → 1 edge flips the state, so the object floats up on `$74A1` and back
    /// down on `$7488`, and the 1 → 0 edge leaves the state alone. (The capture
    /// trace of level 19's anim-94 slot is exactly this: 176 → 144 → 176.)
    #[test]
    fn floater94_floats_up_then_down_through_the_shared_tail() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 200);
        obj.handler_floater94(&mut st);
        assert_eq!((st.b[B_TYPE], st.b[B_FSTATE]), (0x48, 1));
        assert_eq!(y_of(&st.b), 200, "state 0 takes no step");
        obj.handler_floater94(&mut st);
        assert_eq!((y_of(&st.b), st.b[B_PHASE]), (197, 1), "$74A1[0] = 3 up");
        for _ in 0..17 {
            obj.handler_floater94(&mut st);
        }
        assert_eq!(y_of(&st.b), 168, "32 px up over the leg");
        assert_eq!(
            (st.b[B_PHASE], st.b[B_T24], st.b[B_FSTATE]),
            (0, 1, 2),
            "the sentinel resets the profile, +24 flips the direction"
        );
        for _ in 0..18 {
            obj.handler_floater94(&mut st);
        }
        assert_eq!(y_of(&st.b), 200, "32 px back down on $7488");
        assert_eq!(
            (st.b[B_PHASE], st.b[B_T24], st.b[B_FSTATE]),
            (0, 0, 2),
            "the 1 -> 0 edge does not flip"
        );
    }

    /// `$5B28C` (anim 95): the same machine on X — `$74A1` steps left, `$7488`
    /// steps right — with the shared state-0 body and `$7266` tail.
    #[test]
    fn floater95_shuttles_horizontally_with_the_shared_state0() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut st = Staging::default();
        set_x(&mut st.b, 300);
        obj.handler_floater95(&mut st);
        assert_eq!((st.b[B_TYPE], st.b[B_FSTATE]), (0x48, 1));
        assert_eq!(x_of(&st.b), 300);
        obj.handler_floater95(&mut st);
        assert_eq!((x_of(&st.b), st.b[B_PHASE]), (297, 1), "left, 3 px");
        st.b[B_T24] = 1;
        st.b[B_PHASE] = 11;
        obj.handler_floater95(&mut st);
        assert_eq!((x_of(&st.b), st.b[B_PHASE]), (294, 12), "left on $7488[11]");
        st.b[B_T24] = 0;
        st.b[B_PHASE] = 0x11; // a leg's last call: 17 -> the $12 sentinel
        obj.handler_floater95(&mut st);
        assert_eq!(
            (x_of(&st.b), st.b[B_PHASE], st.b[B_T24], st.b[B_FSTATE]),
            (294, 0, 1, 2),
            "$74A1[17] = 0: no step, reset + flip to state 2"
        );
        st.b[B_PHASE] = 0x11;
        obj.handler_floater95(&mut st);
        assert_eq!((x_of(&st.b), st.b[B_PHASE], st.b[B_T24]), (297, 0, 0));
        assert_eq!(st.b[B_FSTATE], 2, "the 1 -> 0 edge keeps the state");
    }

    /// The two floater profiles, byte-exact from `baserom.gb`: `$7488` (whose
    /// base is `FALL_SPEED` — the c=`$15` fall consumer reads 22 of its bytes)
    /// and `$74A1` (the c=`$12` twin). One leg each is 18 calls and sums to
    /// 32 px.
    #[test]
    fn floater_profiles_are_the_rom_bytes_and_sum_to_32px_each() {
        assert_eq!(
            FALL_SPEED,
            [
                0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x03, 0x02, 0x03,
                0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03
            ],
            "$7488[0..22]"
        );
        assert_eq!(
            STEP_74A1,
            [
                0x03, 0x02, 0x03, 0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x01,
                0x01, 0x01, 0x00, 0x00, 0x00
            ],
            "$74A1[0..19]"
        );
        let left: u32 = STEP_74A1[..18].iter().map(|&v| v as u32).sum();
        let right: u32 = FALL_SPEED[..18].iter().map(|&v| v as u32).sum();
        assert_eq!((left, right), (32, 32), "one leg each");
    }

    /// Level 19's mode-B stream names exactly these four classes; all four now
    /// have a body, so `run_handler` must dispatch them without leaving an
    /// `unported` entry — which is the replay's `note:` line.
    #[test]
    fn level19_anims_dispatch_to_the_new_bodies_unflagged() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json");
        let Ok(txt) = std::fs::read_to_string(path) else {
            eprintln!("skip: {path} not built");
            return;
        };
        let t: ObjectTables = serde_json::from_str(&txt).expect("objects.json parses");
        for (anim, handler) in [(27u8, 0x586EEu32), (92, 0x5B1C8), (94, 0x5B241), (95, 0x5B28C)] {
            assert_eq!(t.behavior_for(19, anim), Some(handler), "level 19 anim {anim}");
            let mut obj = ObjectSim::new(Vec::new(), 0);
            obj.configure(19);
            let mut st = Staging::default();
            st.b[B_STATE] = 2;
            st.b[B_DEF] = anim;
            obj.run_handler(&mut st, &blank_layout(), &t);
            assert!(
                obj.unported.is_empty(),
                "anim {anim} -> ${handler:05X} must not be flagged"
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

    // -----------------------------------------------------------------
    // Level 3 (index 2) — the three new handler classes
    // $59BC5 (anim 88) / $594EB (anim 68) / $58DD1 (anim 46)
    // -----------------------------------------------------------------

    /// A layout of `$60` — no wall (`>= $38`) and no ground (`>= $48`) at any
    /// probe — with one cell overridden. The entity probes read the cell
    /// `(x >> 4, y >> 4)`.
    fn one_tile(x: usize, y: usize, tile: u8) -> LevelLayout {
        let mut cells = vec![0x60u8; LAYOUT_SIZE];
        cells[y * crate::level::LAYOUT_WIDTH + x] = tile;
        LevelLayout::from_bytes(&cells).unwrap()
    }

    /// `$59BC5` (`$5BD1`): the state-0 one-shot — flash `$10`, type `$4E`,
    /// `face_mario` (`$73C6` picks state 1/2 and `$FFDD` 0/`$20` from Mario's
    /// side) and `+28` = 1. The `$7585` bob head runs first on even `$FFF0`:
    /// `+26` cycles and the sprite takes `[FE, 01, FF, 02]`.
    #[test]
    fn groundwalker88_state0_sets_flash_type_faces_mario_and_bobs() {
        let layout = blank_layout();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut st = Staging::default();
        set_x(&mut st.b, 100);
        set_y(&mut st.b, 200);
        st.b[B_SPRITE] = 73; // anim_to_sprite_a[88]
        obj.mario_ref_x = 108; // east of the object: $73C6 -> 2 / $20
        obj.fff0 = 0; // even: the bob head steps
        obj.handler_groundwalker88(&mut st, &layout);
        assert_eq!(st.b[B_FLASH], 0x10);
        assert_eq!(st.b[B_TYPE], 0x4E);
        assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (2, 0x20), "Mario east");
        assert_eq!(st.b[B_T28], 1, "the walk arm is armed");
        assert_eq!((st.b[B_ANIMFRAME], st.b[B_SPRITE]), (1, 74), "bob: +1");

        // Mario west on the same slot's next init call.
        st.b[B_FSTATE] = 0;
        obj.mario_ref_x = 92;
        obj.fff0 = 2; // even again
        obj.handler_groundwalker88(&mut st, &layout);
        assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (1, 0), "Mario west");
        assert_eq!((st.b[B_FLASH], st.b[B_TYPE], st.b[B_T28]), (0x10, 0x4E, 1));
        assert_eq!((st.b[B_ANIMFRAME], st.b[B_SPRITE]), (2, 73), "bob: -1");
        assert!(obj.unported.is_empty());
    }

    /// `$59BC5` states 1/2 (`$5D0F`/`$5D4B`, the `$5D22` tail): the state-0
    /// init leaves `+28` = 1, so the very next call arms `+25` = `$30` and
    /// does not move, the **48** calls after that each take one 1-px step
    /// (the 48th leaving `+28` = `$10`), the next **16** only count (the 16th
    /// re-arming `+25`), and the cycle repeats. The blank layout's `$60`
    /// cells are neither walls nor ground, so nothing interrupts the machine.
    #[test]
    fn groundwalker88_counter_cadence_is_16_idle_calls_then_48_steps() {
        let layout = blank_layout();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.fff0 = 1; // odd: the bob head is a no-op
        let mut st = Staging::default();
        set_x(&mut st.b, 100);
        set_y(&mut st.b, 200);
        obj.mario_ref_x = 92; // west: state 1, walks left
        obj.handler_groundwalker88(&mut st, &layout);
        assert_eq!((st.b[B_FSTATE], st.b[B_T28], st.b[B_T25]), (1, 1, 0));

        obj.handler_groundwalker88(&mut st, &layout); // +28 1 -> 0: arms +25
        assert_eq!(
            (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
            (100, 0, 0x30),
            "the count-to-zero call arms $30 and does not move"
        );
        for _ in 0..47 {
            obj.handler_groundwalker88(&mut st, &layout);
        }
        assert_eq!(
            (x_of(&st.b), st.b[B_T25]),
            (53, 1),
            "47 one-pixel steps, one still budgeted"
        );
        obj.handler_groundwalker88(&mut st, &layout); // the 48th step
        assert_eq!(
            (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
            (52, 0x10, 0),
            "the 48th step exhausts +25 and re-arms the pause"
        );
        for _ in 0..15 {
            obj.handler_groundwalker88(&mut st, &layout);
        }
        assert_eq!(
            (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
            (52, 1, 0),
            "16 countdown calls, no motion"
        );
        obj.handler_groundwalker88(&mut st, &layout);
        assert_eq!(
            (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
            (52, 0, 0x30),
            "the 16th count re-arms the step budget"
        );
        obj.handler_groundwalker88(&mut st, &layout);
        assert_eq!(x_of(&st.b), 51, "and the next call steps again");
    }

    /// `$59BC5` states 1/2: a wall on the moving side flips through `$5D2D`
    /// (`+13 ^= $20`, `+15 ^= 3` — 1 <-> 2) and touches neither counter nor
    /// X. The probes sit at X∓7, Y−11 / Y−5 (`$3C1C`/`$3BB8` -> the port's
    /// `m_left_wall`/`m_right_wall`), so the blocking tile goes in the cell
    /// at `(x∓7) >> 4`.
    #[test]
    fn groundwalker88_side_walls_flip_the_walk_state_in_both_directions() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.fff0 = 1;
        let mut st = Staging::default();
        set_x(&mut st.b, 100);
        set_y(&mut st.b, 200); // the left probes read (5, 11) and (5, 12)
        st.b[B_FSTATE] = 1;
        st.b[B_T28] = 5;
        st.b[B_T25] = 0x30;
        st.b[B_FLIP] = 0;

        let layout = one_tile(5, 11, 0x00); // a wall left of x = 100
        obj.handler_groundwalker88(&mut st, &layout);
        assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (2, 0x20), "1 -> 2, flipped");
        assert_eq!(
            (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
            (100, 5, 0x30),
            "a wall frame touches neither counter nor X"
        );

        // The right probe reads (107 >> 4, y) = the cell (6, ...).
        let layout = one_tile(6, 11, 0x00);
        obj.handler_groundwalker88(&mut st, &layout);
        assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (1, 0), "2 -> 1, unflipped");
        assert_eq!((x_of(&st.b), st.b[B_T28], st.b[B_T25]), (100, 5, 0x30));
        assert!(obj.unported.is_empty());
    }

    /// `$594EB` (anim 68) and `$582D1` (anim 1) are one body: identical init
    /// calls produce byte-identical stagings except `+18` — `$52` vs `$56`,
    /// the one immediate `$54F4` differs by.
    #[test]
    fn fallwalker68_is_the_shared_body_with_type52() {
        let layout = blank_layout();
        let mut a = ObjectSim::new(Vec::new(), 0);
        let mut b = ObjectSim::new(Vec::new(), 0);
        a.fff0 = 1; // odd: the shared bob head is a no-op
        b.fff0 = 1;
        let mut sa = Staging::default();
        let mut sb = Staging::default();
        for st in [&mut sa, &mut sb] {
            set_x(&mut st.b, 100);
            set_y(&mut st.b, 200);
            st.b[B_SPRITE] = 125; // anim_to_sprite_a[68]
        }
        a.handler_fallwalker(&mut sa, &layout);
        b.handler_fallwalker52(&mut sb, &layout);
        assert_eq!((sa.b[B_TYPE], sb.b[B_TYPE]), (0x56, 0x52));
        assert_eq!(sa.b[B_PHASE], 1, "no ground: the fall curve stepped");
        assert_eq!(y_of(&sa.b), 200, "step 0 of the curve is 0 px");
        let (mut ta, mut tb) = (sa.b, sb.b);
        ta[B_TYPE] = 0;
        tb[B_TYPE] = 0;
        assert_eq!(ta, tb, "the whole body but the type byte");
    }

    /// The `run_handler` arms: an in-memory level-2 table routes anims
    /// 46/68/88 to `$58DD1`/`$594EB`/`$59BC5`, each body runs (anim 68's type
    /// is `$52`, not the `$582D1` arm's `$56`) and none is flagged unported —
    /// a flagged handler is the gate's `note:` line.
    #[test]
    fn level3_anims_dispatch_to_the_new_bodies_unflagged() {
        let mut t = tables();
        t.anim_profiles_a[46] = prof(0x58DD1);
        t.anim_profiles_a[68] = prof(0x594EB);
        t.anim_profiles_a[88] = prof(0x59BC5);
        let layout = blank_layout();
        for (anim, handler, ty) in [
            (46u8, 0x58DD1u32, 0x20u8),
            (68, 0x594EB, 0x52),
            (88, 0x59BC5, 0x4E),
        ] {
            let mut obj = ObjectSim::new(Vec::new(), 0);
            obj.configure(2); // level index 2 -> mode A
            let mut st = Staging::default();
            st.b[B_DEF] = anim;
            obj.run_handler(&mut st, &layout, &t);
            assert_eq!(st.b[B_TYPE], ty, "anim {anim} -> ${handler:05X}");
            assert!(obj.unported.is_empty(), "anim {anim} must not be flagged");
        }
    }

    /// `$58DD1` state 0 (`$4DD1`): the one-shot — type `$20` (the on-object
    /// arm: Mario stands on it) and state 2. Nothing else is touched.
    #[test]
    fn carrier46_state0_sets_type20_and_enters_the_wait() {
        let layout = blank_layout();
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut st = Staging::default();
        set_x(&mut st.b, 1712); // level 3's record: (1712, 176)
        set_y(&mut st.b, 176);
        st.b[B_SPRITE] = 35; // anim_to_sprite_a[46]
        obj.handler_carrier46(&mut st, &layout, &t);
        assert_eq!((st.b[B_TYPE], st.b[B_FSTATE]), (0x20, 2));
        assert_eq!((y_of(&st.b), st.b[B_T28]), (176, 0), "no move, no timer");
        assert_eq!(obj.done, 0);
        assert!(obj.unported.is_empty());
    }

    /// `$58DD1` state 2 (`$4E04`): `+23` bit 3 — the `$2DF7` on-object flag
    /// the port's `respond` stages — is the only gate; any other flag bit
    /// leaves it waiting. The handover call arms state 1 with `+28` = 28 and
    /// itself neither moves nor carries.
    #[test]
    fn carrier46_state2_waits_for_the_stood_on_flag() {
        let layout = blank_layout();
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.mario.y = 500;
        let mut st = Staging::default();
        set_y(&mut st.b, 176);
        st.b[B_FSTATE] = 2;
        st.b[B_FLAGS] = 0x04; // some other interaction flag
        obj.handler_carrier46(&mut st, &layout, &t);
        assert_eq!(
            (st.b[B_FSTATE], st.b[B_T28], y_of(&st.b)),
            (2, 0, 176),
            "bit 3 clear: still waiting"
        );
        st.b[B_FLAGS] = 0x08; // stood on this frame
        obj.handler_carrier46(&mut st, &layout, &t);
        assert_eq!((st.b[B_FSTATE], st.b[B_T28]), (1, 0x1C));
        assert_eq!(
            (y_of(&st.b), obj.mario_y_adjust),
            (176, None),
            "the handover call does not move or carry"
        );
        assert_eq!(obj.done, 0);
    }

    /// `$58DD1` state 1 (`$4DE5`/`$4DEE`): `+28` = 28 counts down — those
    /// calls do nothing and the call that takes it to 0 falls *through* to
    /// the move — then every call steps the `$7488` curve (`$7451`), moves
    /// the object down (`$3D3D`), carries Mario by the same `b` (`$3CDF`,
    /// only while `+23` bit 3 is held) and probes the ground (`$3B40`).
    #[test]
    fn carrier46_counts_down_then_falls_and_carries_mario() {
        let layout = blank_layout(); // $60: no ground anywhere
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.mario.y = 500;
        let mut st = Staging::default();
        set_y(&mut st.b, 400);
        st.b[B_FSTATE] = 1;
        st.b[B_T28] = 3;
        st.b[B_PHASE] = 2; // $7488[2] = 1 px
        st.b[B_FLAGS] = 0x08;
        for _ in 0..2 {
            obj.handler_carrier46(&mut st, &layout, &t);
        }
        assert_eq!(
            (y_of(&st.b), st.b[B_T28], st.b[B_PHASE]),
            (400, 1, 2),
            "the countdown calls do not move"
        );
        obj.handler_carrier46(&mut st, &layout, &t); // +28 1 -> 0: the move
        assert_eq!(
            (y_of(&st.b), st.b[B_T28], st.b[B_PHASE]),
            (401, 0, 3),
            "the count-to-zero call falls through"
        );
        assert_eq!(obj.mario_y_adjust, Some(501), "carried by b = 1");
        // +28 already 0: the next call moves immediately, and the carry adds
        // onto the Y staged this frame (the ROM reads the live $FFC0).
        obj.handler_carrier46(&mut st, &layout, &t);
        assert_eq!((y_of(&st.b), st.b[B_PHASE]), (402, 4));
        assert_eq!(obj.mario_y_adjust, Some(502));
        // A fresh frame (the $2D7D pass resets the Option) with Mario off:
        // the object keeps falling, the carry stays closed.
        obj.mario_y_adjust = None;
        st.b[B_FLAGS] = 0;
        obj.handler_carrier46(&mut st, &layout, &t);
        assert_eq!((y_of(&st.b), obj.mario_y_adjust), (403, None));
        assert_eq!(obj.done, 0, "no ground, no removal");
    }

    /// `$58DD1` state 1's terminal step: with ground under the object the
    /// `$3B40` probe keeps bit 3 and the `$72D7` removal runs — the staging
    /// zeroes, the active count drops, `sObjectDone = 2` and (remove-mode 0)
    /// a stream-backed slot clears its record marker.
    #[test]
    fn carrier46_grounded_step_end_removes_itself_and_clears_the_marker() {
        let t = tables();
        let mut obj = ObjectSim::new(records(), 0);
        obj.active = 1;
        obj.spawned[0] = true;
        let layout = one_tile(6, 25, 0x00); // ground under (96, 400)
        let mut st = Staging::default();
        set_x(&mut st.b, 96);
        set_y(&mut st.b, 400); // Y & $0C == 0: the probe is taken
        st.b[B_STATE] = 2;
        st.b[B_DEF] = 46;
        st.b[B_FSTATE] = 1;
        st.b[B_T28] = 0; // the countdown is already spent
        st.b[B_FLAGS] = 0x08;
        st.b[B_RECHI] = 0xAB; // record 0 of the stream buffer
        st.b[B_RECLO] = 0x06;
        obj.handler_carrier46(&mut st, &layout, &t);
        assert_eq!(obj.done, 2, "$72D7 sets sObjectDone = 2");
        assert_eq!(st.b, [0u8; 32], "the removal zeroes the staging");
        assert_eq!(obj.active, 0);
        assert!(!obj.spawned[0], "remove-mode 0 clears the record marker");
        assert_eq!(obj.remove_mode, 0, "$5B2D7 resets the mode");
    }

    /// `$3CDF`/`$3CD7`: the carry adds `b` to Mario's **16-bit** Y — the high
    /// byte takes the `adc` — gated on `+23` bit 3; a Y already staged this
    /// frame is the base, and with the bit clear the byte is untouched.
    #[test]
    fn carry_adds_b_to_marios_16bit_y_only_when_stood_on() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.mario.y = 0x20FF;
        let mut st = Staging::default();
        st.b[B_FLAGS] = 0x08;
        obj.carry_mario(&mut st, 3);
        assert_eq!(obj.mario_y_adjust, Some(0x2102), "carry into the high byte");
        obj.carry_mario(&mut st, 2);
        assert_eq!(obj.mario_y_adjust, Some(0x2104), "stacks on the staged Y");
        st.b[B_FLAGS] = 0x00;
        obj.mario_y_adjust = None;
        obj.carry_mario(&mut st, 9);
        assert_eq!(obj.mario_y_adjust, None, "bit 3 clear: nothing");
    }

    // -----------------------------------------------------------------
    // $2FED / $3007 — the fireball-vs-object pass
    // -----------------------------------------------------------------

    /// Every slot boxed around (0x50, 0x50) with a 0x10 hitbox: `$3007`'s
    /// expansion is `top - 6`, `bottom + 6` (and the same on X), so the window
    /// is `(sy, sx)` in `0x4A..0x66`.
    fn fire_slot(obj: &mut ObjectSim, i: usize, ty: u8, flags: u8) {
        let s = &mut obj.slots[i];
        s.b[B_TYPE] = ty;
        s.b[B_FLAGS] = flags;
        s.b[B_SCRY] = 0x50;
        s.b[B_SCRX] = 0x50;
        s.b[B_HB_TOP] = 0;
        s.b[B_HB_BOTTOM] = 0x10;
        s.b[B_HB_LEFT] = 0;
        s.b[B_HB_RIGHT] = 0x10;
    }

    /// `$2FED`/`$3007`: a bit-2 type is *defeated* (bit 1 of `+17`, the flag
    /// `$85AA`'s dispatch turns into `defeat()`), everything else just takes
    /// the sfx arm — and both leave `sObjectHitResult` non-zero, which is what
    /// kills the ball. The window test is byte-wise: the far edge misses.
    #[test]
    fn fire_hit_defeats_bit2_types_and_dies_on_the_rest() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        fire_slot(&mut obj, 0, 0x24, 0); // bit 2 set: fireball-defeatable
        assert_eq!(obj.fire_hit(0x4A, 0x4A), FireHit::Defeat);
        assert_eq!(obj.slots[0].b[B_FLAGS], 0x02);
        obj.slots[0].b[B_FLAGS] = 0;
        assert_eq!(obj.fire_hit(0x4A, 0x66), FireHit::None, "one past the right edge");
        assert_eq!(obj.fire_hit(0x66, 0x4A), FireHit::None, "one past the bottom edge");
        assert_eq!(obj.fire_hit(0x49, 0x4A), FireHit::None, "one short of the top");
        assert_eq!(obj.slots[0].b[B_FLAGS], 0, "a miss writes nothing");

        fire_slot(&mut obj, 0, 0x20, 0); // bit 2 clear
        assert_eq!(obj.fire_hit(0x4A, 0x4A), FireHit::Die);
        assert_eq!(obj.slots[0].b[B_FLAGS], 0, "no defeat for a bit-2-clear type");
    }

    /// The scan's own gates: the flags byte must be exactly 0 — bit 3
    /// included, `$2FED` has no `$2D7D`-style `and $F7` — and a bit-7 type is
    /// an item pose, transparent to a ball.
    #[test]
    fn fire_hit_skips_flagged_slots_items_and_empty_slots() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        fire_slot(&mut obj, 0, 0x24, 0x08); // stood on: not hittable
        fire_slot(&mut obj, 1, 0x82, 0x00); // the item pose
        fire_slot(&mut obj, 2, 0x00, 0x00); // empty
        fire_slot(&mut obj, 3, 0x80, 0x00); // the base item type
        assert_eq!(obj.fire_hit(0x4A, 0x4A), FireHit::None);
        for (i, want) in [(0usize, 0x08u8), (1, 0), (2, 0), (3, 0)] {
            assert_eq!(obj.slots[i].b[B_FLAGS], want, "slot {i}");
        }
    }

    /// The scan keeps walking after a hit, exactly like `$2FED`'s loop, so the
    /// *last* hit is the verdict `sObjectHitResult` holds — and an earlier
    /// defeat is not undone by it.
    #[test]
    fn fire_hit_keeps_the_last_verdict_and_still_defeats() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        fire_slot(&mut obj, 0, 0x24, 0); // defeat
        fire_slot(&mut obj, 1, 0x20, 0); // die
        assert_eq!(obj.fire_hit(0x4A, 0x4A), FireHit::Die, "the last hit wins");
        assert_eq!(obj.slots[0].b[B_FLAGS], 0x02, "the earlier defeat stands");
    }

    // ---- Bank 2 $8DFD / $8E6C boss spawn staging unit tests (U1) -----------

    #[test]
    fn boss_spawn_clears_pages_and_populates_metadata() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.boss_slots_raw.fill(0xAA);
        obj.slots[0].b.fill(0xFF);
        obj.spawn_boss(0);

        assert_eq!(obj.boss_hits_remaining, 6);
        assert_eq!(obj.boss_sprite, 1);
        assert_eq!(obj.boss_timer_reload, 80);
        assert_eq!(obj.boss_slots_raw[0..5], [0, 0, 0, 0, 0]);
        let (_, _, _, obj0, _) = crate::scene::lookup_boss_spawn_def(0);
        assert_eq!(&obj.boss_slots_raw[5..23], &obj0);
        assert_eq!(&obj.slots[0].b[5..23], &obj0);
    }

    #[test]
    fn boss_spawn_count_1_stages_one_sub_without_spill() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.spawn_boss(0); // def 0: count 1

        let (_, _, _, _, subs) = crate::scene::lookup_boss_spawn_def(0);
        assert_eq!(&obj.boss_slots_raw[0x25..0x25 + 18], &subs[0]);
        assert_eq!(obj.boss_slots_raw[0x25 + 18], 0);
        assert_eq!(obj.slots[2].b, [0; 32]);
    }

    #[test]
    fn boss_spawn_def_index_4_stages_literal_slot_bytes_with_10_stride() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.spawn_boss(4); // def 4: count 2, stride +$10

        // Test literal slot bytes from §4.3 of deleg_C_scene_boss_verdict.md:
        assert_eq!(
            &obj.slots[0].b,
            &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x30, 0x90, 0x34, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00
            ]
        );
        assert_eq!(
            &obj.slots[1].b,
            &[
                0x00, 0x00, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x30, 0x90, 0x0E, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x40, 0x98, 0x0E,
                0x00, 0x00, 0x00, 0x00
            ]
        );
        assert_eq!(
            &obj.slots[2].b,
            &[0; 32]
        );
    }

    #[test]
    fn boss_spawn_index_8_stages_no_subs_but_keeps_obj0() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.spawn_boss(8); // count 0

        let (count, _, _, obj0, _) = crate::scene::lookup_boss_spawn_def(8);
        assert_eq!(count, 0);
        assert_eq!(&obj.slots[0].b[5..23], &obj0);
        assert_eq!(obj.slots[1].b, [0; 32]);
        assert_eq!(obj.slots[2].b, [0; 32]);
    }

    // ---- $8611 / $5839B / $58444 — the carry, the kick and the KO ---------

    /// `$8611` with B held: the one-shot `sActionLatch`, anim += 2 (the stunned
    /// shell's 2 -> the carried 4) and `$863C`, whose `$8240` clears `+14..+30`
    /// — the type byte with them, which is what leaves the carried object
    /// outside the KO scan. A second contact cannot re-trigger it.
    #[test]
    fn the_8611_pickup_takes_the_shell_to_the_carried_anim() {
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        let mut st = streamed(2, 0x06, 0x10);
        st.b[B_FSTATE] = 1;
        let contact = MarioContact {
            b_held: true,
            ..Default::default()
        };
        obj.post_touch(&mut st, &[0u8; 48], &t, &contact);
        assert_eq!(st.b[B_DEF], 4, "anim 2 + 2 = the carried state");
        assert_eq!(obj.action_latch, 1, "the latch arms");
        assert_eq!(st.b[B_TYPE], 0, "$8240 cleared +14..+30, type included");
        assert_eq!(st.b[B_FSTATE], 0, "and the state byte");
        assert_eq!(obj.done, 1, "$863C sets sObjectDone");
        assert!(obj.unported.is_empty(), "$8611 is ported now");

        // The latch holds: a second B-held contact does nothing.
        let mut st2 = streamed(4, 0x00, 0x10);
        obj.post_touch(&mut st2, &[0u8; 48], &t, &contact);
        assert_eq!(st2.b[B_DEF], 4, "no second bump");
    }

    /// `$8611` with B *not* held: `$8633` bumps the anim to 3 (the kicked
    /// walker) and the `$864A` tail nudges the shell 16 px to Mario's facing
    /// side — open air there means it just walks off, no type change.
    #[test]
    fn the_8611_kick_places_the_shell_on_marios_facing_side() {
        let t = tables();
        let layout = blank_layout();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.active = 1;
        obj.spawned = vec![true];
        let mario = MarioContact {
            x: 100,
            y: 100,
            dir_right: true,
            ..Default::default()
        };
        obj.mario = mario;
        obj.mario_ref_x = 108;
        {
            let s = &mut obj.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_DEF] = 2;
            s.b[B_TYPE] = 0x06;
            s.b[B_FLAGS] = 0x10;
            set_x(&mut s.b, 100);
            set_y(&mut s.b, 100);
        }
        // Through the engine, so the $864A tail runs with the layout.
        obj.engine(0, 0, &layout, &t);
        let s = &obj.slots[0];
        assert_eq!(s.b[B_DEF], 3, "$8633: anim += 1");
        assert_eq!(s.b[B_TYPE], 0x06, "no wall: the type stays");
        assert_eq!(s.x(), 116, "nudged 16 px to Mario's right");
    }

    /// The same tail with a wall on Mario's facing side: the shell is put back
    /// and becomes a *live* kicked shell — type `$5F`, walking into the wall
    /// (state 1), which the anim-3 handler turns around next frame. The probe
    /// runs from the *nudged* position, i.e. (x + 16 + 7, y - 8).
    #[test]
    fn a_wall_on_marios_facing_side_makes_a_live_kicked_shell() {
        let mut layout = blank_layout();
        layout.cells[5 * 256 + 7] = 0x20; // the nudged (x+7, y-8) probe's cell
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.active = 1;
        obj.spawned = vec![true];
        obj.mario = MarioContact {
            x: 100,
            y: 100,
            dir_right: true,
            ..Default::default()
        };
        obj.mario_ref_x = 108;
        {
            let s = &mut obj.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_DEF] = 2;
            s.b[B_TYPE] = 0x06;
            s.b[B_FLAGS] = 0x10;
            set_x(&mut s.b, 100);
            set_y(&mut s.b, 100);
        }
        obj.engine(0, 0, &layout, &t);
        let s = &obj.slots[0];
        assert_eq!(s.b[B_DEF], 3);
        assert_eq!(s.b[B_TYPE], 0x5F, "a live kicked shell");
        assert_eq!(s.b[B_FSTATE], 1, "walking into the wall");
        assert_eq!(s.x(), 100, "put back where it was");
    }

    /// `$5839B` (anim 3): the kicked shell walks 3 px a frame *away* from Mario
    /// (`$5B3C6` puts Mario-to-the-right at state 2 = the left walk), turns at a
    /// wall, steps off a ledge into the `$5B451` fall and lands back into the
    /// walk with the fall index cleared and Y snapped to the tile.
    #[test]
    fn the_kicked_shell_walks_turns_falls_and_lands() {
        let mut layout = blank_layout();
        layout.cells[6 * 256..=6 * 256 + 20].fill(0x20); // ground under it
        let mut st = streamed(3, 0x00, 0);
        st.b[B_FSTATE] = 0;
        set_x(&mut st.b, 100);
        set_y(&mut st.b, 96); // (y & $0C) == 0: the ground probe is live
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.mario = MarioContact {
            x: 300,
            ..Default::default()
        };
        obj.mario_ref_x = 308; // Mario to the right
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.b[B_TYPE], 0x5F, "state 0 arms the shell type");
        assert_eq!(st.b[B_FSTATE], 2, "$5B3C6: Mario right -> the left walk");

        // State 2 walks left 3 px while the ground probe holds.
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.x(), 97, "3 px left, away from Mario");
        assert_eq!(st.b[B_FSTATE], 2);

        // A left wall turns it (state 2 -> 1).
        layout.cells[5 * 256 + 5] = 0x20; // (x-7=90, y-8=88) -> row 5 col 5
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.b[B_FSTATE], 1, "turned at the wall");
        assert_eq!(st.x(), 97, "and it did not step into it");

        // State 1 walks right (away from the wall) before the ledge.
        layout.cells[5 * 256 + 5] = 0x60;
        let x_wall = st.x();
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.x(), x_wall + 3, "state 1 walks right");

        // Walking right with no ground under it -> the fall state (+2 = 3).
        layout.cells[6 * 256..=6 * 256 + 20].fill(0x60);
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.b[B_FSTATE], 3, "no ground: state += 2");

        // Falling: Y += the $5B451 curve + 1, and the index steps.
        let y0 = st.y();
        obj.handler_kicked(&mut st, &layout);
        assert!(st.y() > y0, "it falls");
        assert_eq!(st.b[B_PHASE], 1, "the fall index stepped");

        // Land it: ground back, state -= 2 into the walk.
        layout.cells[6 * 256..=6 * 256 + 20].fill(0x20);
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.b[B_FSTATE], 1, "landed back into the right walk");
        assert_eq!(st.b[B_PHASE], 0, "the fall index clears");
        assert_eq!(st.b[B_YLO] & 0x0F, 0, "and Y snaps to the tile");
    }

    fn layout_for_ko() -> LevelLayout {
        blank_layout()
    }

    /// `$5B697`: the kicked shell defeats an enemy it overlaps — the target's
    /// `+17` bit 1 (the flag the interaction dispatch turns into `defeat()`)
    /// plus `sObjectHitFlag`. Type `$5F` (itself, another shell), bit 7 item
    /// poses and bit 5 types are all skipped, and the box is the target's own
    /// hitbox widened by 3/10 on Y and 5 on X.
    #[test]
    fn the_kicked_shell_knocks_out_another_enemy() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        let mut st = streamed(3, 0x5F, 0);
        set_x(&mut st.b, 100);
        set_y(&mut st.b, 100);
        st.b[B_SCRY] = 96;
        st.b[B_SCRX] = 96;
        for (i, ty) in [(0usize, 0x4Eu8), (1, 0x5F), (2, 0x82), (3, 0x0A)] {
            let s = &mut obj.slots[i];
            s.b[B_STATE] = 2;
            s.b[B_TYPE] = ty;
            s.b[B_SCRY] = 90;
            s.b[B_SCRX] = 96;
            s.b[B_HB_TOP] = 0;
            s.b[B_HB_BOTTOM] = 0x10;
            s.b[B_HB_LEFT] = 0;
            s.b[B_HB_RIGHT] = 0x10;
        }
        // Through the handler, so the `bit 6` gate is covered too.
        obj.handler_kicked(&mut st, &layout_for_ko());
        assert_eq!(obj.slots[0].b[B_FLAGS], 0x02, "the walker is defeated");
        assert_eq!(obj.slots[1].b[B_FLAGS], 0, "another shell is not");
        assert_eq!(obj.slots[2].b[B_FLAGS], 0, "an item pose is not");
        assert_eq!(obj.slots[3].b[B_FLAGS], 0x02, "a bit-5/7-clear enemy is");
        assert_eq!(obj.object_hit_flag, 1, "and the hit flag rises");

        // With bit 6 clear (a `$06` shell that never went through state 0) the
        // KO scan does not even run.
        for i in 0..4 {
            obj.slots[i].b[B_FLAGS] = 0;
        }
        st.b[B_TYPE] = 0x06;
        obj.handler_kicked(&mut st, &layout_for_ko());
        assert_eq!(obj.slots[0].b[B_FLAGS], 0, "bit 6 clear: no KO");
        st.b[B_TYPE] = 0x5F;

        // Far enough out on X (the box is ~26 px wide): nothing.
        obj.object_hit_flag = 0;
        obj.slots[0].b[B_FLAGS] = 0;
        obj.slots[3].b[B_FLAGS] = 0;
        for i in 0..4 {
            obj.slots[i].b[B_SCRX] = 96u8.wrapping_add(0x30);
        }
        obj.ko_overlap(&st);
        assert_eq!(obj.slots[0].b[B_FLAGS], 0, "out of the window");
        assert_eq!(obj.object_hit_flag, 0);
    }

    /// `$581B1` (anim 0): the death animation a defeat runs into — the enemy
    /// hops 20 px up and flips upside down, rises on the `$74AB` curve at 2 b a
    /// frame, then falls on the `$5B451` curve and is removed once its staged
    /// screen Y passes `$A0`. The engine now routes anim 0 here, so a defeat
    /// raises no `unported` note either.
    #[test]
    fn a_defeated_enemy_flips_up_and_falls_off_the_screen() {
        let t = tables();
        let layout = blank_layout();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        let mut st = streamed(0, 0x00, 0);
        st.b[B_FSTATE] = 0;
        st.b[B_SPRITE] = 3;
        st.b[B_TYPE] = 0x00;
        set_x(&mut st.b, 100);
        set_y(&mut st.b, 100);
        obj.handler_death(&mut st, &layout, &t);
        assert_eq!(st.y(), 80, "the 20 px hop (Y -= 20)");
        assert_eq!(st.b[B_FLIP], 0x40, "flipped upside down");
        assert_eq!(st.b[B_FSTATE], 1, "and rising");
        assert_eq!(st.b[B_TYPE], 0);

        // $581CE: 2x the $74AB curve, one byte a frame.
        let y0 = st.y();
        obj.handler_death(&mut st, &layout, &t);
        assert_eq!(st.y(), y0 - 4, "$74AB[0] = 2, doubled by the sla");
        for _ in 0..7 {
            obj.handler_death(&mut st, &layout, &t);
        }
        assert_eq!(st.b[B_FSTATE], 2, "the curve's sentinel ends the rise");
        assert_eq!(st.b[B_PHASE], 0, "and resets the fall index");

        // $581E2: it falls until the draw's staged screen Y passes $A0.
        let mut frames = 0;
        while st.b[B_STATE] != 0 && frames < 80 {
            st.b[B_SCRY] = st.b[B_SCRY].wrapping_add(6); // the draw step
            obj.handler_death(&mut st, &layout, &t);
            frames += 1;
        }
        assert_eq!(st.b[B_STATE], 0, "removed once off the bottom");
        assert_eq!(obj.remove_mode, 0, "$5B2D7 resets the mode");
    }

    /// `$C014`: the bank-3 draw returns before drawing a slot whose
    /// `sObjectDone` is set, which is why a defeated enemy disappears instead
    /// of freezing where it stood — `$85FC` (the defeat) sets that byte.
    #[test]
    fn a_defeated_slot_is_not_drawn() {
        let layout = blank_layout();
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.active = 1;
        obj.spawned = vec![true];
        obj.mario = MarioContact::default();
        {
            let s = &mut obj.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_DEF] = 9;
            s.b[B_TYPE] = 0x4E;
            s.b[B_FSTATE] = 1;
            s.b[B_SPRITE] = 3;
            set_x(&mut s.b, 100);
            set_y(&mut s.b, 96);
        }
        obj.engine(0, 0, &layout, &t);
        assert!(!obj.slot_done[0], "a live object draws");
        assert_eq!(obj.slots[0].b[B_TYPE], 0x4E);

        // The fireball's flag (bit 1) -> the dispatch's defeat -> `$85FC` sets
        // `sObjectDone`, and `$C014` stops the draw on that very frame.
        obj.slots[0].b[B_FLAGS] = 0x02;
        obj.engine(0, 0, &layout, &t);
        assert_eq!(obj.slots[0].b[B_TYPE], 0, "defeated");
        assert_eq!(obj.slots[0].b[B_DEF], 0);
        assert!(obj.slot_done[0], "and it is not drawn");
    }

    /// The same chain through the engine, with the *committed* extraction's
    /// anim-0 handler ($581B1, the death animation): the frames after a
    /// fireball defeat flip the enemy, hop it up and draw it again as it falls,
    /// instead of freezing the sprite and flagging anim 0 as an unported gap.
    #[test]
    fn a_fireball_defeat_plays_the_death_animation_through_the_engine() {
        let real: ObjectTables = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../extracted/objects.json"
            ))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            real.profile_for(0, 0).map(|p| p.handler),
            Some(0x581B1),
            "the extraction's anim 0 IS the death handler"
        );
        let mut t = tables();
        t.anim_profiles_a[0] = prof(0x581B1);

        let layout = blank_layout();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.active = 1;
        obj.spawned = vec![true];
        obj.mario = MarioContact::default();
        {
            let s = &mut obj.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_DEF] = 9;
            s.b[B_TYPE] = 0x4E;
            s.b[B_FSTATE] = 1;
            s.b[B_SPRITE] = 3;
            s.b[B_FLAGS] = 0x02; // the fireball's defeat flag
            set_x(&mut s.b, 100);
            set_y(&mut s.b, 96);
        }
        obj.engine(0, 0, &layout, &t);
        assert_eq!(obj.slots[0].b[B_TYPE], 0, "defeated");
        assert!(obj.slot_done[0], "and not drawn on the defeat frame");

        let y0 = y_of(&obj.slots[0].b);
        for _ in 0..3 {
            obj.engine(0, 0, &layout, &t);
        }
        assert!(obj.unported.is_empty(), "anim 0 is dispatched, not a gap");
        assert!(y_of(&obj.slots[0].b) < y0, "it hops up (Y -= 20)");
        assert_eq!(obj.slots[0].b[B_FLIP] & 0x40, 0x40, "flipped upside down");
        assert!(!obj.slot_done[0], "and it is drawn again while it falls");
    }

    /// The whole koopa chain the play test exercised: a stunned shell (anim 2,
    /// type `$06`) is picked up with B held (`$8611` -> anim 4, carried),
    /// follows Mario, is released (anim 4 -> 3) and then *walks* as the kicked
    /// walker instead of hanging where it was thrown.
    #[test]
    fn the_shell_chain_pickup_carry_release_walks() {
        let mut layout = blank_layout();
        layout.cells[6 * 256..=6 * 256 + 24].fill(0x20); // ground at row 6
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        // Mario's Y is 91 so that the carried offset (+5 for big Mario) leaves
        // the shell at Y = 96 — row 6, the ground row — while its wall probes
        // (y-8 = 88) read row 5, the air above it.
        obj.mario = MarioContact {
            x: 100,
            y: 91,
            powerup: 1,
            b_held: true,
            dir_right: true,
            ..Default::default()
        };
        obj.mario_ref_x = 108;
        // The stunned shell beside Mario.
        {
            let s = &mut obj.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_DEF] = 2;
            s.b[B_TYPE] = 0x06;
            s.b[B_FSTATE] = 1;
            set_x(&mut s.b, 120);
            set_y(&mut s.b, 96);
        }
        // 1. B-held contact: the pickup.
        let mut st = Staging {
            b: obj.slots[0].b,
            ..Default::default()
        };
        st.b[B_FLAGS] = 0x10;
        let contact = obj.mario;
        obj.post_touch(&mut st, &[0u8; 48], &t, &contact);
        assert_eq!(st.b[B_DEF], 4, "the shell is carried");
        assert_eq!(obj.action_latch, 1);

        // 2. Carried: the first frame only initialises the carried state
        //    ($58452: type 0, state 1), from then on it follows Mario
        //    (+8 X, +5 Y for big Mario).
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!(st.b[B_FSTATE], 1, "$58452: type 0, state 1");
        assert_eq!(st.b[B_TYPE], 0);
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!((st.x(), st.y()), (108, 96), "pinned to Mario");
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!((st.x(), st.y()), (108, 96), "and it stays there");

        // 3. B released: anim 3 and the placement nudge.
        obj.mario.b_held = false;
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!(st.b[B_DEF], 3, "the kicked walker");
        assert_eq!(st.x(), 124, "nudged 16 px out of Mario");

        // 4. And it *moves* — the bug was that it froze mid-air instead. A
        //    release in open air keeps type 0 and the state the carried arm
        //    left (1 = the right walk); the ROM arms the live `$5F` type only
        //    from the anim-3 state-0 arm or a wall placement.
        let x0 = st.x();
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.b[B_TYPE], 0, "open-air release: still inert");
        assert_eq!(st.x(), x0 + 3, "but it walks 3 px");
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.x(), x0 + 6, "and keeps walking");

        // 5. Off the ledge: it falls, then lands back into the walk.
        layout.cells[6 * 256..=6 * 256 + 24].fill(0x60);
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.b[B_FSTATE], 3, "no ground: state += 2 (falling right)");
        let y0 = st.y();
        obj.handler_kicked(&mut st, &layout);
        assert!(st.y() > y0, "and it falls rather than hanging");
        layout.cells[6 * 256..=6 * 256 + 24].fill(0x20);
        obj.handler_kicked(&mut st, &layout);
        assert_eq!(st.b[B_FSTATE], 1, "landed, walking again");
        assert_eq!(st.b[B_PHASE], 0, "with the fall index cleared");
    }

    /// `$58444` (anim 4): while B is held the carried shell is pinned to Mario
    /// (`$5B4C1`: his X + 8 and Y + 5 big / 14 small / 20 small+ducking), and on
    /// release it drops to anim 3 (via `$3AB4`) and runs the placement tail.
    #[test]
    fn the_carried_shell_follows_mario_and_releases_into_a_kick() {
        let layout = blank_layout();
        let t = tables();
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.action_latch = 1;
        obj.mario = MarioContact {
            x: 200,
            y: 300,
            powerup: 1,
            b_held: true,
            dir_right: true,
            ..Default::default()
        };
        let mut st = streamed(4, 0x00, 0);
        st.b[B_FSTATE] = 1;
        set_x(&mut st.b, 0);
        set_y(&mut st.b, 0);
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!(st.x(), 208, "Mario's X + 8");
        assert_eq!(st.y(), 305, "and Y + 5 for big Mario");

        // Small and ducking takes the 20 px offset.
        obj.mario.powerup = 0;
        obj.mario.ducking = true;
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!(st.y(), 320, "small ducking: Y + 20");

        // Release: anim 4 -> 3 plus the placement tail (no wall -> the shell
        // stays anim 3, 16 px further out than the carried position).
        obj.mario.ducking = false;
        obj.mario.powerup = 1;
        obj.mario.b_held = false;
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!(st.b[B_DEF], 3, "the release drops the anim");
        assert_eq!(obj.action_latch, 0, "the latch clears");
        assert_eq!(st.x(), 224, "and the placement nudges +16 (no wall)");
        assert_eq!(st.b[B_TYPE], 0, "open air: no live-shell type");
    }

    /// The carried shell's exit arms: an object it knocked out
    /// (`sObjectHitFlag`) and a ceiling inside the `$3B68` probe both defeat the
    /// shell itself (`$584C3` -> `$3ADC` = `$85FC`), and a pipe ride removes it.
    #[test]
    fn the_carried_shell_is_defeated_by_a_hit_or_a_ceiling() {
        let layout = blank_layout();
        let t = tables();
        // An enemy under the carried shell's staged screen pair.
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.mario = MarioContact {
            x: 200,
            y: 300,
            powerup: 1,
            b_held: true,
            ..Default::default()
        };
        let mut st = streamed(4, 0x00, 0);
        st.b[B_FSTATE] = 1;
        st.b[B_SCRY] = 96;
        st.b[B_SCRX] = 96;
        {
            let s = &mut obj.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_TYPE] = 0x4E;
            s.b[B_SCRY] = 90;
            s.b[B_SCRX] = 96;
            s.b[B_HB_BOTTOM] = 0x10;
            s.b[B_HB_RIGHT] = 0x10;
        }
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!(st.b[B_DEF], 0, "$584C3 defeated the shell");
        assert_eq!(st.b[B_TYPE], 0);
        assert_eq!(obj.action_latch, 0, "and cleared the latch");

        // A ceiling above it does the same.
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.mario = MarioContact {
            x: 200,
            y: 300,
            powerup: 1,
            b_held: true,
            ..Default::default()
        };
        let mut layout2 = blank_layout();
        layout2.cells[18 * 256 + 13] = 0x20; // the (x, y-8) probe: the followed (208, 297)
        let mut st = streamed(4, 0x00, 0);
        st.b[B_FSTATE] = 1;
        obj.handler_carried(&mut st, &layout2, &t);
        assert_eq!(st.b[B_DEF], 0, "the $3B68 ceiling defeats it");
        assert_eq!(obj.action_latch, 0);

        // A pipe ride removes it instead.
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(0);
        obj.action_latch = 1;
        obj.mario = MarioContact {
            travelling: true,
            ..Default::default()
        };
        let mut st = streamed(4, 0x00, 0);
        st.b[B_STATE] = 2;
        obj.handler_carried(&mut st, &layout, &t);
        assert_eq!(st.b[B_STATE], 0, "$5B2D7 removal");
        assert_eq!(obj.action_latch, 0);
    }
}
