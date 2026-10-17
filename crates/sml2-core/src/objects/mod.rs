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
use helpers::bcd_add;

mod staging;
pub use staging::*;
use staging::{BOUNCE_SPEED, DEATH_RISE_74AB, FALL_SPEED, HOVER_SPEED, SPIN_WOBBLE, STEP_74A1};
mod boss;
mod boss_flash;
mod boss_stun;
#[cfg(test)]
mod boss_flash_paths;
mod boss_damage;
mod boss_spawn;
mod boss_sprite_cycle;
mod boss_reverse_picker;
#[cfg(test)]
mod boss_screen_input_tests;
mod dispatch;
mod terrain_probes;
mod handler_0x58848;
mod handler_0x58848_adapter;
#[path = "handler_0x58B43.rs"]
mod handler_0x58b43;
#[path = "handler_0x58B43_adapter.rs"]
mod handler_0x58b43_adapter;
#[path = "handler_0x5902D.rs"]
mod handler_0x5902d;
#[path = "handler_0x5902D_adapter.rs"]
mod handler_0x5902d_adapter;
#[path = "handler_0x5934A.rs"]
mod handler_0x5934a;
#[path = "handler_0x5934A_adapter.rs"]
mod handler_0x5934a_adapter;
#[path = "handler_0x593DA.rs"]
mod handler_0x593da;
#[path = "handler_0x593DA_adapter.rs"]
mod handler_0x593da_adapter;
#[path = "handler_0x59431.rs"]
mod handler_0x59431;
#[path = "handler_0x59431_adapter.rs"]
mod handler_0x59431_adapter;
#[path = "handler_0x5945A.rs"]
mod handler_0x5945a;
#[path = "handler_0x5945A_adapter.rs"]
mod handler_0x5945a_adapter;
#[path = "handler_0x59487.rs"]
mod handler_0x59487;
#[path = "handler_0x59487_adapter.rs"]
mod handler_0x59487_adapter;
#[path = "handler_0x589A6.rs"]
mod handler_0x589a6;
#[path = "handler_0x589A6_adapter.rs"]
mod handler_0x589a6_adapter;
#[path = "handler_0x58A6F.rs"]
mod handler_0x58a6f;
#[path = "handler_0x58A6F_adapter.rs"]
mod handler_0x58a6f_adapter;
#[path = "handler_0x58BBA.rs"]
mod handler_0x58bba;
#[path = "handler_0x58BBA_adapter.rs"]
mod handler_0x58bba_adapter;
#[path = "handler_0x58BCB.rs"]
mod handler_0x58bcb;
#[path = "handler_0x58BCB_adapter.rs"]
mod handler_0x58bcb_adapter;
#[path = "handler_0x58C41.rs"]
mod handler_0x58c41;
#[path = "handler_0x58C41_adapter.rs"]
mod handler_0x58c41_adapter;
#[path = "handler_0x58CE0.rs"]
mod handler_0x58ce0;
#[path = "handler_0x58CE0_adapter.rs"]
mod handler_0x58ce0_adapter;
#[path = "handler_0x58D34.rs"]
mod handler_0x58d34;
#[path = "handler_0x58D34_adapter.rs"]
mod handler_0x58d34_adapter;
#[path = "handler_0x58C52.rs"]
mod handler_0x58c52;
#[path = "handler_0x58C52_adapter.rs"]
mod handler_0x58c52_adapter;
#[path = "handler_0x58E7E.rs"]
mod handler_0x58e7e;
#[path = "handler_0x58E7E_adapter.rs"]
mod handler_0x58e7e_adapter;
#[path = "handler_0x591A3.rs"]
mod handler_0x591a3;
#[path = "handler_0x591A3_adapter.rs"]
mod handler_0x591a3_adapter;
#[path = "handler_0x59270.rs"]
mod handler_0x59270;
#[path = "handler_0x59270_adapter.rs"]
mod handler_0x59270_adapter;
#[path = "handler_0x595FE.rs"]
mod handler_0x595fe;
#[path = "handler_0x595FE_adapter.rs"]
mod handler_0x595fe_adapter;
#[path = "handler_0x596D9.rs"]
mod handler_0x596d9;
#[path = "handler_0x596D9_adapter.rs"]
mod handler_0x596d9_adapter;
#[path = "handler_0x59763.rs"]
mod handler_0x59763;
#[path = "handler_0x59763_adapter.rs"]
mod handler_0x59763_adapter;
#[path = "handler_0x59774.rs"]
mod handler_0x59774;
#[path = "handler_0x59774_adapter.rs"]
mod handler_0x59774_adapter;
#[path = "handler_0x597BD.rs"]
mod handler_0x597bd;
#[path = "handler_0x597BD_adapter.rs"]
mod handler_0x597bd_adapter;
#[path = "handler_0x599A6.rs"]
mod handler_0x599a6;
#[path = "handler_0x599A6_adapter.rs"]
mod handler_0x599a6_adapter;
#[path = "handler_0x59AAD.rs"]
mod handler_0x59aad;
#[path = "handler_0x59AAD_adapter.rs"]
mod handler_0x59aad_adapter;
#[path = "handler_0x59B0E.rs"]
mod handler_0x59b0e;
#[path = "handler_0x59B0E_adapter.rs"]
mod handler_0x59b0e_adapter;
#[path = "handler_0x59B1F.rs"]
mod handler_0x59b1f;
#[path = "handler_0x59B1F_adapter.rs"]
mod handler_0x59b1f_adapter;
#[path = "handler_0x59E5B.rs"]
mod handler_0x59e5b;
#[path = "handler_0x59E5B_adapter.rs"]
mod handler_0x59e5b_adapter;
#[path = "handler_0x59EFF.rs"]
mod handler_0x59eff;
#[path = "handler_0x59EFF_adapter.rs"]
mod handler_0x59eff_adapter;
#[path = "handler_0x59FB9.rs"]
mod handler_0x59fb9;
#[path = "handler_0x59FB9_adapter.rs"]
mod handler_0x59fb9_adapter;
#[path = "handler_0x59FCE.rs"]
mod handler_0x59fce;
#[path = "handler_0x59FCE_adapter.rs"]
mod handler_0x59fce_adapter;
#[path = "handler_0x59FDF.rs"]
mod handler_0x59fdf;
#[path = "handler_0x59FDF_adapter.rs"]
mod handler_0x59fdf_adapter;
#[path = "handler_0x5A0AA.rs"]
mod handler_0x5a0aa;
#[path = "handler_0x5A0AA_adapter.rs"]
mod handler_0x5a0aa_adapter;
#[path = "handler_0x5A179.rs"]
mod handler_0x5a179;
#[path = "handler_0x5A179_adapter.rs"]
mod handler_0x5a179_adapter;
#[path = "handler_0x5A236.rs"]
mod handler_0x5a236;
#[path = "handler_0x5A236_adapter.rs"]
mod handler_0x5a236_adapter;
#[path = "handler_0x5A258.rs"]
mod handler_0x5a258;
#[path = "handler_0x5A258_adapter.rs"]
mod handler_0x5a258_adapter;
#[path = "handler_0x5A2D0.rs"]
mod handler_0x5a2d0;
#[path = "handler_0x5A2D0_adapter.rs"]
mod handler_0x5a2d0_adapter;
#[path = "handler_0x5A30E.rs"]
mod handler_0x5a30e;
#[path = "handler_0x5A30E_adapter.rs"]
mod handler_0x5a30e_adapter;
#[path = "handler_0x5A355.rs"]
mod handler_0x5a355;
#[path = "handler_0x5A355_adapter.rs"]
mod handler_0x5a355_adapter;
#[path = "handler_0x5A545.rs"]
mod handler_0x5a545;
#[path = "handler_0x5A545_adapter.rs"]
mod handler_0x5a545_adapter;
#[path = "handler_0x5A5EC.rs"]
mod handler_0x5a5ec;
#[path = "handler_0x5A5EC_adapter.rs"]
mod handler_0x5a5ec_adapter;
#[path = "handler_0x5A667.rs"]
mod handler_0x5a667;
#[path = "handler_0x5A667_adapter.rs"]
mod handler_0x5a667_adapter;
#[path = "handler_0x5A683.rs"]
mod handler_0x5a683;
#[path = "handler_0x5A683_adapter.rs"]
mod handler_0x5a683_adapter;
#[path = "handler_0x586E6.rs"]
mod handler_0x586e6;
#[path = "handler_0x586E6_adapter.rs"]
mod handler_0x586e6_adapter;
#[path = "handler_0x58804.rs"]
mod handler_0x58804;
#[path = "handler_0x58804_adapter.rs"]
mod handler_0x58804_adapter;
mod handler_0x586eb;
mod handler_0x591f9;
mod vertical_bobber;
mod handler_0x5b0d5;
mod handler_0x5b36d;
#[path = "handler_0x5A8D2.rs"]
mod handler_0x5a8d2;
#[path = "handler_0x5A8D2_adapter.rs"]
mod handler_0x5a8d2_adapter;
mod helpers;
mod interaction;
mod items;
mod scratch;
mod shell;
mod stream;
mod stream_cursor;
#[cfg(test)]
mod tests;
mod mario_zone;
mod boo;
mod carrier;
mod cannon_ant;
mod charging_enemy;
mod dashing_walker;
mod edge_walker;
mod falling_dropper;
mod fallwalker;
mod fire_piranha;
mod groundwalker;
mod horizontal_platform;
mod left_marcher;
mod paced_walker;
mod perched_flyer;
mod pipe_bobber;
mod piranha_fireball;
mod rising_platform;
mod rising_platform_adapter;
mod squashed;
mod vertical_pacer;
mod walker;
mod walkers;
mod wario;
mod wave_flyer;
mod winged_walker;

#[derive(Debug, Clone)]
pub struct ObjectSim {
    pub slots: [ObjSlot; SLOT_COUNT],
    pub records: Vec<SpawnRecord>,
    initial_records: Vec<SpawnRecord>,
    pub stream_index: usize,
    stream_cursor_cleared: bool,
    pub spawned: Vec<bool>,
    pub prev_cam_x: u16,
    pub scan_dir: u8,
    pub spawn_x: u16,
    pub phase: bool,
    pub odd: bool,
    pub fff0: u8,
    pub update_phase: u8,
    pub standby: u8,
    pub active: u8,
    pub easy_mode: bool,
    pub sprite_map_index: u8,
    pub level_id: u8,
    pub done: u8,
    pub clear_stage_flags: bool,
    pub pending_item: u8,
    pub remove_mode: u8,
    pub move_flags: u8,
    pub facing: u8,
    pub mario_ref_x: u16,
    pub mario_ref_y: u16,
    pub mario_dead: u8,
    pub bounce: bool,
    pub stomp_jump: bool,
    pub stomp_bounce: bool,
    pub on_object: bool,
    pub on_ground_cleared: bool,
    pub mario_x_adjust: Option<u16>,
    pub mario_y_adjust: Option<u16>,
    pub stomp_bounce_timer: u8,
    pub stomp_chain: u8,
    pub money_bag: u8,
    pub window_hit: bool,
    pub sprite_flip_flags: u8,
    pub goal_bell: u8,
    pub secret_route: bool,
    pub spawn_anim: u8,
    pub spawn_sprite: u8,
    pub spawn_attr: u8,
    pub mario: MarioContact,
    /// Current player screen X, projected before boss targeting.
    pub player_screen_x: u8,
    pub slot_done: [bool; SLOT_COUNT],
    pub action_latch: u8,
    pub object_hit_flag: u8,
    pub pending_kick: bool,
    pub sfx0: u8,
    pub sfx1: u8,
    pub sfx2: u8,
    pub music_request: Option<u8>,
    pub boss_clear_flag: u8,
    pub boss_clear_handoff: bool,
    pub fade_timer_set: Option<u16>,
    /// Object-produced camera writes, consumed after the object pass.
    pub camera_pan_set: Option<u8>,
    /// Wario exit clears scene state and selects this boss phase.
    pub scene_phase_set: Option<u8>,
    pub slot_ptr_hi: u8,
    pub boss_slot_ptr_lo: u8,
    pub fade_timer: u16,
    /// Entry-frame `$A254` input from the level countdown owner.
    pub timer_low: u8,
    /// Sampled hardware `$FF04`, supplied by the runtime clock.
    pub div: u8,
    pub saved_frame_state: u8,
    pub saved_type: u8,
    pub boss_def_index: u8,
    pub boss_def_count: u8,
    pub boss_hit_latch: u8,
    pub boss_5290: u8,
    pub boss_hits_remaining: u8,
    pub boss_sprite: u8,
    pub boss_timer_reload: u8,
    /// `$AFD0`: Wario's falling-child completion latch.
    pub boss_delay: u8,
    /// `$AFD1`: player power minus two, saved when final Wario begins a jump.
    pub saved_power_state: u8,
    /// `$AFCE`: Wario's transition latch, consumed by anim21/24.
    pub boss_transition: u8,
    /// `$AFCF`: Wario intro door-animation latch.
    pub ending_door_active: u8,
    pub boss_defeated: bool,
    pub boss_slots_raw: [u8; 512],
    pub boss_oam: Vec<BossOamCell>,
    pub unported: Vec<u32>,
}

impl ObjectSim {
    pub fn new(records: Vec<SpawnRecord>, cam_x: u16) -> Self {
        let n = records.len();
        Self {
            slots: Default::default(),
            initial_records: records.clone(),
            records,
            stream_index: 0,
            stream_cursor_cleared: false,
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
            mario_x_adjust: None,
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
            player_screen_x: 64,
            slot_done: [false; SLOT_COUNT],
            action_latch: 0,
            object_hit_flag: 0,
            pending_kick: false,
            sfx0: 0,
            sfx1: 0,
            sfx2: 0,
            music_request: None,
            boss_clear_flag: 0,
            boss_clear_handoff: false,
            fade_timer_set: None,
            camera_pan_set: None,
            scene_phase_set: None,
            slot_ptr_hi: 0xAD,
            boss_slot_ptr_lo: 0,
            fade_timer: 0,
            timer_low: 0,
            div: 0,
            saved_frame_state: 0,
            saved_type: 0,
            boss_def_index: 0,
            boss_def_count: 0,
            boss_hit_latch: 0,
            boss_5290: 0,
            boss_hits_remaining: 0,
            boss_sprite: 0,
            boss_timer_reload: 0,
            boss_delay: 0,
            saved_power_state: 0,
            boss_transition: 0,
            ending_door_active: 0,
            boss_defeated: false,
            boss_slots_raw: [0; 512],
            boss_oam: Vec::new(),
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
    /// 0. Runs `$8D9C`'s first call, `$EC40` (`clear_object_scratch`), then the
    ///    `$EC31` slot clear — in the ROM's order, before any staging.
    /// 1. Clears both slot pages ($AD00-$AEFF, 512 bytes) and resets slot array.
    /// 2. Sets `boss_hits_remaining = hits`, `boss_sprite = sprite`, `boss_timer_reload = 80`.
    /// 3. Copies 18-byte `obj0` to `$AD05` (offset 5 in slot 0).
    /// 4. If `count > 0`, loops over sub-objects from destination `$AD25` (slot 1)
    ///    at the `$8E6C` step — `((e + 16) & $F0) + 5` evaluated on the cursor
    ///    *after* its 18-byte copy, i.e. +$20 per sub-object, so a count-2 def
    ///    fills slots 1 and 2.
    /// 5. Reflects raw staged bytes into `slots[0..4]` for slot-level consumers.
    pub fn spawn_boss(&mut self, def_index: u8) {
        if def_index > 8 {
            // `$904E` would read past the 9-entry pointer tables. Only
            // reachable with a non-zero `sBossFightPhase` (whose scene-script
            // writer is unported), so report it instead of silently serving
            // def 8's data.
            self.note_unported(0x8DFD);
        }
        let (count, hits, sprite, obj0, sub_objs) = crate::scene::lookup_boss_spawn_def(def_index);
        // `$8D9C`: `$3A28` ($EC40 scratch clear) then `$3A00` ($EC31 slot
        // clear) run before `$8DFD`/`$8E6C` stage anything.
        self.clear_object_scratch();
        self.boss_slots_raw = [0; 512];
        for slot in &mut self.slots {
            slot.b = [0; 32];
        }
        // The per-slot draw latch is derived from what the cleared page held.
        self.slot_done = [false; SLOT_COUNT];
        self.boss_def_index = def_index;
        self.boss_def_count = count;
        self.boss_hits_remaining = hits;
        self.boss_sprite = sprite;
        self.boss_timer_reload = 80;

        // obj0 at $AD05 (offset 5 in slot 0)
        self.boss_slots_raw[5..5 + 18].copy_from_slice(&obj0);

        // Sub-objects start at $AD25 (sObject1Def); see the stride note below.
        if count > 0 {
            // `$8E6C`: the first sub-image lands at `sObject1Def` (`$AD25`), and
            // then the destination advances by `e = ((e + 16) & $F0) + 5` —
            // where `ld a, e` reads E **after** `$3CAA` already walked it the 18
            // bytes it copied. So the real step is
            // `(18 + 16)` folded into the low byte: `$25 -> $45 -> $65`, i.e.
            // +$20 per sub-image, filling slot 1 then slot 2 of the object page.
            // The earlier `+$10` reading (which skipped the post-copy E) left a
            // count-2 def's second part inert — the console stages it at `$AD45`
            // and the arena is one house short without it.
            let mut offset = 0x25usize;
            for sub in sub_objs.iter().take(count as usize) {
                if offset + 18 <= self.boss_slots_raw.len() {
                    self.boss_slots_raw[offset..offset + 18].copy_from_slice(sub);
                }
                offset = ((offset + 18 + 16) & 0xF0) + 5;
            }
        }

        // Copy raw staged slot pages into slots 0..4 (32 bytes each)
        for i in 0..4 {
            self.slots[i]
                .b
                .copy_from_slice(&self.boss_slots_raw[i * 32..(i + 1) * 32]);
        }
    }

    /// Helper for testing interaction dispatch directly.
    pub fn post_touch(
        &mut self,
        st: &mut Staging,
        _layout: &[u8; 48],
        tables: &ObjectTables,
        contact: &MarioContact,
    ) {
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
        boss_mode: bool,
    ) {
        self.mario = *mario;
        if !mario.travelling {
            self.collide(cam_x, cam_y, layout);
        }
        // `$0652`: while `sSceneState & $F0 != 0` the frame's object pass is
        // `$3C9F` -> bank 2 `$8DA9` (the boss driver) *instead of* `$8000`.
        // Both must switch together: arming `$F0` without this driver parks the
        // port with nothing to advance the scene.
        if boss_mode {
            self.boss_engine(cam_x, cam_y, layout, tables);
        } else {
            self.engine(cam_x, cam_y, layout, tables);
        }
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
        self.slot_done = [false; SLOT_COUNT];
        self.clear_object_scratch();
        self.records.clone_from(&self.initial_records);
        self.spawned = vec![false; self.records.len()];
        self.seed_stream_cursor();
        self.pending_item = 0;
        self.mario_dead = 0;
        self.bounce = false;
        self.stomp_jump = false;
        self.stomp_bounce = false;
        self.on_object = false;
        self.on_ground_cleared = false;
        self.mario_x_adjust = None;
        self.mario_y_adjust = None;
        self.stomp_bounce_timer = 0;
        self.stomp_chain = 0;
        self.money_bag = 0;
        self.goal_bell = 0;
        self.mario = MarioContact::default();
        self.camera_pan_set = None;
        self.scene_phase_set = None;
        // $AF06: $EC40 clears it; $EC4B then sets 1 for levels >= $0A.
        self.sprite_map_index = if self.level_id >= 0x0A { 1 } else { 0 };
        self.odd = self.fff0 & 1 == 1; // fff0 kept ($FFF0 is HRAM)
    }

    // ---------------------------------------------------------------------
    // $8000 — windows, slot loop, spawn scan
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // The bank-2 boss helper library ($9285, $B14E-$B411)
    //
    // A *parallel* set to the bank-22 `$5B4xx` helpers the level objects use:
    // byte-compare the two regions and 2020 of 2048 bytes differ, so the same
    // local address is not the same routine. These move the 8-bit *screen*
    // bytes (`B_SCRY`/`B_SCRX`), never the world X/Y that `mv_x_add`/`mv_y_add`
    // touch.
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // The def-2 (Mario Zone) fight: anims 10/11/12
    //
    // anim 12 is the *controller*: no HP at all (its hit body never touches
    // `$FFD8`), it waits for the parts to clear and then spawns one child.
    // anims 10 and 11 are the two *parts*, and `B_STATE` (`$FFD8`, slot byte 8
    // — normally the level engine's slot state) is this fight's per-part HP,
    // seeded straight from the def image: def 2's two sub-objects carry 6, its
    // controller 0. `$9C72` is the only writer of that byte in the whole of
    // bank 2.
    // ---------------------------------------------------------------------

    /// `$9C3F`: the hit body anims 10 and 11 share. A stomp takes 2 HP, a
    /// projectile 1, and 0 *or a borrow* sends the part to `$5435` — the
    /// defeat fall. The controller never runs this.

    /// `$9CAC`: the shared hit-stun (state 1). Flash, count `B_T28` down, then
    /// either the defeat fall (remove-mode set) or restore the banked
    /// sprite/state/type.

    /// `$9C8B`: the intro's end — arm the fight (type `$45`, state 3, sprite
    /// `$20`, a `$2F`-style sfx).

    /// `$9CDF`: the bob body the walk states run each frame. `B_T24` picks the
    /// ascent profile (cap 15) or the descent one (cap 21) — the descent also
    /// ends the flight at `B_SCRY = $90`.

    /// `$9D3D`: a walk hit the end of the arena — flip the state bit and the
    /// object, and re-announce the anim id.

    /// `$9C36` (boss anim 10) and `$9D4F` (anims 11): the two parts of the
    /// Mario Zone boss. Same hit body, same shared states; the walk states are
    /// mirrored and anim 11's intro waits for the other part to clear.

    /// `$9E07` (boss anim 12): the controller. It has no HP — its hit body banks
    /// the slot and parks it without touching `$FFD8` — waits for the parts to
    /// clear, then spawns one child (anim `$0D`, sprite `$0E`) and starts the
    /// fade-done walk, whose speed comes from `sBossHits` (`4 - hits/2`).

    /// `$9323` (boss anim 3): a **defeated** part's way out — the body the
    /// `$9443` morph hands every finishing slot to (`B_DEF = 3`), parts and the
    /// controller alike.
    ///
    /// ```text
    /// $9323  entry: B_SCxy... `$B270` b=32 — the screen Y goes *up* — plus the
    ///        Y flip (`$FFDD ^= $40`), type 0, `$FFE0 = 0`, state 1, and sfx 13
    ///        (23 when this is not the primary slot)
    /// $5354  state 1: the cap-15 picker (`$B1EB`, the very table the bob uses)
    ///        keeps `$B270`-ing it up until `$FFE0` reaches 15 -> state 2
    /// $5366  state 2: the cap-21 picker (`$B18F`) `$B276`-s it *down* until
    ///        `SCRY >= $D0`, then splits at `$9399`:
    ///          non-primary -> `$B157` (drop `sObjectDefCount`) + done + remove 2
    ///          primary     -> state 3, a 208-frame timer, `sMusicRequest = 12`
    /// $5385  state 3: the timer runs out -> `$B15B` (blank only) + done + remove 2
    /// ```
    ///
    /// This is the piece that was missing: the morph left the slot at anim 3,
    /// dispatch reported `$9323`, and a defeated house stopped dead wherever the
    /// morph had frozen it — with `ObjectSim::boss_finish_blank`/`_counted` never
    /// reached, and the controller therefore waiting for a live count that never
    /// dropped.

    /// `$9606` (boss anim 2): the def-2 spawner. While its remove-mode is 2 it
    /// spawns one child (anim 4 / sprite 9) and parks itself at sprite 7; every
    /// later call returns at the state test, because its state bodies belong to
    /// the def-4 family's table, not to its own.

    /// `$0EB0`'s scene branch could not be taken: its counterpart (the bank-15
    /// `$3C000` handlers that clear `sSceneState`) is unported, and taking it
    /// would park the camera and swap this pass for the boss driver. Reported
    /// through the same `unported` list the handlers use, so it is visible in
    /// play mode and cannot silently regress.
    pub fn note_blocked_scene_load(&mut self) {
        self.note_unported(0x0EB0);
    }

    /// `$8DA9` (bank 2) — the alternate per-frame object pass that replaces the
    /// `$8000` engine during a boss fight (`$0652` selects it while
    /// `sSceneState & $F0 != 0`; `$0EB0` -> `$3C94` -> `$8D9C` filled the def
    /// page it walks). It walks `sObjectDefCount + 1` slots from `$AD00` with a
    /// **$20 stride** — note this is *not* the `+$10` the def images use, which
    /// is why a multi-part def's later images straddle slot pages and the
    /// slots they leak into come up with a zero anim byte and are skipped — and
    /// for each: clears `sObjectDone`/`sBossHitLatch`/`$5290`, stages the slot
    /// into `$FFD0` (`$81A7`, low byte masked to `$F0`), and only when the
    /// staged anim byte (`$FFD5`) is non-zero dispatches the anim (`$92DF`,
    /// bank 2's own 32-entry table), clears `$FFE7` (= `B_FLAGS`, the
    /// interaction-flag clear the level path does in `$818D`), fetches the
    /// `$505B` extent record (`$9048`), and copies the slot back (`$81BD`).
    ///
    /// Note the boss slots' *screen* bytes are NOT derived from world coords:
    /// in this path `B_SCRY`/`B_SCRX` are the object's own OAM position — the
    /// def image carries it and the handlers' movers change it — and
    /// [`Self::boss_render`] consumes it as-is.

    /// `$3A14` -> bank 25 `$66000` — the boss path's render pass.
    ///
    /// A *different* draw from the level's `$C000`: it reads the staged slot's
    /// sprite byte (`$FFDB` = [`B_SPRITE`]) as an index into the 118-entry `dw`
    /// table at bank-25 `$6092`, and the slot's `$FFD9`/`$FFDA` ([`B_SCRY`] /
    /// [`B_SCRX`]) as the base *OAM* position — no world-coordinate math, which
    /// is why `$8DA9` never stages the `$C090` pair.
    ///
    /// Each record cell is four bytes `[dy, dx, tile, attr]`, emitted as one
    /// OAM entry: `y = dy + base`, `x = dx + base`, `tile`, `attr ^ flags`. The
    /// `$6087` flags byte is `$FFDC ^ $FFDD ^ $FFDE` (slot bytes 12/13/14) and
    /// does double duty — bit 5 mirrors the X offset (`~dx - 7`) and bit 6 the
    /// Y offset (`~dy - 7`), and the whole byte is XORed into every emitted
    /// attr. The bounds are the console's own OAM ones: a Y past `$A0` drops
    /// the cell (four record bytes consumed, nothing written), an X past `$A8`
    /// writes a `0` over the Y byte the cell had already stored — the port
    /// emits that entry with `y = 0`, which draws nothing (OAM Y 0 is 16 px
    /// above the screen) and keeps the slot count/order identical.
    ///
    /// `$88 <adj>` is the big-blob origin marker: one operand byte, and while
    /// flip bit 6 is set it shifts the base Y for every cell after it (the
    /// 32x32 blobs' offsets are computed from the flipped origin).

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
        // $818D: set slot pointer registers $AF28/$AF29
        let (hi, lo) = handler_0x5b36d::slot_address(i);
        self.slot_ptr_hi = hi;
        self.boss_slot_ptr_lo = lo;
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
        self.refresh_hitbox(&mut st, tables, false);
        // $81BD: staging → slot.
        self.slots[i].b = st.b;
    }

    /// `$81CF` (level tables `$4737`/`$4935`) or `$9048` (the scene-state
    /// table `$505B`): copy the 4-byte extent record for the staged sprite id
    /// into `$FFE3-$FFE6`.
    fn refresh_hitbox(&self, st: &mut Staging, tables: &ObjectTables, boss: bool) {
        let rec = if boss {
            tables.draw_extent_for(st.b[B_SPRITE])
        } else {
            tables.hitbox_for(self.level_id, st.b[B_SPRITE])
        };
        if let Some(hb) = rec {
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
        self.dispatch_handler(handler, st, layout, tables);
    }

    // ---------------------------------------------------------------------
    // Shared bank-16 helpers
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // Anim 9 — the walker ($58201)
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // Anim 88 — the ground walker ($59BC5)
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // Anim 46 — the carrying platform ($58DD1)
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // Anim 12 — the bobber ($584E0)
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
    // Anim 8 — the hover crawler ($5862E)
    // ---------------------------------------------------------------------

    // ---------------------------------------------------------------------
}
