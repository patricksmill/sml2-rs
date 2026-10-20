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
//! palettes and raster state), then `$3C38C`, which is the cursor:
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
//! ## Presentation state
//!
//! Zone sprite motion, route growth, IRQ tile copies, palette flashes and
//! raster scroll bands are native modules under `map/`. They update once per
//! map tick; the renderer consumes their staged sprites and persistent tile
//! writes without advancing them. DIV is an explicit clock input. Random
//! rejection uses successive DIV samples rather than reproducing CPU timing.
//! Blocking screen transitions remain represented by their native flow effects.

use crate::map::{map_enterable, progress_offset, MapNodeRecords, MapNodes, MapWalk, WalkDir};

/// One class of the `$3C38C` node chain — the synthetic key the map presses when
/// the cursor sits on a node of that class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainClass {
    /// `$3C29A` — press Right.
    Right,
    /// `$3C2A0` — press Left.
    Left,
    /// `$3C2AC` — press Up.
    Up,
    /// `$3C2A6` — press Down.
    Down,
    /// `$3C2B2` — press Start, which `$3C5F4` turns into the node's action.
    Start,
    /// `$3C2B8` — A retargets to node `$36`, else B retargets to node `$37`.
    MacroA,
    /// `$3C2E7` — Select retargets to node `$39`.
    MacroSelect,
    /// `$3C2FE` — A retargets to node `$6C`.
    MacroGate,
}

/// `(node, class)` in the ROM's own comparison order (main.asm:18756-18877). The
/// chain re-reads `sMapNode` at every step, so the order is load-bearing: the
/// three retargeting classes sit early, and a retarget changes what the rest of
/// the chain matches against.
/// `$3C364`: `sLevelProgress` index the arrival walk tests, indexed by `$650E` slot.
const ARRIVAL_PROGRESS_INDEX: [u8; 40] = [
    0x00, 0x02, 0x03, 0x05, 0x05, 0x05, 0x07, 0x08, 0x09, 0x09, 0x0B, 0x0C, 0x0D, 0x0E, 0x0E, 0x10,
    0x11, 0x11, 0x13, 0x10, 0x15, 0x16, 0x17, 0x17, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1D, 0x1F, 0x20,
    0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
];

const NODE_CHAIN: &[(u8, ChainClass)] = &[
    (0x14, ChainClass::Up),
    (0x35, ChainClass::MacroA),
    (0x32, ChainClass::MacroSelect),
    (0x6A, ChainClass::MacroGate),
    (0x17, ChainClass::Left),
    (0x4B, ChainClass::Down),
    (0x2C, ChainClass::Down),
    (0x4A, ChainClass::Up),
    (0x2F, ChainClass::Up),
    (0x31, ChainClass::Down),
    (0x38, ChainClass::Down),
    (0x2E, ChainClass::Up),
    (0x6B, ChainClass::Up),
    (0x47, ChainClass::Up),
    (0x65, ChainClass::Left),
    (0x45, ChainClass::Left),
    (0x27, ChainClass::Left),
    (0x67, ChainClass::Start),
    (0x5B, ChainClass::Start),
    (0x5A, ChainClass::Up),
    (0x68, ChainClass::Right),
    (0x25, ChainClass::Right),
    (0x6C, ChainClass::Right),
    (0x43, ChainClass::Right),
    (0x79, ChainClass::Right),
    (0x69, ChainClass::Down),
    (0x73, ChainClass::Left),
    (0x72, ChainClass::Right),
    (0x3F, ChainClass::Right),
    (0x74, ChainClass::Left),
    (0x75, ChainClass::Right),
    (0x11, ChainClass::Start),
    (0x02, ChainClass::Start),
    (0x0C, ChainClass::Start),
    (0x0B, ChainClass::Start),
    (0x09, ChainClass::Start),
    (0x06, ChainClass::Start),
    (0x64, ChainClass::Right),
];

/// One of `$3C60B`'s A/START node actions: what pressing A or START on a hub node
/// actually does.
///
/// All nine zone doors make the same five mechanical writes — the cursor, the
/// node, `sMarioDead = $FF`, music `$FF` and the blocking walk-out
/// `$3E3BA(dir)` — and then load a screen (its gfx and script: art). The four
/// coin-operated doors are the only ones that can *fail*, and they fail into
/// `$3C7D9`'s idle path.
struct NodeDoor {
    /// `sMapCursorX` after the set-piece.
    cursor_x: u8,
    cursor_y: u8,
    /// `sMapNode` after it.
    node: u8,
    /// The screen the handler loads.
    screen_id: u8,
    /// `$3E3BA`'s walk-out direction; the coin doors do not run one.
    dir: Option<u8>,
    /// `sMapCursorMove`, when the handler sets one ($06/$1B press a fresh move).
    move_: Option<u8>,
    /// The handler masks the move byte's low nibble off first ($3D5D0/$3D848 keep
    /// the bump bit).
    mask_move_nibble: bool,
    /// `sMusicRequest`.
    music: u8,
    /// The coin doors' BCD price — the only doors that can be refused.
    price: Option<u16>,
}

/// `(node, door)`, with the shared destinations spelled out per node: `$23`/`$28`
/// share their targets with nothing, but `$02`/`$19` and `$09`/`$1C` (and the
/// `$67`/`$0D` pair) run the same handler.
const NODE_DOORS: &[(u8, NodeDoor)] = &[
    // $3D0D7: Tree Zone screen A. Cursor 48/158, node $23, screen 2.
    (
        0x02,
        NodeDoor {
            cursor_x: 48,
            cursor_y: 158,
            node: 0x23,
            screen_id: 2,
            dir: Some(4),
            move_: None,
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    (
        0x19,
        NodeDoor {
            cursor_x: 48,
            cursor_y: 158,
            node: 0x23,
            screen_id: 2,
            dir: Some(4),
            move_: None,
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    // $3D1E0: Pumpkin Zone screen A. Cursor 83/137, node $28, screen 1.
    (
        0x09,
        NodeDoor {
            cursor_x: 83,
            cursor_y: 137,
            node: 0x28,
            screen_id: 1,
            dir: Some(4),
            move_: None,
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    (
        0x1C,
        NodeDoor {
            cursor_x: 83,
            cursor_y: 137,
            node: 0x28,
            screen_id: 1,
            dir: Some(4),
            move_: None,
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    // $3D33A: Macro Zone. Cursor 226/118, move $21, node $32, screen 3.
    (
        0x06,
        NodeDoor {
            cursor_x: 226,
            cursor_y: 118,
            node: 0x32,
            screen_id: 3,
            dir: Some(1),
            move_: Some(0x21),
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    (
        0x1B,
        NodeDoor {
            cursor_x: 226,
            cursor_y: 118,
            node: 0x32,
            screen_id: 3,
            dir: Some(1),
            move_: Some(0x21),
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    // $3D477: Space Zone. Cursor 16/100, node $3C, screen 4.
    (
        0x12,
        NodeDoor {
            cursor_x: 16,
            cursor_y: 100,
            node: 0x3C,
            screen_id: 4,
            dir: Some(1),
            move_: None,
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    // $3D5D0/$3D5EC share the $3D606 tail: move nibble masked, screen 5, music 18.
    (
        0x67,
        NodeDoor {
            cursor_x: 146,
            cursor_y: 234,
            node: 0x4B,
            screen_id: 5,
            dir: Some(8),
            move_: None,
            mask_move_nibble: true,
            music: 18,
            price: None,
        },
    ),
    (
        0x0D,
        NodeDoor {
            cursor_x: 75,
            cursor_y: 242,
            node: 0x46,
            screen_id: 5,
            dir: Some(8),
            move_: None,
            mask_move_nibble: true,
            music: 18,
            price: None,
        },
    ),
    // $3D6C3: Wario's Castle — hub screen 0 only, which the caller already checked.
    (
        0x11,
        NodeDoor {
            cursor_x: 129,
            cursor_y: 137,
            node: 0x5A,
            screen_id: 7,
            dir: Some(2),
            move_: None,
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    // $3D760: Mario Zone. Cursor 12/112, node $50, screen 6.
    (
        0x0B,
        NodeDoor {
            cursor_x: 12,
            cursor_y: 112,
            node: 0x50,
            screen_id: 6,
            dir: Some(1),
            move_: None,
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    (
        0x1D,
        NodeDoor {
            cursor_x: 12,
            cursor_y: 112,
            node: 0x50,
            screen_id: 6,
            dir: Some(1),
            move_: None,
            mask_move_nibble: false,
            music: 255,
            price: None,
        },
    ),
    // $3D848: screen 8. Cursor 212/131, node $72, move nibble masked.
    (
        0x15,
        NodeDoor {
            cursor_x: 212,
            cursor_y: 131,
            node: 0x72,
            screen_id: 8,
            dir: Some(1),
            move_: None,
            mask_move_nibble: true,
            music: 255,
            price: None,
        },
    ),
    // The four coin-operated doors ($3F3B9/$3F3F0/$3F427/$3F45E). They do not move
    // the cursor or the node — $3F307/$3F35A only clear the scroll, the walk state
    // and the key byte, load the bonus screen and set the window registers — so the
    // cursor fields here are the loader's own no-ops.
    (
        0x71,
        NodeDoor {
            cursor_x: 0,
            cursor_y: 0,
            node: 0,
            screen_id: 9,
            dir: None,
            move_: None,
            mask_move_nibble: false,
            music: 21,
            price: Some(0x0999),
        },
    ),
    (
        0x70,
        NodeDoor {
            cursor_x: 0,
            cursor_y: 0,
            node: 0,
            screen_id: 10,
            dir: None,
            move_: None,
            mask_move_nibble: false,
            music: 21,
            price: Some(0x0200),
        },
    ),
    (
        0x6F,
        NodeDoor {
            cursor_x: 0,
            cursor_y: 0,
            node: 0,
            screen_id: 11,
            dir: None,
            move_: None,
            mask_move_nibble: false,
            music: 21,
            price: Some(0x0050),
        },
    ),
    (
        0x6E,
        NodeDoor {
            cursor_x: 0,
            cursor_y: 0,
            node: 0,
            screen_id: 12,
            dir: None,
            move_: None,
            mask_move_nibble: false,
            music: 21,
            price: Some(0x0030),
        },
    ),
];

/// `$3F38F`: the coin-door price check — a 16-bit **BCD** subtraction of the price
/// from `sCoinLow`/`sCoinHigh` (`sub l / daa` then `sbc h / daa`).
///
/// `None` means the ROM's `jr c` fired: the player cannot pay, the coins are left
/// alone and the door answers with sfx 19 and the idle path. The port subtracts
/// digit by digit with a borrow rather than emulating `daa`'s flags, which is the
/// same arithmetic the ROM's correction produces.
fn bcd_sub_u16(lo: u8, hi: u8, price: u16) -> Option<(u8, u8)> {
    let (pl, ph) = (price as u8, (price >> 8) as u8);
    let mut borrow = 0i16;
    let mut digits = [0i16; 4];
    let minuend = [
        (lo & 0x0F) as i16,
        ((lo >> 4) & 0x0F) as i16,
        (hi & 0x0F) as i16,
        ((hi >> 4) & 0x0F) as i16,
    ];
    let subtrahend = [
        (pl & 0x0F) as i16,
        ((pl >> 4) & 0x0F) as i16,
        (ph & 0x0F) as i16,
        ((ph >> 4) & 0x0F) as i16,
    ];
    let mut out_borrow = false;
    for i in 0..4 {
        let mut d = minuend[i] - subtrahend[i] - borrow;
        borrow = 0;
        if d < 0 {
            d += 10;
            borrow = 1;
        }
        if i == 3 && borrow == 1 {
            out_borrow = true;
        }
        digits[i] = d;
    }
    if out_borrow {
        return None;
    }
    let low = ((digits[1] as u8) << 4) | digits[0] as u8;
    let high = ((digits[3] as u8) << 4) | digits[2] as u8;
    Some((low, high))
}

/// `$3C60B`'s dispatch, in the ROM's own order of comparisons.
fn node_door(node: u8) -> Option<&'static NodeDoor> {
    NODE_DOORS.iter().find(|(n, _)| *n == node).map(|(_, d)| d)
}

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

impl MapInput {
    pub const fn new(keys_held: u8, keys_pressed: u8, div: u8) -> Self {
        Self {
            keys_held,
            keys_pressed,
            div,
        }
    }

    /// Construct map input using a sampled hardware divider register ($FF04).
    pub fn with_divider(
        keys_held: u8,
        keys_pressed: u8,
        divider: &crate::divider::Divider,
    ) -> Self {
        Self {
            keys_held,
            keys_pressed,
            div: divider.sample(),
        }
    }
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
    /// `$3C518` called the hub loader after releasing the B camera pan.
    pub hub_reload: bool,
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

/// Song each overworld screen's loader requests, indexed by `sMapScreenId`.
///
/// The zone loaders all end with `ld a,<id> / ld [sMusicRequest],a / call
/// $2AAA`: `$3D0FF`->`$10` (16, screen 2), `$3D208`->`$1E` (30, screen 1),
/// `$3D371`->`$05` (screen 3), `$3D49F`->`$1C` (28, screen 4), `$3D61F`->`$12`
/// (18, screen 5), `$3D79A`->`$1B` (27, screen 6), `$3D6EB`->`$0E` (14, screen
/// 7), `$3D880`->`$0D` (13, screen 8). The hub loader `$3F156` stops music at
/// `$3F160` and ends with `ld a,6 / ld [sMusicRequest],a` at `$3F1FE` (`$3EA9C`
/// requests the same 6); the castle screens (`$3F495`/`$3F513`/`$3F5A4`) request 21.
pub const SCREEN_MUSIC: [u8; 13] = [6, 30, 16, 5, 28, 18, 27, 14, 13, 21, 21, 21, 21];

/// `sMusicRequest` value a screen's loader writes; 0 for a screen with no
/// modelled loader.
pub fn screen_music(screen_id: u8) -> u8 {
    SCREEN_MUSIC.get(screen_id as usize).copied().unwrap_or(0)
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
    /// `sMapRockAnimTimer` (`$A789`): the turtle's swim counter, set to 1 by the
    /// `$3C4E7` secret-route arm and stepped every hub frame by `$3CF16`.
    pub rock_anim_timer: u8,
    /// `sMapSpecialNodeLatch` (`$A7B8`) — written by `$3C201`, read by nothing.
    pub special_node_latch: u8,
    /// `sMapKeysXor` (`$A6B2`) — a census-only byte with no reader in the ROM.
    pub keys_xor: u8,
    /// A zone screen's sixteen decor slots (`$A6B5`, 3 bytes each: `x`, `y`, `id`),
    /// which `$3E277` expands into OAM from entry 4.
    pub zone_slots: [(u8, u8, u8); 16],
    pub zone_animation: ZoneAnimation,
    /// `sMapAnimPhase` (`$A699`).
    pub anim_phase: u8,
    /// `sMapAnimIndexA` (`$A6A2`): 0..$4F walk of the `$6227` step table.
    pub anim_index: u8,
    /// `sMapAnimValueA` (`$A6A5`): the last step value read.
    pub anim_value: u8,
    /// `sMapSprite*Page/X/Y/Id` (`$A6B5`, 18 x 4): the decor slot list `$3CFC8`
    /// expands into OAM from index 13.
    pub decor_slots: [(u8, u8, u8, u8); 18],
    /// `sMapCoinIconPhase` (`$A780`): 0-2, cycled every 8 frames.
    pub coin_icon_phase: u8,
    /// `sMapCloudDriftCounter` (`$A781`): bit 6 is the drift direction.
    pub cloud_drift: u8,
    /// `sMapSparkleTimer` (`$A7A9`): the two random sparkle slots' countdown.
    pub sparkle_timer: u8,
    /// `sMapFrameCounter` (`$A69A`).
    pub frame_counter: u8,
    /// `sMapTickTimer` (`$A697`) — 40-frame countdown.
    pub tick_timer: u8,
    /// `sMapRandLatch` (`$A696`).
    pub rand_latch: u8,
    /// The map camera *staging* (`$FFBA:B9` X, `$FFB8:B7` Y). `$3EEA4` seeds it
    /// from the node's record, the pan helpers move it, and `$3ED4C` latches it
    /// into `sMapCamX/Y` — which the port does not model, because the port's map
    /// has nothing to draw. It is still mechanical: the pan helpers gate on it.
    pub cam_x: u16,
    pub cam_y: u16,
    /// `sMarioDead` (`$A224`): 0 means "just arrived on this screen", and `$3C32D`
    /// uses that to fire each zone node's auto-walk **once** — it sets the byte and
    /// then blocks itself. The level-entry handlers set it too.
    pub mario_dead: u8,
    /// `sCoinLow`/`sCoinHigh` (`$A262`/`$A263`, BCD) — the coin purse the
    /// coin-operated bonus doors subtract their price from.
    pub coins_low: u8,
    pub coins_high: u8,
    /// `$2934` was called: the caller must persist the save image and clear it.
    pub pending_save: bool,
    /// The active bank-24 room script (screens 9-14), started by the loader.
    pub room: Option<crate::castle_room::CastleRoom>,
    /// `sLives` (BCD) as the room scripts read and write it.
    pub lives: u8,
    /// `sCurPowerup`.
    pub powerup: u8,
    /// The extracted bank-24 room blobs; a coin door builds its room from them.
    pub castle_scripts: Option<crate::castle::CastleScripts>,
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
            zone_slots: [(0, 0, 0xFF); 16],
            zone_animation: ZoneAnimation::default(),
            anim_phase: 0,
            anim_index: 0,
            anim_value: 0,
            decor_slots: crate::map::HUB_DECOR_SLOTS,
            coin_icon_phase: 0,
            cloud_drift: 0,
            sparkle_timer: 0,
            frame_counter: 0,
            tick_timer: 40,
            rand_latch: 0,
            cam_x: 0,
            cam_y: 0,
            mario_dead: 0,
            coins_low: 0,
            coins_high: 0,
            pending_save: false,
            room: None,
            lives: 0,
            powerup: 0,
            castle_scripts: None,
        }
    }

    /// Attach the extracted room blobs (`extracted/map/castle_scripts.*`), so a
    /// coin door can stage its room.
    pub fn set_castle_scripts(&mut self, scripts: crate::castle::CastleScripts) {
        self.castle_scripts = Some(scripts);
    }

    /// Seed the map's persistent block from the player's save image.
    ///
    /// `PlayerSim` holds `sMapNode` and `sLevelProgress` because the `$14`
    /// hand-off and the save record need them; the map is where they *live*. Both
    /// structs carry a copy and these two functions are the bridge between them.
    pub fn from_player(sim: &crate::player_sim::PlayerSim) -> Self {
        let mut me = Self::new();
        me.node = sim.map_node;
        me.coins_low = sim.coins_low;
        me.coins_high = sim.coins_high;
        me.lives = sim.lives;
        me.powerup = sim.powerup;
        me.mario_dead = sim.dead;
        let n = me.level_progress.len().min(sim.level_progress.len());
        me.level_progress[..n].copy_from_slice(&sim.level_progress[..n]);
        me
    }

    /// Write the map's node, progress bytes and coin purse back onto the player's
    /// save image.
    pub fn to_player(&self, sim: &mut crate::player_sim::PlayerSim) {
        sim.map_node = self.node;
        sim.coins_low = self.coins_low;
        sim.coins_high = self.coins_high;
        sim.lives = self.lives;
        sim.powerup = self.powerup;
        let n = self.level_progress.len().min(sim.level_progress.len());
        sim.level_progress[..n].copy_from_slice(&self.level_progress[..n]);
    }

    /// Restore map state from a validated 0x50-byte save record.
    pub fn apply_save_record(&mut self, rec: &[u8], records: &MapNodeRecords) -> bool {
        if !crate::map::save_record_valid(rec) {
            return false;
        }
        self.node = rec[0];
        let n = self.level_progress.len().min(0x40 - 0x08);
        self.level_progress[..n].copy_from_slice(&rec[0x08..0x08 + n]);
        self.coins_low = rec[0x40];
        self.coins_high = rec[0x41];
        self.load_node(self.node, records);
        true
    }

    /// Build a 0x50-byte save record from current map progress and given player stats.
    pub fn save_record(&self, lives: u8, level_tally: u8, money_bag: u8) -> [u8; 0x50] {
        let mut rec = [0u8; 0x50];
        rec[0] = self.node;
        let n = self.level_progress.len().min(0x40 - 0x08);
        rec[0x08..0x08 + n].copy_from_slice(&self.level_progress[..n]);
        rec[0x40] = self.coins_low;
        rec[0x41] = self.coins_high;
        rec[0x42] = level_tally;
        rec[0x43] = lives;
        rec[0x44] = money_bag;
        rec[0x45] = rec[..0x45].iter().fold(0u8, |c, b| c.wrapping_add(*b));
        rec[0x46..0x4A].copy_from_slice(&crate::map::SAVE_SIGNATURE);
        rec
    }

    /// `$3EEA4`: adopt a node's record — the map camera it centres on and where the
    /// cursor sits there.
    ///
    /// The ROM calls this whenever a *screen* changes node (`$3C7CA` on a zone
    /// door, `$3C518` when B is released), not on a walk arrival: a walk moves the
    /// cursor pixel by pixel and leaves the camera alone.
    pub fn load_node(&mut self, node: u8, records: &MapNodeRecords) {
        let Some(r) = records.record(node) else {
            // No records loaded: leave the camera and cursor where they are rather
            // than seeding them from a table that is not there.
            return;
        };
        self.cam_x = r.cam_x;
        self.cam_y = r.cam_y;
        self.cursor_x = r.cursor_x;
        self.cursor_y = r.cursor_y;
    }

    /// Load a zone screen ($3D0FF/$3D208/$3D371/$3D49F/$3D61F/$3D79A/$3D6EB/$3D880)
    /// or the hub ($3F156), as selected by the level->map hand-off ($3F04D/$3F07E)
    /// or a map door.
    ///
    /// Clears scroll and walking latches, resets camera to screen origin (0, 0)
    /// for zone screens or loads staged camera for the hub, applies screen-specific
    /// node retargets, path unlocks and gate latches, loads camera/cursor records
    /// when the node changes, sets the move-frame anim gate ($92), and returns
    /// the requested music track ID.
    pub fn load_screen(
        &mut self,
        screen_id: u8,
        exit_tile_flag: u8,
        records: &MapNodeRecords,
    ) -> u8 {
        use crate::map::progress_offset as po;

        let old_progress = self.level_progress;
        let fresh_exit = exit_tile_flag != 0 && self.mario_dead == 0;

        self.screen_id = screen_id;
        self.cursor_walking = 0;
        self.scroll_only_latch = 0;
        self.frame_counter = 0;
        // `$3CBE7` clears the drift counter and `$CBE7`'s tail reseeds the list.
        self.decor_slots = crate::map::HUB_DECOR_SLOTS;
        if self.screen_id == 0 {
            self.seed_hub_coin_icons();
        }
        self.seed_zone_sprites(screen_id, exit_tile_flag);
        self.anim_phase = 0;
        self.anim_index = 0;
        self.anim_value = 0;
        self.coin_icon_phase = 0;
        self.cloud_drift = 0;
        self.sparkle_timer = 0;

        let music = match screen_id {
            0 => {
                // $3F156: Overworld hub loader
                self.load_node(self.node, records);
                self.rock_anim_timer = 0;
                self.cursor_anim = 0;
                6 // $3EA9A music 6
            }
            1 => {
                // $3D208: Pumpkin Zone loader
                self.cam_x = 0;
                self.cam_y = 0;
                self.cursor_anim = 0x92;
                if exit_tile_flag != 0 && self.mario_dead == 0 {
                    if self.node == 0x2C {
                        self.node = 44; // 0x2C
                        self.load_node(44, records);
                        self.level_progress[po::PUMPKIN_PATH_1] |= 1;
                    } else if self.node == 0x2F {
                        self.node = 47; // 0x2F
                        self.load_node(47, records);
                        self.level_progress[po::PUMPKIN_PATH_2] |= 1;
                    }
                }
                30 // Music request 30
            }
            2 => {
                // $3D0FF: Tree Zone screen A loader
                self.cam_x = 0;
                self.cam_y = 0;
                self.cursor_anim = 0x92;
                if exit_tile_flag != 0 && self.mario_dead == 0 {
                    self.node = 37; // 0x25
                    self.load_node(37, records);
                    self.level_progress[po::TREE_PATH_1] |= 1;
                }
                if self.node == 0x26 {
                    self.node = 39; // 0x27
                    self.load_node(39, records);
                    self.mario_dead = 0xFF;
                }
                16 // Music request 16
            }
            3 => {
                // $3D371: Macro Zone loader
                self.cam_x = 0;
                self.cam_y = 0;
                self.cursor_move = 0x21; // $3D38D: ld a, 33; $3D402: or $20
                self.cursor_anim = 0x92;
                if self.node == 0x32 && exit_tile_flag != 0 && self.mario_dead == 0 {
                    self.node = 107; // 0x6B
                    self.load_node(107, records);
                    self.level_progress[po::MACRO_PATH_0] |= 1;
                }
                if self.node == 0x6A && self.mario_dead == 0 {
                    self.node = 108; // 0x6C
                    self.load_node(108, records);
                    self.level_progress[po::GATE_MACRO] |= 0x80;
                    self.mario_dead = 0xFF;
                }
                5 // Music request 5
            }
            4 => {
                // $3D49F: Space Zone loader
                self.cam_x = 0;
                self.cam_y = 0;
                self.cursor_anim = 0x92;
                if exit_tile_flag != 0 && self.mario_dead == 0 {
                    self.node = 67; // 0x43
                    self.load_node(67, records);
                    self.level_progress[po::SPACE_PATH_0] |= 1;
                }
                if self.node == 0x44 {
                    self.node = 69; // 0x45
                    self.load_node(69, records);
                    self.mario_dead = 0xFF;
                }
                28 // Music request 28
            }
            5 => {
                // $3D61F: Turtle Zone loader
                self.cam_x = 0;
                self.cam_y = 0;
                self.cursor_anim = 0x92;
                if exit_tile_flag != 0 && self.mario_dead == 0 {
                    self.level_progress[po::TURTLE_PATH_1] |= 1;
                }
                18 // Music request 18
            }
            6 => {
                // $3D79A: Mario Zone loader
                self.cam_x = 0;
                self.cam_y = 0;
                self.cursor_anim = 0x92;
                27 // Music request 27
            }
            7 => {
                // $3D6EB: Wario's Castle loader
                self.cam_x = 0;
                self.cam_y = 0;
                self.cursor_anim = 0x92;
                14 // Music request 14
            }
            8 => {
                // $3D880: Castle loader
                self.cam_x = 0;
                self.cam_y = 0;
                self.cursor_anim = 0x92;
                self.load_node(self.node, records);
                13 // Music request 13
            }
            _ => 6,
        };
        self.seed_zone_path_markers(screen_id);
        self.reset_zone_animation(screen_id, fresh_exit, old_progress);
        music
    }

    /// `$3F156`: load overworld hub (screen 0).
    pub fn load_screen_hub(&mut self, records: &MapNodeRecords) -> u8 {
        self.load_screen(0, 0, records)
    }

    /// `$3D208`: load Pumpkin Zone (screen 1).
    pub fn load_screen_pumpkin(&mut self, exit_tile_flag: u8, records: &MapNodeRecords) -> u8 {
        self.load_screen(1, exit_tile_flag, records)
    }

    /// `$3D0FF`: load Tree Zone screen A (screen 2).
    pub fn load_screen_tree(&mut self, exit_tile_flag: u8, records: &MapNodeRecords) -> u8 {
        self.load_screen(2, exit_tile_flag, records)
    }

    /// `$3D371`: load Macro Zone (screen 3).
    pub fn load_screen_macro(&mut self, exit_tile_flag: u8, records: &MapNodeRecords) -> u8 {
        self.load_screen(3, exit_tile_flag, records)
    }

    /// `$3D49F`: load Space Zone (screen 4).
    pub fn load_screen_space(&mut self, exit_tile_flag: u8, records: &MapNodeRecords) -> u8 {
        self.load_screen(4, exit_tile_flag, records)
    }

    /// `$3D61F`: load Turtle Zone (screen 5). It copies the Tree Zone tile
    /// bank, which is why the decomp's names call this screen "Tree B".
    pub fn load_screen_turtle(&mut self, exit_tile_flag: u8, records: &MapNodeRecords) -> u8 {
        self.load_screen(5, exit_tile_flag, records)
    }

    /// `$3D79A`: load Mario Zone (screen 6).
    pub fn load_screen_mario(&mut self, records: &MapNodeRecords) -> u8 {
        self.load_screen(6, 0, records)
    }

    /// `$3D6EB`: load Wario's Castle (screen 7), the `$3BD80` tilemap.
    pub fn load_screen_wario_castle(&mut self, records: &MapNodeRecords) -> u8 {
        self.load_screen(7, 0, records)
    }

    /// `$3D880`: load Castle (screen 8).
    pub fn load_screen_castle(&mut self, records: &MapNodeRecords) -> u8 {
        self.load_screen(8, 0, records)
    }

    /// `$3C38C`: the map presses its own keys.
    ///
    /// Forty `cp $XX / call z` comparisons run before the frame's cursor logic, and
    /// each one **overwrites `hKeysPressed`** with a single direction bit
    /// (`$3C29A` Right, `$3C2A0` Left, `$3C2A6` Down, `$3C2AC` Up) or with Start
    /// (`$3C2B2`). The same frame's `$3C52B` then turns that synthetic press into an
    /// auto-walk along the node's walk-graph entry — which is how the map tours the
    /// cursor between screens — and a synthetic Start makes `$3C5F4` dispatch the
    /// node's own action.
    ///
    /// Returns the pressed byte for the rest of the frame.
    ///
    /// (`$3C316` jumps straight to the gate `$3C4D3` rather than through `$3C4CC`,
    /// but it only does so with `sMapCursorWalking` clear, and `$3C4CC`'s own test is
    /// the same one — so the two paths are the same code here.)
    fn synthetic_press(&mut self, pressed: u8, nodes: &MapNodes) -> u8 {
        let mut p = pressed;
        // The chain is sequential and re-reads `sMapNode` at every comparison, so a
        // helper that *retargets* the node (`$3C2B8`/`$3C2E7`/`$3C2FE`) changes what
        // the later comparisons see. Mirrored by re-reading `self.node` in the loop.
        for &(node, class) in NODE_CHAIN {
            if self.node != node {
                continue;
            }
            match class {
                ChainClass::Right => p = 0x10,
                ChainClass::Left => p = 0x20,
                ChainClass::Up => p = 0x40,
                ChainClass::Down => p = 0x80,
                ChainClass::Start => p = 0x08,
                // `$3C2B8`: A retargets to node $36 if Macro path 2 bit 7 is set;
                // otherwise B retargets to node $37 if the Macro gate bit 7 is set.
                ChainClass::MacroA => {
                    if p & 0x10 != 0 {
                        if self.block(progress_offset::MACRO_PATH_2) & 0x80 != 0 {
                            self.node = 0x36;
                            p = 0x10;
                        }
                    } else if p & 0x20 != 0 && self.block(progress_offset::GATE_MACRO) & 0x80 != 0 {
                        self.node = 0x37;
                        p = 0x20;
                    }
                }
                // `$3C2E7`: Select (bit 6) retargets to node $39 when Macro path 0's
                // bit 0 is set.
                ChainClass::MacroSelect => {
                    if p & 0x40 != 0 && self.block(progress_offset::MACRO_PATH_0) & 0x01 != 0 {
                        self.node = 0x39;
                        p = 0x40;
                    }
                }
                // `$3C2FE`: A retargets to node $6C when the Macro gate bit 7 is set.
                ChainClass::MacroGate => {
                    if p & 0x10 != 0 && self.block(progress_offset::GATE_MACRO) & 0x80 != 0 {
                        self.node = 0x6C;
                        p = 0x10;
                    }
                }
            }
        }
        // `$3C32D` for every node from $1E up: a zone node's arrival auto-walk,
        // fired once per screen (the `sMarioDead` byte is its latch) and only while
        // the stage is not already cleared.
        if self.node >= 0x1E {
            p = self.zone_arrival_walk(p, nodes);
        }
        // `$3C316`: node 0 on the hub. Once level 1 is cleared, the story entry is
        // done with and the cursor is moved to node $14 — where the chain's Up class
        // auto-walks it next frame. The node change happens *after* the chain, so
        // `$14`'s own class does not fire this frame; the ROM jumps straight to the
        // gate (`$3C4D3`), skipping the walking check.
        if self.node == 0 && self.cursor_walking == 0 && self.level_progress[0] & 0x80 != 0 {
            self.node = 0x14;
        }
        p
    }

    /// A byte of the map's own block, by `sLevelProgress` offset.
    fn block(&self, off: usize) -> u8 {
        self.level_progress.get(off).copied().unwrap_or(0)
    }

    /// `$3C32D`: a zone node's arrival auto-walk.
    ///
    /// Once per screen — the walk has stopped, `sMarioDead` is 0 (the level-entry
    /// handlers set it to `$FF`) — the map *presses the node's own direction mask*
    /// (`$6591[slot]`) so the cursor walks off the node by itself. The latch is
    /// spent before the clear test, so a skipped walk still does not fire again.
    ///
    /// The clear test does not read the slot `$3EEF0` just wrote. It reads
    /// `sLevelProgress[$3C364[slot]]`. That table runs one ahead of the slot
    /// (`$3C364[$1A] = $1B`), so finishing node `$50` still walks Up toward `$51`
    /// while `$1B` is clear. The direction press stays `$6591[slot]`.
    fn zone_arrival_walk(&mut self, pressed: u8, nodes: &MapNodes) -> u8 {
        if self.cursor_walking != 0 || self.mario_dead != 0 {
            return pressed;
        }
        self.mario_dead = 0xFF;
        let Some(slot) = nodes.progress_slot(self.node) else {
            return pressed;
        };
        // `$3C364` is 40 bytes. Slots `$28` and `$29` (nodes `$04` and `$44`)
        // read the two opcodes that follow it, `ei` / `ld a, [sMapNode]`.
        let index = match slot {
            0..=39 => ARRIVAL_PROGRESS_INDEX[slot as usize],
            0x28 => 0xFB,
            0x29 => 0xFA,
            _ => slot,
        };
        if self.block(index as usize) & 0x80 != 0 {
            return pressed;
        }
        // `$6591[slot]`: the mask is a direction byte, so this is the press the node
        // would accept from the player.
        nodes.key_mask(slot).unwrap_or(pressed)
    }

    /// `$3CA31`/`$3CA5D`/`$3CA84`/`$3CAAB` — the position gates.
    ///
    /// Called from the walk's own x/y update while the cursor is on the hub: when it
    /// reaches `$4C` (walking Right), `$4A` (Left), `$4B` (Up) or `$4D` (Down) —
    /// around the middle of the 160-px screen — the camera takes over.
    fn pan_gate(&mut self, dir: u8) {
        let ok = match dir {
            0x01 => self.cursor_x == 0x4C,
            0x02 => self.cursor_x == 0x4A,
            0x04 => self.cursor_y == 0x4B,
            0x08 => self.cursor_y == 0x4D,
            _ => false,
        };
        if ok {
            self.pan_body(dir);
        }
    }

    /// The four pan bodies (`$3CA37`/`$3CA63`/`$3CA8A`/`$3CAB1`): move the camera
    /// one pixel, and — unless the player is holding B, which is what
    /// `sMapScrollOnlyLatch` records — push the cursor back the other way, holding it
    /// near the centre while the walk continues. The hub's B-pan calls these
    /// directly, with no position gate.
    fn pan_body(&mut self, dir: u8) {
        match dir {
            0x01 => {
                if self.cam_x >> 8 != 0 && (self.cam_x & 0xFF) >= 0x60 {
                    return;
                }
                self.cam_x = self.cam_x.wrapping_add(1);
                if self.scroll_only_latch == 0 {
                    self.cursor_x = self.cursor_x.wrapping_sub(1);
                }
            }
            0x02 => {
                if self.cam_x == 0 {
                    return;
                }
                self.cam_x = self.cam_x.wrapping_sub(1);
                if self.scroll_only_latch == 0 {
                    self.cursor_x = self.cursor_x.wrapping_add(1);
                }
            }
            0x04 => {
                if self.cam_y == 0 {
                    return;
                }
                self.cam_y = self.cam_y.wrapping_sub(1);
                if self.scroll_only_latch == 0 {
                    self.cursor_y = self.cursor_y.wrapping_add(1);
                }
            }
            0x08 => {
                if self.cam_y >> 8 != 0 && (self.cam_y & 0xFF) >= 0x70 {
                    return;
                }
                self.cam_y = self.cam_y.wrapping_add(1);
                if self.scroll_only_latch == 0 {
                    self.cursor_y = self.cursor_y.wrapping_sub(1);
                }
            }
            _ => {}
        }
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
    /// `$3CE43`: the six coin icons are live only where the matching
    /// `sGoldenCoinN` byte (`$4E37`'s pointer table) has bit 7 set.
    pub fn seed_hub_coin_icons(&mut self) {
        for (i, index) in crate::map::HUB_COIN_FLAGS.iter().enumerate() {
            let flag = self.level_progress.get(*index).copied().unwrap_or(0);
            self.decor_slots[4 + i].3 = if flag & 0x80 != 0 { 2 } else { 0xFF };
        }
    }

    /// `$3D8E6`: the Pumpkin screen's decor handler, run every frame by `$3C063`
    /// before `$3C38C`. Slot 0's id comes from `sMapAnimPhase` via `$5A21`, slot
    /// 1's id from `$5A29` and its y drifts one pixel every 8 frames, and slot 2
    /// walks `$6227` one step on alternate frames, moving its x by `(v >> 4) & 7`
    /// (sign bit 7) and its y by `v & 7` (sign bit 3); slot 2's id is `22 + $5A16[...]` plus 3 unless the
    /// step has bit 7 set, and the whole sprite is dropped once the second golden
    /// coin is held.
    fn pumpkin_decor_tick(&mut self) {
        self.anim_phase = self.anim_phase.wrapping_add(1);
        let e = ((self.anim_phase & 0x38) >> 3) as usize;
        self.zone_slots[0].2 = crate::map::PUMPKIN_ID_Y[e];
        let e = ((self.anim_phase & 0x0C) >> 2) as usize;
        self.zone_slots[1].2 = crate::map::PUMPKIN_ID_X[e];
        if self.frame_counter & 0x07 == 0 {
            let step = if self.frame_counter & 0x20 != 0 {
                0xFFu8
            } else {
                1
            };
            self.zone_slots[1].1 = self.zone_slots[1].1.wrapping_add(step);
        }
        if self.tick_timer & 0x01 != 0 {
            self.anim_index = (self.anim_index + 1) % 0x50;
            let v = crate::map::PUMPKIN_STEPS[self.anim_index as usize];
            self.anim_value = v;
            // `sMapSprite1Y` is slot 2's x: `(v >> 4) & 7`, signed by bit 7.
            let a = (v >> 4) & 0x07;
            self.zone_slots[2].0 = if v & 0x80 != 0 {
                self.zone_slots[2].0.wrapping_sub(a)
            } else {
                self.zone_slots[2].0.wrapping_add(a)
            };
            // `sMapSprite1Id` is slot 2's y: `v & 7`, signed by bit 3.
            let a = v & 0x07;
            self.zone_slots[2].1 = if v & 0x08 != 0 {
                self.zone_slots[2].1.wrapping_sub(a)
            } else {
                self.zone_slots[2].1.wrapping_add(a)
            };
        }
        let v = self.anim_value;
        let e = ((self.anim_index & 0x78) >> 3) as usize;
        let mut id = 22u8.wrapping_add(crate::map::PUMPKIN_PAGE[e]);
        if v & 0x80 == 0 {
            id = id.wrapping_add(3);
        }
        if self.level_progress.get(9).copied().unwrap_or(0) & 0x80 != 0 {
            id = 0xFF;
        }
        self.zone_slots[2].2 = id;
    }

    /// `$3CE6B`: the hub's decor updater, run every frame by `$3CFC8` just before
    /// it expands the 18 slots. Coins (slots 4-9) cycle their ids 2/3/4 every 8
    /// frames, the three drifting sprites (slots 1-3) move one pixel every 4
    /// frames with the direction on the counter's bit 6, and the two sparkle
    /// slots (16/17) re-arm from `rDIV` on the same 4-frame beat.
    fn hub_decor_tick(&mut self, div: u8) {
        if self.screen_id != 0 {
            return;
        }
        let fc = self.frame_counter;
        if fc & 0x07 == 0 {
            self.coin_icon_phase = (self.coin_icon_phase + 1) % 3;
            for slot in 4..10 {
                if self.decor_slots[slot].3 != 0xFF {
                    self.decor_slots[slot].3 = self.coin_icon_phase + 2;
                }
            }
        }
        if fc & 0x03 == 0 {
            self.cloud_drift = self.cloud_drift.wrapping_add(1);
            let step = if self.cloud_drift & 0x40 != 0 {
                1u8
            } else {
                0xFF
            };
            for slot in 1..4 {
                self.decor_slots[slot].1 = self.decor_slots[slot].1.wrapping_add(step);
            }
            if self.sparkle_timer != 0 {
                self.sparkle_timer = self.sparkle_timer.wrapping_sub(1);
                let visible = self.sparkle_timer & 1 != 0;
                self.set_sparkles(div, visible);
            } else if div & 0x72 == 0 {
                // `$3CF09` arms four and falls into `$3CEC4`, which decrements.
                self.sparkle_timer = 4u8.wrapping_sub(1);
                let visible = self.sparkle_timer & 1 != 0;
                self.set_sparkles(div, visible);
            }
        }
        // `$3CEB7` falls into `$3CF16` on every frame, not only the fourth.
        self.turtle_swim_tick();
    }

    /// `$3CF16`: the turtle by nodes `$0C`/`$1E` (hub slot 14) once `$3C4E7`
    /// arms `sMapRockAnimTimer`. The counter steps every frame: up to `$2B` the
    /// turtle swims left one pixel a frame, opening and closing its mouth
    /// (ids 5-8 from bits 3-4 of `counter - 2`) once the counter reaches `$10`;
    /// from `$2C` it swims back right with its mouth shut; from `$58` it holds
    /// id 5. Slots 10-13 and 15 are its wake (id 11), one more bubble for each
    /// 8 px it has swum left of `$88`.
    ///
    /// The counter is never cleared here: it runs on and wraps through `$FF`
    /// to 0, which takes one more left step before the routine idles.
    fn turtle_swim_tick(&mut self) {
        const TURTLE: usize = 14;
        const WAKE: [usize; 5] = [10, 11, 12, 13, 15];
        if self.rock_anim_timer == 0 {
            return;
        }
        self.rock_anim_timer = self.rock_anim_timer.wrapping_add(1);
        let counter = self.rock_anim_timer;
        let mut mouth = if counter >= 0x10 {
            (counter.wrapping_sub(2) & 0x18) >> 3
        } else {
            0
        };
        if counter >= 0x58 {
            self.decor_slots[TURTLE].3 = 5;
            return;
        }
        let x = &mut self.decor_slots[TURTLE].1;
        if counter < 0x2C {
            *x = x.wrapping_sub(1);
        } else {
            *x = x.wrapping_add(1);
            mouth = 0;
        }
        let swum = (x.wrapping_sub(88) >> 3) & 7;
        self.decor_slots[TURTLE].3 = mouth + 5;
        // `$3CF5B`..`$3CFA7`: wake slot k is shown while `swum < 6 - k`.
        for (k, slot) in WAKE.into_iter().enumerate() {
            self.decor_slots[slot].3 = if (swum as usize) + k < 6 { 11 } else { 0xFF };
        }
    }

    /// `$3CEC4`: hide both sparkle slots, then show them at positions out of
    /// `$4FA8`/`$4FB8` when the countdown is odd.
    fn set_sparkles(&mut self, div: u8, visible: bool) {
        let id = if visible { 0x0Au8 } else { 0xFF };
        self.decor_slots[16].3 = id;
        self.decor_slots[17].3 = id;
        if visible {
            let i = (div & 0x0E) as usize;
            self.decor_slots[16].1 = crate::map::SPARKLE_SLOT_16[i];
            self.decor_slots[16].2 = crate::map::SPARKLE_SLOT_16[i + 1];
            let j = ((div & 0xE0) >> 4) as usize;
            self.decor_slots[17].1 = crate::map::SPARKLE_SLOT_17[j];
            self.decor_slots[17].2 = crate::map::SPARKLE_SLOT_17[j + 1];
        }
    }

    pub fn bookkeeping(&mut self, div: u8) {
        self.keys_xor ^= div;
        self.frame_counter = self.frame_counter.wrapping_add(1);
        self.hub_decor_tick(div);
        self.tick_timer = self.tick_timer.wrapping_sub(1);
        if self.tick_timer == 0 {
            self.tick_timer = 40;
            self.rand_latch ^= div;
        }
        if self.screen_id == 1 {
            self.pumpkin_decor_tick();
        }
        self.zone_animation_tick(div);
    }

    /// One map frame: `$3C000` for the frame's screen, then `$3C169`.
    ///
    /// `records` seeds the camera and the cursor whenever the frame moves the cursor
    /// to a different *screen* (`$3C7CA` on a zone door, `$3C518` when B is
    /// released); a plain walk arrival does not re-seed anything.
    pub fn tick(
        &mut self,
        input: MapInput,
        nodes: &MapNodes,
        walk: &MapWalk,
        records: &MapNodeRecords,
    ) -> MapEntry {
        let mut out = MapEntry::default();
        // `$3C38C` runs *before* the cursor logic and may overwrite the pressed byte
        // with a synthetic key, so the whole frame sees the map's own press.
        // Bonus screens $3C0C3-$3C10E run only their room scripts.
        let pressed = if self.screen_id < 9 {
            self.synthetic_press(input.keys_pressed, nodes)
        } else {
            input.keys_pressed
        };
        let input = MapInput {
            keys_pressed: pressed,
            ..input
        };
        let consumed = if self.screen_id == 0 {
            let c = self.cursor_chain(input, nodes, walk, records, &mut out);
            self.bookkeeping(input.div);
            c
        } else if (9..=14).contains(&self.screen_id) {
            // `$3C0C3`..`$3C10E`: the bonus screens run the bank-24 room
            // script instead of the cursor chain.
            self.bookkeeping(input.div);
            self.room_tick(input, &mut out);
            if self.screen_id <= 12 && self.room.as_ref().is_some_and(|r| r.state == 3) {
                // `$3F4A0`: state 3 calls `$3D880`, the castle screen loader.
                self.room = None;
                out.music = Some(self.load_screen(8, 0, records));
            }
            false
        } else {
            self.bookkeeping(input.div);
            // Bookkeeping includes the zone routine before the cursor chain.
            self.cursor_chain(input, nodes, walk, records, &mut out)
        };
        // The zone loaders clear `hKeysPressed` (`xor a / ldh [$FF81]`), so a door that
        // opens cannot also enter a level on the same frame. The zone→hub doors do
        // *not* clear it — which is why pressing A on the untested door nodes can open
        // a door and reach a level in one frame.
        let entry_input = if consumed {
            MapInput {
                keys_pressed: 0,
                ..input
            }
        } else {
            input
        };
        // `$3C0C3`..`$3C10E` return without `$3C169`: a bonus screen's A press
        // never re-enters a level.
        if self.screen_id < 9 {
            self.enter_check(entry_input, nodes, &mut out);
        }
        if self.zone_animation.sfx0 != 0 && out.sfx.is_none() {
            out.sfx = Some(self.zone_animation.sfx0);
        }
        out
    }

    /// One frame of the bank-24 room script (`$3C0C3`..`$3C10E`).
    fn room_tick(&mut self, input: MapInput, out: &mut MapEntry) {
        let mut rewards = crate::castle_room::RoomRewards {
            powerup: self.powerup,
            lives: self.lives,
            coins_low: self.coins_low,
            coins_high: self.coins_high,
            save: false,
        };
        let room_input = crate::castle_room::RoomInput {
            keys_pressed: input.keys_pressed,
            keys_held: input.keys_held,
            frame: self.frame_counter,
            node: self.node,
            div: input.div,
        };
        let Some(room) = self.room.as_mut() else {
            return;
        };
        room.tick(room_input, &mut rewards);
        // `$3EAFF` runs `$3EBD5` on screens 9-12 at the frame sync.
        if self.screen_id <= 12 {
            room.frame_sync();
        }
        if room.music != 0 {
            out.music = Some(room.music);
            room.music = 0;
        }
        if room.sfx[0] != 0 {
            out.sfx = Some(room.sfx[0]);
            room.sfx[0] = 0;
        }
        self.powerup = rewards.powerup;
        self.lives = rewards.lives;
        self.coins_low = rewards.coins_low;
        self.coins_high = rewards.coins_high;
        if rewards.save {
            self.pending_save = true;
            out.saved = true;
        }
    }

    /// `$3C38C`'s tail: the cursor, `$3C4CC` onward. Returns whether the frame's
    /// press was *consumed* — a node action that cleared `hKeysPressed`, which stops
    /// the same frame from also entering a level.
    fn cursor_chain(
        &mut self,
        input: MapInput,
        nodes: &MapNodes,
        walk: &MapWalk,
        records: &MapNodeRecords,
        out: &mut MapEntry,
    ) -> bool {
        if self.cursor_walking != 0 {
            self.walk_step(walk);
            return false;
        }
        // `$3C4D3`: 0 from the gate means the node's own action runs.
        if self.enterable(nodes, input.keys_held) {
            return self.node_action(input, records, out);
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
            return false;
        }
        if self.screen_id == 0 {
            // `$3C5B2`: B (bit 1 of the held byte) turns the hub into a camera pan —
            // the cursor stays put and each held direction bit pans the camera.
            if input.keys_held & 0x02 != 0 {
                self.scroll_only_latch = 0xFF;
                for (bit, dir) in [(0x10u8, 0x01u8), (0x20, 0x02), (0x40, 0x04), (0x80, 0x08)] {
                    if input.keys_held & bit != 0 {
                        self.pan_body(dir);
                    }
                }
                return false;
            }
            // `$3C518`: the frame after B is released, re-centre the screen on the
            // cursor's node. $3EEA4 seeds the camera and cursor from the node's
            // record, and $3EA9A clears the anim byte and asks for music 6.
            if self.scroll_only_latch != 0 {
                out.music = Some(self.load_screen(0, 0, records));
                out.hub_reload = true;
                return true;
            }
        }
        self.scroll_only_latch = 0;
        let pressed = input.keys_pressed & 0xF0;
        if pressed == 0 {
            return self.node_action(input, records, out);
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
            Some(ptr) if ptr != MapWalk::NO_PATH => {
                self.start_walk(ptr, walk);
                false
            }
            _ => self.node_action(input, records, out),
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
            if self.screen_id == 0 {
                self.pan_gate(0x04);
            }
        } else if m & 0x08 != 0 {
            self.cursor_pose = ((self.walk_frame & 0x0C) >> 2) + 12;
            if self.cursor_y < 0x8A {
                self.cursor_y += 1;
            }
            if self.screen_id == 0 {
                self.pan_gate(0x08);
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
            if self.screen_id == 0 {
                self.pan_gate(0x01);
            }
        } else if m & 0x02 != 0 {
            if self.walk_frame & 0x0C == 0x0C {
                self.walk_frame = 0;
            }
            self.cursor_pose = (self.walk_frame & 0x0C) >> 2;
            if self.cursor_x >= 0x08 {
                self.cursor_x -= 1;
            }
            if self.screen_id == 0 {
                self.pan_gate(0x02);
            }
        }
    }

    /// `$3C5DA` — the node's own action, reached when the cursor is not walking.
    /// Returns whether it cleared the frame's press (only `$3C60B`'s loaders do).
    ///
    /// `$3C5F4` needs A or START *pressed* to run the node's entry action, and
    /// only on the hub or screen 8; everything else falls to `$3C66C`. On the hub
    /// that is the idle path (`$3C7D9`); on a zone screen it is the door cluster
    /// (`$3C6B0`-`$3C7A3`), which is what turns a zone screen back into the hub.
    fn node_action(
        &mut self,
        input: MapInput,
        records: &MapNodeRecords,
        out: &mut MapEntry,
    ) -> bool {
        // `$3C5DA`: nodes $15 and $0D *force* a Start press on the map's own behalf
        // — the synthetic `$3C2B2` — and the gate below then sees it like any other
        // press. Node $0D only does that while the rock animation is not running, in
        // which case it goes straight to `$3C60B` with no gate at all.
        let mut pressed = input.keys_pressed;
        if self.node == 0x15 {
            pressed = 0x08;
        } else if self.node == 0x0D {
            pressed = 0x08;
            if self.rock_anim_timer != 0 {
                return self.dispatch_door(pressed, records, out);
            }
        }
        // `$3C5F4`: A or START, on the hub or screen 8, with a door to open.
        if pressed & 0x09 != 0
            && (self.screen_id == 0 || self.screen_id == 8)
            && node_door(self.node).is_some()
        {
            return self.dispatch_door(pressed, records, out);
        }
        // `$3C66C`: on the hub that is the idle path; on a zone screen it is the door
        // cluster, and everything else falls through to the idle path too.
        if self.screen_id != 0 && self.door(pressed, records, out) {
            // The zone→hub doors do not clear the key byte, so the frame's press
            // survives into `$3C169` — see `tick`.
            return false;
        }
        self.idle(out);
        false
    }

    /// `$3C7D9`: the idle tail, shared by the hub's no-input path and by a
    /// coin-operated door the player cannot pay for.
    fn idle(&mut self, _out: &mut MapEntry) {
        self.cursor_walking = 0;
        self.walk_step_timer = 8;
        self.walk_frame = self.walk_frame.wrapping_add(1);
        self.cursor_pose = ((self.walk_frame & 0x18) >> 3) + 12;
    }

    /// `$3C60B`: the node's A/START action.
    ///
    /// The loader behind each door is a blocking palette fade, a zone gfx load and a
    /// script pointer (art, see the module docs); what is modelled is what it leaves
    /// behind: the cursor, the node, the screen, `sMarioDead`, the music request, the
    /// walk-out transition, and — for the four coin-operated doors — the BCD price it
    /// takes out of the purse.
    ///
    /// Returns whether the press was consumed. Every door's loader clears
    /// `hKeysPressed` (`xor a / ldh [$FF81]`) but the *refused* coin doors do not:
    /// they fall into `$3C7D9` with the press intact.
    fn dispatch_door(
        &mut self,
        _pressed: u8,
        records: &MapNodeRecords,
        out: &mut MapEntry,
    ) -> bool {
        let Some(d) = node_door(self.node) else {
            out.note(Some("map:$3C60B A/START node action"));
            return false;
        };
        if let Some(price) = d.price {
            // `$3F38F`: the coin check, and the only way a door can refuse.
            match bcd_sub_u16(self.coins_low, self.coins_high, price) {
                Some((lo, hi)) => {
                    self.coins_low = lo;
                    self.coins_high = hi;
                    // $3F38F commits the new purse and calls $2934 (the save).
                    out.saved = true;
                }
                None => {
                    out.sfx = Some(19);
                    self.idle(out);
                    return false;
                }
            }
        }
        if d.mask_move_nibble {
            self.cursor_move &= 0x0F;
        }
        if let Some(m) = d.move_ {
            self.cursor_move = m;
        }
        if d.price.is_none() {
            self.cursor_x = d.cursor_x;
            self.cursor_y = d.cursor_y;
            self.node = d.node;
            // Every zone door sets sMarioDead = $FF, which is what suppresses the
            // arrival auto-walk on the screen it lands on until the next load.
            self.mario_dead = 0xFF;
        }
        self.screen_id = d.screen_id;
        if (9..=12).contains(&self.screen_id) {
            // `$3F307` zeroes `sMapFrameCounter`; the room's even/odd steps
            // (`$60C59`, `$60BBF`) and four-frame flashes count from here.
            self.frame_counter = 0;
            self.room = self.castle_scripts.as_ref().map(|scripts| {
                let mut room = crate::castle_room::CastleRoom::enter_coin(scripts, d.screen_id);
                room.powerup = self.powerup;
                room.refresh_oam_tile(scripts);
                room
            });
        }
        self.scroll_only_latch = 0;
        self.cursor_anim = 0;
        // $3E3BA's walk-out (the zone doors); the coin doors load without one.
        if let Some(dir) = d.dir {
            out.transition = Some(dir);
            // `$3D0D7`..`$3D880`: after the hub's fade each zone door runs its
            // screen loader, which resets scroll, sprites and scripts. The
            // loader's own music request lands after that 48-frame fade, so
            // the door's request is the one this frame reports; the session
            // issues the zone's when its transition reaches the loader.
            self.load_screen(d.screen_id, 0, records);
            if d.screen_id == 8 {
                // `$3D848` is the one door that runs its loader first: `$3D880`
                // ends in `$3EEA4` (the node record), and only then does the
                // door write its cursor and node, so the door's landing wins.
                self.cursor_x = d.cursor_x;
                self.cursor_y = d.cursor_y;
                self.node = d.node;
            }
        }
        out.music = Some(d.music);
        true
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
    fn door(&mut self, keys_pressed: u8, records: &MapNodeRecords, out: &mut MapEntry) -> bool {
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
        // `$3C7CA`: the cursor's new node, `$3EEA4` (the new node's camera and
        // cursor), then the hub's screen id; `$3C7BD` moves the state to $0C and
        // `$3EA9A` asks for music 6.
        self.node = dest;
        self.load_node(dest, records);
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

    #[test]
    fn player_return_preserves_the_global_map_arrival_latch() {
        let mut player = crate::player_sim::PlayerSim::new();
        player.dead = 21;
        player.map_node = 60;
        player.level_progress[16] = 8;
        let map = MapSim::from_player(&player);
        assert_eq!(
            (map.mario_dead, map.node, map.level_progress[16]),
            (21, 60, 8)
        );
    }

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

    /// The node records, as `$3EEA4` reads them: node 0 (`00 81 01 42 4b 4c`), the
    /// story node $14, the hub node $19 a Tree-Zone door lands on, and the zone exit
    /// node $23 (`00 11 00 f2 30 6e` = camera $0011/$00F2, cursor 48/110). Every other
    /// record is zeroed: only the nodes these tests move to need real data.
    fn recs() -> MapNodeRecords {
        const REAL: &[(usize, &str)] = &[
            (0x00, "00 81 01 42 4b 4c 00 00"),
            (0x14, "00 81 01 42 4b 4c 00 00"),
            (0x19, "00 11 00 f2 4b 4c 00 00"),
            (0x1A, "00 15 00 32 4b 4c 00 00"),
            (0x23, "00 11 00 f2 30 6e 00 00"),
            (0x7A, "01 35 00 3a 13 24 00 00"),
        ];
        let mut blob = vec![0u8; MapNodeRecords::LEN];
        for (node, bytes) in REAL {
            blob[node * 8..node * 8 + 8].copy_from_slice(&hex(bytes));
        }
        MapNodeRecords::from_bytes(&blob).expect("record-sized blob")
    }

    /// A blob holding the real table row and the real stream, at the offsets the
    /// ROM addresses them at. Every other entry is `$FFFF` — no path — the way the
    /// ROM's own empty entries are, which is what sends a direction press into the
    /// node's own action (`$3C52B`'s `jr z, $3C5DA`). Node `$14` is the one the
    /// `$3C38C` chain auto-walks Up.
    fn walk_blob() -> MapWalk {
        let mut blob = vec![0xFFu8; MapWalk::TABLE_LEN];
        // Node 1 at its real offset in the table.
        blob[8..16].copy_from_slice(&hex(NODE1_ROW_HEX));
        // Node $14's row (Up = $5A99) — the chain's Up class.
        blob[0x14 * 8..0x14 * 8 + 8].copy_from_slice(&hex("ff ff ff ff 99 5a ff ff"));
        // The streams, at (their window pointer - the table's own window address).
        for (ptr, bytes) in [
            (NODE1_R, NODE1_R_STREAM_HEX),
            (0x5A99u16, "14 44 44 44 14 04 00 13"),
        ] {
            let at = (ptr - 0x563A) as usize;
            let b = hex(bytes);
            if blob.len() < at + b.len() {
                blob.resize(at + b.len(), 0);
            }
            blob[at..at + b.len()].copy_from_slice(&b);
        }
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
        b[N::NODES + 0x31] = 0x06; // a zone node with a slot, for $3C32D
        b[2 * N::NODES] = 0x00; // slot 0: no direction opens it
        b[2 * N::NODES + 1] = 0x10; // slot 1: Right
        b[2 * N::NODES + 5] = 0x40; // slot 5: Up, the castle
        b[2 * N::NODES + 6] = 0x10; // slot 6: Right, for the $3C32D test
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
            keys_held: 0x10,    // the gate reads the *held* byte for directions...
            keys_pressed: 0x10, // ...and the walk reads the pressed one
            ..Default::default()
        };
        let out = map.tick(input, &n, &walk, &recs());
        assert_eq!(map.cursor_walking, 0xFF, "the walk started");
        assert_eq!(map.cursor_move, 0x01, "the stream's first move byte");
        // The walk-start frame *is* a move frame ($3C588 jumps into $3C7EC), so
        // the 9 it loads is already one down by the time the frame ends.
        assert_eq!(
            map.walk_step_timer, 8,
            "9 frames when bit 4 is clear, less this one"
        );
        assert_eq!(map.walk_ptr, NODE1_R);
        assert_eq!(map.cursor_x, 9, "and the frame moves the cursor too");
        assert_eq!(out.level, None, "A was not pressed");

        // Release the key: the walk keeps going on its own.
        let idle = MapInput::default();
        let mut ticks = 1;
        while map.cursor_walking != 0 && ticks < 1000 {
            map.tick(idle, &n, &walk, &recs());
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
            &recs(),
        );
        assert_eq!(map.rock_anim_timer, 1, "$3C4E7's rock counter");
        assert_eq!(out.music, Some(25));
        assert_eq!(map.cursor_walking, 0xFF, "the stream walk started");
        assert_eq!(map.walk_ptr, 0x5BF6);
        assert_eq!(map.cursor_move, 0xA0, "the hardcoded stream's first byte");
        assert_eq!(
            map.walk_step_timer, 8,
            "$A0 has no bit 4: 9, less the start frame"
        );
        assert_eq!(out.level, None, "nothing was entered");

        // Walking it out lands on the stream's arrival node, $0D.
        let idle = MapInput::default();
        let mut guard = 0;
        while map.cursor_walking != 0 && guard < 2000 {
            map.tick(idle, &n, &walk, &recs());
            guard += 1;
        }
        assert_eq!(map.cursor_walking, 0, "the secret stream ended");
        assert_eq!(map.node, 0x0D, "the byte after its $00");

        // Any other pressed bit and the arm does not fire. Node $1E is used here
        // because node $0C is in the `$3C38C` chain's Start class: the map presses
        // Start for the player there, so the arm fires with no input at all — the
        // secret route is automatic once the cursor reaches either node.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x1E;
        map.tick(
            MapInput {
                keys_pressed: 0x19, // A plus a direction
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(map.rock_anim_timer, 0);
        assert_eq!(map.cursor_walking, 0);

        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x0C;
        let out = map.tick(MapInput::default(), &n, &walk, &recs());
        assert_eq!(map.rock_anim_timer, 1, "the chain presses Start by itself");
        assert_eq!(map.cursor_walking, 0xFF);
        assert_eq!(out.music, Some(25));
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
            &recs(),
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
            &recs(),
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
            &recs(),
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
        let out = map.tick(press_a, &n, &walk, &recs());
        assert_eq!(out.level, None, "the castle slot refuses");
        assert!(!out.saved);

        // The same node uncleared enters: $4218[$20] is 4, i.e. level 5. (`mario_dead`
        // is set so `$3C32D`'s arrival press does not clobber the player's A first —
        // the ROM's own quirk is that the auto-walk's synthetic direction replaces the
        // press on the frame it fires.)
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x20;
        map.mario_dead = 0xFF;
        let out = map.tick(press_a, &n, &walk, &recs());
        assert_eq!(out.level, Some(4));
        assert_eq!(out.progress_flags, 0);

        // Node 2's slot is 1, not one of the six: a cleared level re-enters.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 2;
        map.level_progress[1] = 0x80;
        let out = map.tick(press_a, &n, &walk, &recs());
        assert_eq!(out.level, Some(1), "a cleared level still re-enters");
        assert_eq!(out.progress_flags, 0x80, "and keeps its cleared bit");
    }

    /// `$3C32D`'s arrival press replaces whatever the player pressed — so A on the
    /// frame it fires cannot also enter the level, and only the next frame can.
    /// `$3CF16`: once `$3C4E7` arms the counter, the hub turtle (slot 14) swims
    /// left 42 px snapping its mouth, trails its wake, swims back 44 px, then
    /// holds; the counter wraps through `$FF` for one last left step.
    #[test]
    fn the_hub_turtle_swims_out_and_back_once_armed() {
        let mut map = MapSim::new();
        map.screen_id = 0;
        let start_x = map.decor_slots[14].1;
        assert_eq!((start_x, map.decor_slots[14].3), (0x88, 5));
        // Unarmed: the turtle does not move.
        for _ in 0..8 {
            map.bookkeeping(0x3D);
        }
        assert_eq!(map.decor_slots[14].1, start_x);

        map.rock_anim_timer = 1;
        let mut ids = Vec::new();
        for _ in 2..=0x2B {
            map.bookkeeping(0x3D);
            ids.push(map.decor_slots[14].3);
        }
        assert_eq!(map.rock_anim_timer, 0x2B);
        assert_eq!(map.decor_slots[14].1, 0x88 - 42, "one pixel left a frame");
        // Mouth shut until `$10`, then ids 6, 7, 8, 5 in eight-frame runs.
        assert!(ids[..14].iter().all(|&id| id == 5), "{ids:?}");
        assert_eq!(&ids[14..17], &[6, 6, 7]);
        assert_eq!(ids[0x1A - 2], 8);
        assert_eq!(ids[0x22 - 2], 5);
        // 42 px left of `$88`: all five wake bubbles are up.
        let wake = |map: &MapSim| [10, 11, 12, 13, 15].map(|k| map.decor_slots[k].3);
        assert_eq!(wake(&map), [11; 5]);

        for _ in 0x2C..=0x57 {
            map.bookkeeping(0x3D);
            assert_eq!(map.decor_slots[14].3, 5, "the swim back keeps its mouth shut");
        }
        assert_eq!(map.decor_slots[14].1, 0x88 - 42 + 44);
        assert_eq!(wake(&map), [0xFF; 5], "back past `$88`: no wake");

        for _ in 0x58..=0xFF {
            map.bookkeeping(0x3D);
        }
        assert_eq!(map.decor_slots[14].1, 0x8A, "held from `$58`");
        map.bookkeeping(0x3D);
        assert_eq!(map.rock_anim_timer, 0, "the counter wrapped");
        assert_eq!(map.decor_slots[14].1, 0x89, "the wrap frame steps left once more");
        map.bookkeeping(0x3D);
        assert_eq!(map.decor_slots[14].1, 0x89, "then the routine idles");
    }

    #[test]
    fn the_arrival_walk_swallows_the_players_press() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x20; // a slot-5 node with an arrival walk
        map.level_progress[5] = 0x00;
        let out = map.tick(
            MapInput {
                keys_pressed: 0x01, // A
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(out.level, None, "the synthetic Up press replaced the A");
        assert_eq!(map.mario_dead, 0xFF, "and the one-shot is spent");
        // The next frame the player's A survives the chain and enters.
        let out = map.tick(
            MapInput {
                keys_pressed: 0x01,
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(out.level, Some(4));
    }

    /// The zone screens' door cluster: walk into one and the cursor lands on the
    /// hub at another node, in state `$0C`, seeded from that node's record. Node
    /// `$23` wants Down exactly; node `$2D` has no key test at all.
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
            &recs(),
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
            &recs(),
        );
        assert_eq!(map.node, 0x23, "Left is not this door's key");
        assert_eq!(map.cursor_pose, 12, "the idle path instead");

        // Node $2D has no test: any frame at that node takes the door, which is
        // why the ROM's own $3C6E4 has no `cp`.
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 0x2D;
        let out = map.tick(MapInput::default(), &n, &walk, &recs());
        assert_eq!(map.node, 121);
        assert_eq!(map.screen_id, 0);
        assert_eq!(out.transition, Some(8));
        assert_eq!(map.cursor_move, 8);
    }

    /// The hub's B button: the cursor stays put, the pan latch is set while B is
    /// down, and B plus a direction pans the camera by hand.
    #[test]
    fn the_hub_b_button_sets_the_scroll_only_latch() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0;
        map.cam_x = 0x40;
        let out = map.tick(
            MapInput {
                keys_held: 0x02, // B
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(map.scroll_only_latch, 0xFF);
        assert_eq!(out.unported, None, "the pan is ported");
        assert_eq!(map.cursor_walking, 0, "the cursor does not walk");
        assert_eq!(map.node, 0, "and does not move");
        assert_eq!(map.cam_x, 0x40, "no direction held: no pan");

        // B plus Right pans the camera, and because the latch is set the cursor is
        // not pushed back.
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0;
        map.cam_x = 0x40;
        map.cursor_x = 0x40;
        map.tick(
            MapInput {
                keys_held: 0x12, // B + Right
                keys_pressed: 0x10,
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(map.cam_x, 0x41, "the camera pans");
        assert_eq!(map.cursor_x, 0x40, "and the cursor holds still");

        // The frame B is released, the screen re-centres on the node's record and the
        // anim byte is cleared; the latch goes with it.
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0x23;
        map.scroll_only_latch = 0xFF;
        map.rock_anim_timer = 19;
        map.cloud_drift = 77;
        let out = map.tick(MapInput::default(), &n, &walk, &recs());
        assert_eq!(map.scroll_only_latch, 0);
        assert!(
            out.hub_reload,
            "$3C518 must run the hub loader, not just re-centre"
        );
        assert_eq!(map.rock_anim_timer, 0, "$3F156 clears the rock animation");
        assert_eq!(map.cloud_drift, 0, "$3CBE7 resets decoration drift");
        assert_eq!(out.music, Some(6), "$3EA9A asks for music 6");
        assert_eq!(map.cursor_anim, 0);
        assert_eq!(
            (map.cursor_x, map.cursor_y),
            (48, 110),
            "$3EEA4 seeds the cursor from the node's record"
        );
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
            &recs(),
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

    /// `$3C13E`: source-derived divider input timing and parity integration.
    ///
    /// Hardware DIV (upper byte of the 16-bit 4,194,304 Hz counter) advances
    /// 70,224 cycles per Game Boy frame (456 cycles/line * 154 lines).
    /// Advances across 40 frames sample both parities (even and odd), accumulate
    /// through `sMapKeysXor`, and latch into `sMapRandLatch` on the 40th frame.
    #[test]
    fn map_bookkeeping_source_derived_timing_and_parities() {
        use crate::divider::Divider;

        let mut map = MapSim::new();
        let mut divider = Divider::new(0x00FF);
        let mut even_count = 0;
        let mut odd_count = 0;
        let mut expected_xor = 0u8;

        for frame in 1..=40 {
            // Nominal Game Boy frame cycle count (70,224 cycles)
            divider.advance_cycles(70_224);
            let input = MapInput::with_divider(0, 0, &divider);
            if input.div & 1 == 0 {
                even_count += 1;
            } else {
                odd_count += 1;
            }
            expected_xor ^= input.div;
            map.bookkeeping(input.div);

            assert_eq!(map.frame_counter, frame as u8);
            assert_eq!(map.keys_xor, expected_xor);
            if frame < 40 {
                assert_eq!(map.tick_timer, 40 - frame as u8);
                assert_eq!(map.rand_latch, 0);
            }
        }

        // Both parities must be sampled across the sequence:
        assert!(even_count > 0, "even parity sampled");
        assert!(odd_count > 0, "odd parity sampled");
        assert_eq!(map.frame_counter, 40);
        assert_eq!(map.tick_timer, 40, "tick timer reloads to 40");
        assert_eq!(
            map.rand_latch,
            divider.sample(),
            "40th frame latches divider sample"
        );
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
            &recs(),
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

    /// `$3C60B`: a hub node with a door loads its zone screen. Node `$02` is also in
    /// the `$3C38C` chain's Start class, so the map presses Start *itself* — no input
    /// at all is enough — and the loader's `xor a / ldh [$FF81]` consumes that press,
    /// which is what stops the same frame from also entering a level.
    #[test]
    fn a_hub_zone_door_loads_its_screen() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0x02; // the Tree Zone door
        map.cursor_move = 0x2F; // a move byte with a bump bit, to be preserved
        let out = map.tick(MapInput::default(), &n, &walk, &recs());
        assert_eq!(map.screen_id, 2, "$3D0D7 loads Tree Zone screen A");
        assert_eq!(map.node, 0x23, "the node the set-piece lands on");
        assert_eq!((map.cursor_x, map.cursor_y), (48, 158));
        assert_eq!(
            map.mario_dead, 0xFF,
            "which is what blocks the arrival walk"
        );
        assert_eq!(map.cursor_move, 0x2F, "the door leaves the move byte alone");
        assert_eq!(out.transition, Some(4), "$3E3BA walks out Up");
        assert_eq!(out.music, Some(255), "the door's own request, before the loader's");
        assert_eq!((map.cam_x, map.cam_y), (0, 0));
        assert_eq!(map.cursor_anim, 0x92);
        assert!(map.zone_slots.iter().any(|slot| slot.2 != 0xFF));
        assert_eq!(out.level, None, "the press was consumed: no level entry");
        assert_eq!(out.state, 0, "the map stays in state $0C");

        // A node that is not a door: A there does nothing but idle.
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 3;
        let out = map.tick(MapInput::default(), &n, &walk, &recs());
        assert_eq!(out.unported, None);
        assert_eq!(map.screen_id, 0);
        assert_eq!(out.state, 0);
        assert_eq!(map.cursor_pose, 12, "the idle block");
    }

    /// `$3C38C`'s chain: the map's own synthetic presses, node class by node class,
    /// and the three helpers that *retarget the node* instead of pressing a key.
    #[test]
    fn the_node_chain_presses_the_maps_own_keys() {
        let n = nodes();
        let mut map = MapSim::new();

        // The plain classes, one node each.
        for (node, want) in [
            (0x68u8, 0x10u8), // Right
            (0x65, 0x20),     // Left
            (0x2F, 0x40),     // Up
            (0x31, 0x80),     // Down
            (0x67, 0x08),     // Start
        ] {
            map.node = node;
            assert_eq!(map.synthetic_press(0x00, &n), want, "node ${node:02X}");
        }
        // A node the chain does not list keeps the player's own press.
        map.node = 0x40;
        assert_eq!(map.synthetic_press(0x10, &n), 0x10);

        // $3C2B8: A on node $35 retargets to $36 — but only with Macro path 2's
        // bit 7 set.
        map.node = 0x35;
        assert_eq!(map.synthetic_press(0x10, &n), 0x10, "no flag, no retarget");
        assert_eq!(map.node, 0x35);
        map.level_progress[progress_offset::MACRO_PATH_2] = 0x80;
        assert_eq!(map.synthetic_press(0x10, &n), 0x10, "A retargets");
        assert_eq!(map.node, 0x36);
        // ...and with B instead, the Macro gate's bit 7 sends it to $37.
        map.node = 0x35;
        map.level_progress[progress_offset::GATE_MACRO] = 0x80;
        assert_eq!(map.synthetic_press(0x20, &n), 0x20);
        assert_eq!(map.node, 0x37);

        // $3C2E7: Select on node $32 retargets to $39 on Macro path 0's bit 0.
        map.node = 0x32;
        assert_eq!(map.synthetic_press(0x40, &n), 0x40, "no flag, no retarget");
        assert_eq!(map.node, 0x32);
        map.level_progress[progress_offset::MACRO_PATH_0] = 0x01;
        map.synthetic_press(0x40, &n);
        assert_eq!(map.node, 0x39);

        // $3C2FE: A on node $6A retargets to $6C on the Macro gate.
        map.node = 0x6A;
        map.synthetic_press(0x10, &n);
        assert_eq!(map.node, 0x6C);
    }

    /// $3C0DF/$3C10E run bell-room scripts without $3C38C's arrival walk.
    #[test]
    fn bell_rooms_preserve_the_live_mario_byte_for_the_deferred_clear() {
        for screen in [13, 14] {
            let mut map = MapSim::new();
            map.screen_id = screen;
            map.node = 0x50;
            map.tick(MapInput::default(), &nodes(), &walk_blob(), &recs());
            assert_eq!([map.mario_dead, map.node], [0, 0x50]);
        }
    }

    /// `$3C32D`: a zone node presses its own `$6591` mask once per screen. The mask
    /// wins over the chain's class press, and `sMarioDead` is the latch that stops it
    /// firing again.
    #[test]
    fn the_zone_arrival_walk_fires_once_per_screen() {
        let n = nodes();
        let mut map = MapSim::new();
        map.node = 0x31; // a Down-class node and a slot-6 node
        map.screen_id = 1;
        assert_eq!(map.synthetic_press(0x00, &n), 0x10, "the mask wins");
        assert_eq!(map.mario_dead, 0xFF, "and the one-shot is spent");
        // Next frame the chain's own class press runs instead.
        assert_eq!(map.synthetic_press(0x00, &n), 0x80);

        // The clear bit `$3EEF0` writes is the slot. `$3C32D` tests `$3C364[slot]`
        // (`$3C364[6] = 7`), so marking slot 6 leaves the walk armed.
        let mut map = MapSim::new();
        map.node = 0x31;
        map.screen_id = 1;
        map.level_progress[6] = 0x80;
        assert_eq!(map.synthetic_press(0x00, &n), 0x10, "slot 6 is not the byte tested");
        assert_eq!(map.mario_dead, 0xFF, "the one-shot is spent either way");

        let mut map = MapSim::new();
        map.node = 0x31;
        map.screen_id = 1;
        map.level_progress[7] = 0x80;
        assert_eq!(map.synthetic_press(0x00, &n), 0x80, "$3C364[6] cleared suppresses it");
        assert_eq!(map.mario_dead, 0xFF);
    }

    /// Level 10's return: node `$50` is cleared (`sLevelProgress[$1A] = $80`) and
    /// `$3C364[$1A] = $1B` is not, so the cursor still presses Up toward node `$51`.
    #[test]
    fn clearing_mario_zone_1_still_walks_to_the_next_node() {
        let nodes = MapNodes::from_bytes(include_bytes!("../../../extracted/map/nodes.bin"))
            .expect("nodes");
        let mut map = MapSim::new();
        map.node = 0x50;
        map.screen_id = 6;
        map.level_progress[0x1A] = 0x80;
        assert_eq!(map.synthetic_press(0x00, &nodes), 0x40);
        assert_eq!(map.mario_dead, 0xFF);

        let mut done = MapSim::new();
        done.node = 0x50;
        done.screen_id = 6;
        done.level_progress[0x1A] = 0x80;
        done.level_progress[0x1B] = 0x80;
        assert_eq!(done.synthetic_press(0x00, &nodes), 0x00);
    }

    /// `$3C316`: node 0 is the story entry. Once level 1 is cleared the cursor is
    /// moved to node `$14`, whose chain class then walks it Up off the node.
    #[test]
    fn node_zero_hands_over_to_the_story_node_once_level_one_is_cleared() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0;
        map.level_progress[0] = 0x00;
        assert_eq!(map.synthetic_press(0x00, &n), 0x00);
        assert_eq!(map.node, 0, "level 1 uncleared: node 0 stays");

        map.level_progress[0] = 0x80;
        assert_eq!(map.synthetic_press(0x00, &n), 0x00);
        assert_eq!(map.node, 0x14, "$3C316 moved it");

        // And the next frame the chain auto-walks the cursor off node $14 with no
        // input at all: its Up stream is `14 44 44 44 14 04 00 -> node $13`.
        let out = map.tick(MapInput::default(), &n, &walk, &recs());
        assert_eq!(
            map.cursor_walking, 0xFF,
            "the map pressed Up for the player"
        );
        assert_eq!(map.cursor_move, 0x14, "the stream's first byte");
        assert_eq!(out.level, None);
        let idle = MapInput::default();
        let mut guard = 0;
        while map.cursor_walking != 0 && guard < 2000 {
            map.tick(idle, &n, &walk, &recs());
            guard += 1;
        }
        assert_eq!(map.cursor_walking, 0);
        assert_eq!(map.node, 0x13, "the stream's arrival node");
    }

    /// `$3F38F`: the coin doors' BCD price. Payable takes the coins and loads the
    /// bonus screen with no walk-out; unpayable clicks away and changes nothing.
    #[test]
    fn the_coin_doors_take_their_bcd_price() {
        let (n, walk) = (nodes(), walk_blob());
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0x70; // $0200
        map.coins_low = 0x00;
        map.coins_high = 0x20; // BCD 2000
        let out = map.tick(
            MapInput {
                keys_pressed: 0x01, // A
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(map.screen_id, 10, "$3F3F0 loads bonus screen 10");
        assert_eq!((map.coins_low, map.coins_high), (0x00, 0x18), "2000 - 200");
        assert_eq!(out.music, Some(21));
        assert_eq!(out.sfx, None, "the click is for the refused door only");
        assert!(out.saved, "$3F38F calls $2934");
        assert_eq!(out.transition, None, "the coin doors run no walk-out");
        assert_eq!(map.node, 0x70, "and do not move the cursor or the node");

        // Not enough coins: the door answers with sfx 19 and the idle path, and the
        // press is NOT consumed (the refusal does not run a loader).
        let mut map = MapSim::new();
        map.screen_id = 0;
        map.node = 0x70;
        map.coins_low = 0x50;
        map.coins_high = 0x01; // BCD 150
        let out = map.tick(
            MapInput {
                keys_pressed: 0x01,
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(
            out.sfx,
            Some(2),
            "$3C201's click overwrites the refusal's 19"
        );
        assert_eq!((map.coins_low, map.coins_high), (0x50, 0x01), "left alone");
        assert_eq!(map.screen_id, 0);
        assert_eq!(map.node, 0x70);
        assert_eq!(map.cursor_pose, 12, "the idle block");
        assert!(!out.saved);
    }

    /// Screens 9-14 dispatch to the bank-24 room script (`$3C0C3`..`$3C10E`),
    /// not to `$3C38C`'s cursor chain: A starts the coin room's walk-in and the
    /// cursor never moves.
    #[test]
    fn bonus_screens_run_the_room_script() {
        let (n, walk) = (nodes(), walk_blob());
        let scripts = crate::castle::CastleScripts::from_parts(
            include_bytes!("../../../extracted/map/castle_scripts.bin").to_vec(),
            include_str!("../../../extracted/map/castle_scripts.json"),
        )
        .expect("castle scripts asset");
        let mut map = MapSim::new();
        map.screen_id = 9;
        map.node = 0x70;
        map.cursor_x = 40;
        let mut room = crate::castle_room::CastleRoom::coin(&scripts);
        room.set_sprite(crate::castle_room::SPRITES_AT + 8, 7); // sprite 2 page
        map.room = Some(room);
        let before = map.cursor_x;
        let out = map.tick(
            MapInput {
                keys_pressed: 0x01,
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(map.cursor_x, before, "no cursor chain on a bonus screen");
        let room = map.room.as_ref().unwrap();
        assert_eq!(room.state, 5, "$4A81 started the walk-in");
        assert_eq!(out.sfx, Some(1), "the start sfx reaches MapEntry");
        assert_eq!(room.sfx[0], 0, "and is cleared in the room");
    }

    /// The `$71` (999-coin) room against a PyBoy trace. `$3F307` zeroes
    /// `sMapFrameCounter`; with A pressed on counter 204 the ROM hops Mario
    /// (lowest `$FFC4` 86, back to 124), enters state 2 on counter 105 with
    /// item 11, and awards 20 lives ($05 -> $25).
    #[test]
    fn the_999_room_matches_the_rom_trace() {
        let (n, walk) = (nodes(), walk_blob());
        let scripts = crate::castle::CastleScripts::from_parts(
            include_bytes!("../../../extracted/map/castle_scripts.bin").to_vec(),
            include_str!("../../../extracted/map/castle_scripts.json"),
        )
        .expect("castle scripts asset");
        let mut map = MapSim::new();
        map.set_castle_scripts(scripts);
        map.screen_id = 0;
        map.node = 0x71;
        map.coins_low = 0x99;
        map.coins_high = 0x09;
        map.lives = 0x05;
        map.frame_counter = 0x37; // the hub's counter runs freely before the door
        let press = |pressed: u8| MapInput { keys_pressed: pressed, ..Default::default() };
        map.tick(press(0x01), &n, &walk, &recs());
        assert_eq!(map.screen_id, 9);

        let (mut lowest_y, mut state2_at) = (u8::MAX, None);
        // The player's A lands a fixed time after the door: tick 202, which is
        // counter 204 once `$3F307` has reset it.
        for tick in 0..700 {
            let a = if tick == 202 { 0x01 } else { 0 };
            map.tick(press(a), &n, &walk, &recs());
            let Some(room) = map.room.as_ref() else { break };
            if room.state == 5 || room.state == 1 {
                lowest_y = lowest_y.min(room.oam[0]);
            }
            if room.state == 2 && state2_at.is_none() {
                state2_at = Some((map.frame_counter, room.item_index));
            }
        }
        assert_eq!(lowest_y, 86, "the hop peaks at $FFC4 = 86 and never wraps");
        assert_eq!(state2_at, Some((105, 11)));
        assert_eq!(map.lives, 0x25);
        assert_eq!(map.screen_id, 8, "the room hands back to the castle screen");
    }

    /// `$3F3F0` stages a coin room; `$3F4A0` returns to the castle loader once
    /// the script writes state 3.
    #[test]
    fn a_coin_door_stages_its_room_and_returns_to_the_castle() {
        let (n, walk) = (nodes(), walk_blob());
        let scripts = crate::castle::CastleScripts::from_parts(
            include_bytes!("../../../extracted/map/castle_scripts.bin").to_vec(),
            include_str!("../../../extracted/map/castle_scripts.json"),
        )
        .expect("castle scripts asset");
        let mut map = MapSim::new();
        map.set_castle_scripts(scripts);
        map.screen_id = 0;
        map.node = 0x70; // $0200, screen 10
        map.coins_low = 0x00;
        map.coins_high = 0x20;
        map.tick(
            MapInput {
                keys_pressed: 0x01,
                ..Default::default()
            },
            &n,
            &walk,
            &recs(),
        );
        assert_eq!(map.screen_id, 10);
        let room = map.room.as_ref().expect("the door staged a room");
        assert_eq!(room.kind, crate::castle_room::RoomKind::CoinRooms);
        assert_eq!(room.oam, [156, 40, 0x30, 0], "$3F35A's OAM entry");
        assert_eq!(room.sfx[0], 5, "$3F495's SFX request");
        assert_eq!(&room.sprites[..3], &[0x20, 0xA0, 0x00], "$671C record 0");

        // The script reaching state 3 returns through `$3D880`.
        map.room.as_mut().unwrap().state = 3;
        let out = map.tick(MapInput::default(), &n, &walk, &recs());
        assert_eq!(map.screen_id, 8);
        assert!(map.room.is_none(), "the room is done");
        assert_eq!(out.music, Some(13), "$3D880 asks for music 13");
    }

    /// `$3F38F`'s arithmetic, checked directly: BCD with a borrow, and a `None` — the
    /// ROM's `jr c` — whenever the price is above the purse.
    #[test]
    fn the_coin_price_check_is_bcd_with_a_borrow() {
        assert_eq!(
            bcd_sub_u16(0x00, 0x20, 0x0999),
            Some((0x01, 0x10)),
            "2000-999"
        );
        assert_eq!(bcd_sub_u16(0x50, 0x01, 0x0200), None, "150 < 200");
        assert_eq!(bcd_sub_u16(0x00, 0x00, 0x0000), Some((0x00, 0x00)));
        assert_eq!(bcd_sub_u16(0x99, 0x00, 0x0099), Some((0x00, 0x00)), "exact");
        assert_eq!(
            bcd_sub_u16(0x00, 0x01, 0x0050),
            Some((0x50, 0x00)),
            "100-50"
        );
        assert_eq!(bcd_sub_u16(0x30, 0x00, 0x0030), Some((0x00, 0x00)));
        assert_eq!(bcd_sub_u16(0x29, 0x00, 0x0030), None, "29 < 30");
    }

    /// The hub pan: when the cursor reaches the position gate the camera takes the
    /// step and pushes the cursor back — unless the player is holding B, in which case
    /// the cursor keeps the step and just the camera moves.
    #[test]
    fn the_hub_pan_holds_the_cursor_at_the_position_gate() {
        fn step(latch: u8, x: u8, cam: u16) -> MapSim {
            let mut map = MapSim::new();
            map.screen_id = 0;
            map.cursor_move = 0x01; // Right
            map.cursor_x = x;
            map.cursor_y = 0x40;
            map.cam_x = cam;
            map.scroll_only_latch = latch;
            map.cursor_walking = 0xFF;
            map.walk_step_timer = 0xFF;
            map.cursor_anim = 0;
            map.walk_step(&walk_blob());
            map
        }
        let m = step(0x00, 0x4B, 0x40);
        assert_eq!(
            (m.cursor_x, m.cam_x),
            (0x4B, 0x41),
            "the camera takes the pixel and the cursor is held"
        );
        let m = step(0xFF, 0x4B, 0x40);
        assert_eq!(
            (m.cursor_x, m.cam_x),
            (0x4C, 0x41),
            "with B held the cursor keeps the step"
        );
        // Away from the gate nothing pans.
        let m = step(0x00, 0x20, 0x40);
        assert_eq!((m.cursor_x, m.cam_x), (0x21, 0x40));
        // And the camera's own ceiling stops the pan: at $60 in a non-zero page.
        let m = step(0x00, 0x4B, 0x0160);
        assert_eq!((m.cursor_x, m.cam_x), (0x4C, 0x0160), "no pan past $5F");
    }

    /// The node records, pinned to the ROM's own bytes at the ROM's own offsets.
    #[test]
    fn the_node_records_are_pinned_to_the_roms_bytes() {
        let r = recs();
        let n23 = r.record(0x23).expect("node $23");
        assert_eq!((n23.cam_x, n23.cam_y), (0x0011, 0x00F2));
        assert_eq!((n23.cursor_x, n23.cursor_y), (48, 110));
        let n0 = r.record(0).expect("node 0");
        assert_eq!((n0.cam_x, n0.cam_y), (0x0081, 0x0142));
        assert_eq!(
            (n0.cursor_x, n0.cursor_y),
            (75, 76),
            "the hub's default cursor"
        );
        assert_eq!(r.record(128), None, "past the table");

        // `$3EEA4` writes all four bytes.
        let mut map = MapSim::new();
        map.load_node(0x23, &r);
        assert_eq!(
            (map.cam_x, map.cam_y, map.cursor_x, map.cursor_y),
            (0x0011, 0x00F2, 48, 110)
        );
        // With no records loaded, nothing is seeded and nothing panics.
        let mut map = MapSim::new();
        map.cursor_x = 9;
        map.load_node(0x23, &MapNodeRecords::default());
        assert_eq!(map.cursor_x, 9);
        assert!(
            MapNodeRecords::from_bytes(&[0u8; 8]).is_none(),
            "a short blob"
        );
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
        assert_eq!(walk.byte_at(0x5A99), Some(0x14), "node $14's stream");
        assert_eq!(
            walk.byte_at(0x5AA0),
            Some(0x13),
            "the arrival node is the byte right before node 1's stream"
        );
        assert_eq!(walk.byte_at(0x4000), None, "outside the extracted span");
    }

    /// `$3D0FF`..`$3D880` and `$3F156`: zone screen loaders and overworld hub loader.
    ///
    /// Verifies that loading each screen sets screen_id, camera/scroll, cursor_anim,
    /// path bits, node retargets, and requests the exact ROM music track.
    #[test]
    fn zone_screen_loaders_state_effects_and_music() {
        use crate::map::progress_offset as po;
        let r = recs();

        // Screen 0: Hub ($3F156)
        let mut map = MapSim::new();
        map.node = 0x23;
        map.rock_anim_timer = 5;
        map.cursor_anim = 0x92;
        let music = map.load_screen_hub(&r);
        assert_eq!(map.screen_id, 0);
        assert_eq!(map.rock_anim_timer, 0);
        assert_eq!(map.cursor_anim, 0);
        assert_eq!(
            (map.cam_x, map.cam_y),
            (0x0011, 0x00F2),
            "seeded from record"
        );
        assert_eq!(music, 6, "hub music track 6");

        // Screen 1: Pumpkin Zone ($3D208)
        let mut map = MapSim::new();
        map.node = 0x2C;
        map.cam_x = 100;
        map.cam_y = 200;
        let music = map.load_screen_pumpkin(0x80, &r);
        assert_eq!(map.screen_id, 1);
        assert_eq!((map.cam_x, map.cam_y), (0, 0), "scroll reset to 0");
        assert_eq!(map.cursor_anim, 0x92);
        assert_eq!(map.node, 44, "node $2C secret clear retargets to 44");
        assert_eq!(
            map.level_progress[po::PUMPKIN_PATH_1] & 1,
            1,
            "pumpkin path 1 bit 0"
        );
        assert_eq!(music, 30, "pumpkin zone music track 30");

        let mut map = MapSim::new();
        map.node = 0x2F;
        map.load_screen_pumpkin(0x80, &r);
        assert_eq!(map.node, 47, "node $2F secret clear retargets to 47");
        assert_eq!(
            map.level_progress[po::PUMPKIN_PATH_2] & 1,
            1,
            "pumpkin path 2 bit 0"
        );

        // Screen 2: Tree Zone screen A ($3D0FF)
        let mut map = MapSim::new();
        map.node = 0x23;
        let music = map.load_screen_tree(0x80, &r);
        assert_eq!(map.screen_id, 2);
        assert_eq!((map.cam_x, map.cam_y), (0, 0));
        assert_eq!(map.cursor_anim, 0x92);
        assert_eq!(map.node, 37, "secret clear retargets to node 37");
        assert_eq!(map.level_progress[po::TREE_PATH_1] & 1, 1);
        assert_eq!(music, 16, "tree zone music track 16");

        // Tree Zone node $26 clear ($3D1B5)
        let mut map = MapSim::new();
        map.node = 0x26;
        map.load_screen_tree(0, &r);
        assert_eq!(map.node, 39, "node $26 retargets to 39");
        assert_eq!(map.mario_dead, 0xFF, "mario_dead set to $FF");

        // Screen 3: Macro Zone ($3D371)
        let mut map = MapSim::new();
        map.node = 0x32;
        let music = map.load_screen_macro(0x80, &r);
        assert_eq!(map.screen_id, 3);
        assert_eq!((map.cam_x, map.cam_y), (0, 0));
        assert_eq!(map.cursor_move, 0x21, "cursor move byte $21");
        assert_eq!(map.cursor_anim, 0x92);
        assert_eq!(map.node, 107, "node $32 secret clear retargets to 107");
        assert_eq!(map.level_progress[po::MACRO_PATH_0] & 1, 1);
        assert_eq!(music, 5, "macro zone music track 5");

        let mut map = MapSim::new();
        map.node = 0x6A;
        map.load_screen_macro(0, &r);
        assert_eq!(map.node, 108, "node $6A retargets to 108");
        assert_eq!(
            map.level_progress[po::GATE_MACRO] & 0x80,
            0x80,
            "macro gate bit 7"
        );
        assert_eq!(map.mario_dead, 0xFF);

        // Screen 4: Space Zone ($3D49F)
        let mut map = MapSim::new();
        map.node = 0x40;
        let music = map.load_screen_space(0x80, &r);
        assert_eq!(map.screen_id, 4);
        assert_eq!((map.cam_x, map.cam_y), (0, 0));
        assert_eq!(map.cursor_anim, 0x92);
        assert_eq!(map.node, 67, "secret clear retargets to node 67");
        assert_eq!(map.level_progress[po::SPACE_PATH_0] & 1, 1);
        assert_eq!(music, 28, "space zone music track 28");

        let mut map = MapSim::new();
        map.node = 0x44;
        map.load_screen_space(0, &r);
        assert_eq!(map.node, 69, "node $44 retargets to 69");
        assert_eq!(map.mario_dead, 0xFF);

        // Screen 5: Turtle Zone ($3D61F)
        let mut map = MapSim::new();
        map.node = 0x46;
        let music = map.load_screen_turtle(0x80, &r);
        assert_eq!(map.screen_id, 5);
        assert_eq!((map.cam_x, map.cam_y), (0, 0));
        assert_eq!(map.cursor_anim, 0x92);
        assert_eq!(map.level_progress[po::TURTLE_PATH_1] & 1, 1);
        assert_eq!(music, 18, "tree zone B music track 18");

        // Screen 6: Mario Zone ($3D79A)
        let mut map = MapSim::new();
        map.node = 0x50;
        let music = map.load_screen_mario(&r);
        assert_eq!(map.screen_id, 6);
        assert_eq!((map.cam_x, map.cam_y), (0, 0));
        assert_eq!(map.cursor_anim, 0x92);
        assert_eq!(music, 27, "mario zone music track 27");

        // Screen 7: Wario's Castle ($3D6EB)
        let mut map = MapSim::new();
        map.node = 0x5A;
        let music = map.load_screen_wario_castle(&r);
        assert_eq!(map.screen_id, 7);
        assert_eq!((map.cam_x, map.cam_y), (0, 0));
        assert_eq!(map.cursor_anim, 0x92);
        assert_eq!(music, 14, "Wario's Castle music track 14");

        // Screen 8: Castle ($3D880)
        let mut map = MapSim::new();
        map.node = 0x72;
        let music = map.load_screen_castle(&r);
        assert_eq!(map.screen_id, 8);
        assert_eq!((map.cam_x, map.cam_y), (0, 0));
        assert_eq!(map.cursor_anim, 0x92);
        assert_eq!(music, 13, "castle music track 13");

        // Dead Mario suppresses secret exit path unlocks and retargets
        let mut map = MapSim::new();
        map.node = 0x23;
        map.mario_dead = 0xFF;
        map.load_screen_tree(0x80, &r);
        assert_eq!(map.node, 0x23, "dead mario does not retarget");
        assert_eq!(
            map.level_progress[po::TREE_PATH_1],
            0,
            "path 1 bit 0 not set"
        );
    }
}

#[cfg(test)]
mod hub_decor_tests {
    use super::*;

    fn map() -> MapSim {
        let mut m = MapSim::new();
        m.screen_id = 0;
        m
    }

    /// `$3CE6B`: every 8 frames the coin phase advances and rewrites the ids of
    /// the six coin slots (4-9) that are live; `$FF` slots stay empty.
    #[test]
    fn coin_ids_cycle_two_three_four_every_eight_frames() {
        let mut m = map();
        m.decor_slots[4].3 = 2; // a collected golden coin
        for slot in 5..10 {
            m.decor_slots[slot].3 = 0xFF;
        }
        let mut seen = Vec::new();
        for _ in 0..24 {
            m.bookkeeping(0x10); // div chosen so no sparkle re-arm
            seen.push((m.frame_counter, m.decor_slots[4].3, m.decor_slots[5].3));
        }
        // Frame 8 -> phase 1 -> id 3; 16 -> 4; 24 -> 2.
        assert_eq!(seen[7], (8, 3, 0xFF));
        assert_eq!(seen[15], (16, 4, 0xFF));
        assert_eq!(seen[23], (24, 2, 0xFF));
    }

    /// `$3CE93`: slots 1-3 move one pixel every 4th frame, and the direction
    /// flips with bit 6 of `sMapCloudDriftCounter`.
    #[test]
    fn cloud_slots_drift_and_reverse_on_counter_bit_six() {
        let mut m = map();
        let start: [u8; 3] = [m.decor_slots[1].1, m.decor_slots[2].1, m.decor_slots[3].1];
        for _ in 0..4 {
            m.bookkeeping(0x10);
        }
        assert_eq!(m.cloud_drift, 1);
        for (i, _slot) in (1..4).enumerate() {
            assert_eq!(
                m.decor_slots[i + 1].1,
                start[i].wrapping_sub(1),
                "first step is left"
            );
        }
        // Walk the counter to 0x40: 64 left steps, then the direction flips.
        while m.cloud_drift != 0x40 {
            m.bookkeeping(0x10);
        }
        let mid: [u8; 3] = [m.decor_slots[1].1, m.decor_slots[2].1, m.decor_slots[3].1];
        // The step is applied *after* the counter increments, so reaching 0x40
        // already took one rightward step: 63 left then 1 right.
        for (i, _slot) in (1..4).enumerate() {
            assert_eq!(
                m.decor_slots[i + 1].1,
                start[i].wrapping_sub(63).wrapping_add(1),
                "63 left, then bit 6 turns the last step around"
            );
        }
        for _ in 0..4 {
            m.bookkeeping(0x10);
        }
        for (i, _slot) in (1..4).enumerate() {
            assert_eq!(
                m.decor_slots[i + 1].1,
                mid[i].wrapping_add(1),
                "bit 6 flips it"
            );
        }
    }

    /// `$3CEC4`: `rDIV & $72 == 0` arms the sparkles, which then show on odd
    /// countdown values at the `$4FA8`/`$4FB8` positions.
    #[test]
    fn sparkles_arm_from_the_divider() {
        let mut m = map();
        assert_eq!(m.decor_slots[16].3, 0xFF, "starts hidden");
        // div 0x00 satisfies `& $72 == 0`.
        for _ in 0..4 {
            m.bookkeeping(0x00);
        }
        assert_eq!(m.sparkle_timer, 3, "armed 4, then the body decremented");
        assert_eq!(m.decor_slots[16].3, 0x0A);
        assert_eq!(m.decor_slots[17].3, 0x0A);
        assert_eq!(m.decor_slots[16].1, crate::map::SPARKLE_SLOT_16[0]);
        assert_eq!(m.decor_slots[16].2, crate::map::SPARKLE_SLOT_16[1]);
        // A divider that never matches leaves them hidden.
        let mut n = map();
        for _ in 0..16 {
            n.bookkeeping(0x72);
        }
        assert_eq!(n.sparkle_timer, 0);
        assert_eq!(n.decor_slots[16].3, 0xFF);
    }
}

#[cfg(test)]
mod hub_coin_tests {
    use super::*;

    /// `$3CE43`: a coin icon exists only where the matching `sGoldenCoinN` byte
    /// has bit 7 set, and `$3CE6B` then animates the live ones through 2/3/4.
    #[test]
    fn coin_icons_follow_the_golden_coin_flags() {
        let mut m = MapSim::new();
        m.screen_id = 0;
        m.level_progress[crate::map::HUB_COIN_FLAGS[0]] = 0x80;
        m.level_progress[crate::map::HUB_COIN_FLAGS[3]] = 0x80;
        m.seed_hub_coin_icons();
        assert_eq!(m.decor_slots[4].3, 2, "coin 1 is live");
        assert_eq!(m.decor_slots[7].3, 2, "coin 4 is live");
        assert_eq!(m.decor_slots[5].3, 0xFF, "uncollected coins stay empty");
        assert_eq!(m.decor_slots[6].3, 0xFF);
        // The 8-frame phase moves live ids to 3 and leaves empty slots alone.
        for _ in 0..8 {
            m.bookkeeping(0x10);
        }
        assert_eq!(m.decor_slots[4].3, 3);
        assert_eq!(m.decor_slots[7].3, 3);
        assert_eq!(m.decor_slots[5].3, 0xFF);
    }
}

#[cfg(test)]
mod pumpkin_decor_tests {
    use super::*;

    fn map() -> MapSim {
        let mut m = MapSim::new();
        m.screen_id = 1;
        m.zone_slots = [
            (0, 76, 0x4F),
            (1, 16, 0x78),
            (54, 255, 16),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
            (0, 0, 0xFF),
        ];
        m
    }

    /// The per-step deltas measured on the console: `v = $01` -> y+1 only,
    /// `$81` -> y+1 only, `$91` -> x-1 y+1, `$90` -> x-1 only. The step table
    /// entry at `sMapAnimIndexA` is what moves slot 2; `sMapSprite1Y` is its x
    /// and `sMapSprite1Id` its y, one slot up from the names.
    #[test]
    fn slot_two_steps_match_the_console() {
        for (idx_value, dx, dy) in [
            (0x01u8, 0i32, 1i32),
            (0x81, 0, 1),
            (0x91, -1, 1),
            (0x90, -1, 0),
        ] {
            let mut m = map();
            m.anim_index = 0x4F; // wraps to 0 on the first step
            m.zone_slots[2] = (0x40, 0x40, 25);
            // Force the step table to this value by seeding the index that reads it.
            let step_index = crate::map::PUMPKIN_STEPS
                .iter()
                .position(|&v| v == idx_value)
                .expect("the step table holds these values");
            m.anim_index = (step_index as u8).wrapping_sub(1) & 0x7F;
            m.tick_timer = 1; // odd -> the step branch runs
            m.frame_counter = 1; // not a multiple of 8, so no drift
            m.pumpkin_decor_tick();
            assert_eq!(
                (
                    m.zone_slots[2].0 as i32 - 0x40,
                    m.zone_slots[2].1 as i32 - 0x40
                ),
                (dx, dy),
                "v = {idx_value:#04x}"
            );
        }
    }

    /// The two id tracks: slot 0 from `$5A21` by phase bits 3-5, slot 1 from
    /// `$5A29` by phase bits 2-3, and slot 2's id from `22 + $5A16` plus 3.
    #[test]
    fn the_phase_drives_slot_zero_and_one_ids() {
        let mut m = map();
        m.tick_timer = 2; // even: no step
        m.frame_counter = 1;
        let mut seen0 = std::collections::BTreeSet::new();
        let mut seen1 = std::collections::BTreeSet::new();
        for _ in 0..64 {
            m.pumpkin_decor_tick();
            seen0.insert(m.zone_slots[0].2);
            seen1.insert(m.zone_slots[1].2);
        }
        assert_eq!(
            seen0.into_iter().collect::<Vec<_>>(),
            vec![0x0F, 0x10, 0x11]
        );
        assert_eq!(
            seen1.into_iter().collect::<Vec<_>>(),
            vec![0x12, 0x13, 0x14, 0x15]
        );
        assert!(
            (22..=0x1A).contains(&m.zone_slots[2].2),
            "slot 2 id {}",
            m.zone_slots[2].2
        );
    }

    /// `sMapSprite1Page` is slot 1's y and drifts one pixel every 8 frames, the
    /// direction taken from frame-counter bit 5: clear -> +1, set -> -1. Over
    /// 64 frames the two halves cancel.
    #[test]
    fn slot_one_y_drifts_every_eight_frames() {
        let mut m = map();
        m.tick_timer = 2; // even: no step
        let start = m.zone_slots[1].1;
        // fc = 8, 16, 24 are bit-5 clear (+1 each), 32 is set (-1).
        for fc in 1..=32u8 {
            m.frame_counter = fc;
            m.pumpkin_decor_tick();
        }
        assert_eq!(
            m.zone_slots[1].1,
            start.wrapping_add(3).wrapping_sub(1),
            "three +1 steps then one -1"
        );
    }
}

#[path = "map/zone_sprite_loading.rs"]
mod zone_sprite_loading;

#[path = "map/zone_animation.rs"]
mod zone_animation;
pub use zone_animation::ZoneAnimation;

#[path = "map/screen_transition.rs"]
mod screen_transition;
