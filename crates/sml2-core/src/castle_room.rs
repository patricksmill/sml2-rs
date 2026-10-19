//! Bank-24 castle room scripts.
//!
//! `$3E00` only switches banks and jumps, so the three room "scripts" are
//! ordinary code: `$4A81` for the coin doors (screens 9-12), `$4512` and
//! `$4028` for the two bell rooms (screens 13/14). They share one helper set
//! (`$60C41` preview, `$60D08` walk, `$60D32` reveal decode, `$60D80` marker,
//! `$60DC7` award, `$60EB5`/`$60EE6`/`$6106D` commit) and a state byte
//! (`sCastleState`) that each frame's bank-15 screen handler dispatches on.
//!
//! The sprite table is the 48 bytes at `$A6B5` read as 3-byte records
//! `(page, x, id)`; the `sMapSpriteNPage/X/Y/Id` names in `ram.asm` describe
//! the hub's 4-byte view, so this module indexes the bytes by address instead.

use crate::castle::CastleScripts;

/// `sMapSprite0Page`: the 16-record, 3-byte-per-record sprite table.
pub const SPRITES_AT: u16 = 0xA6B5;
pub const SPRITE_BYTES: usize = 48;

/// `$4C4D` (inside the `$4A81` blob): `sCurPowerup` -> preview tile.
pub const POWERUP_PREVIEW_AT: usize = 0x4C4D - 0x4A81;
pub const POWERUP_PREVIEW_LEN: usize = 12;

/// `$4C51` (inside the `$4A81` blob): `sCastleItemIndex` -> preview tile.
pub const ITEM_PREVIEW_AT: usize = 0x4C51 - 0x4A81;

/// `$4F5A` table block (`coin_room_tables`).
pub const TABLE_SPRITE3_PHASE: [usize; 4] = [0x00, 0x04, 0x08, 0x0C];
pub const TABLE_ITEM_SFX: usize = 0x10;
pub const TABLE_NODE_ITEM_PTRS: usize = 0x1C;
pub const TABLE_ITEM_COLUMNS: usize = 0x24;
pub const TABLE_WALK_OFFSETS: usize = 0x4E;
pub const TABLE_REVEAL_STEPS: usize = 0x73;

// Sprite-table addresses the `$4A81` script touches.
const SPRITE1_PAGE: u16 = 0xA6B9;
const SPRITE1_X: u16 = 0xA6BA;
const SPRITE2_PAGE: u16 = 0xA6BD;
const SPRITE2_ID: u16 = 0xA6C0;
const SPRITE3_PAGE: u16 = 0xA6C1;
const SPRITE3_X: u16 = 0xA6C2;
const SPRITE3_Y: u16 = 0xA6C3;
const SPRITE3_ID: u16 = 0xA6C4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind {
    /// `$4A81`, screens 9-12.
    CoinRooms,
    /// `$4512`, screen 13.
    BellRoomA,
    /// `$4028`, screen 14.
    BellRoomB,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoomInput {
    pub keys_pressed: u8,
    pub frame: u8,
    pub node: u8,
}

/// The bytes the scripts touch outside their own RAM block.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoomRewards {
    pub powerup: u8,
    /// BCD.
    pub lives: u8,
    /// BCD.
    pub coins_low: u8,
    pub coins_high: u8,
    pub save: bool,
}

/// One of the five `$0D`-screen slots (`$A7D0` value, `$A7DA` counter,
/// `$A7DF` delay).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoomSlot {
    pub value: u8,
    pub counter: u8,
    pub delay: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastleRoom {
    pub kind: RoomKind,
    /// `sCastleState`.
    pub state: u8,
    /// `sCastleStateTimer`.
    pub timer: u8,
    /// `sCastleWalkScriptIdx`.
    pub walk_idx: u8,
    /// `sCastleRevealProgress`.
    pub reveal_progress: u8,
    /// `sCastleRevealTick`.
    pub reveal_tick: u8,
    /// `sCastleItemIndex`.
    pub item_index: u8,
    /// `sCastleItemPreview`.
    pub item_preview: u8,
    /// `sCastleSavedPowerup`.
    pub saved_powerup: u8,
    /// `sCastleItemReached`.
    pub item_reached: u8,
    /// `sCastleMarkerCol`.
    pub marker_col: u8,
    /// `sCastleShufflePhase`.
    pub shuffle_phase: u8,
    /// `sCastleWalkPhase`.
    pub walk_phase: u8,
    /// `sMapSprite1AnimCount`.
    pub anim_count: u8,
    /// `sMapAnimStep`.
    pub anim_step: u8,
    /// `sMapAnimAccum`.
    pub anim_accum: u8,
    /// `sCurPowerup` as the script sees it.
    pub powerup: u8,
    /// The 16 3-byte records at `$A6B5`.
    pub sprites: [u8; SPRITE_BYTES],
    /// The 48-byte patch list at `$A7D0`.
    pub patches: [u8; 48],
    /// `$A7DA`/`$A7DF`: five slots.
    pub slots: [RoomSlot; 5],
    /// `sCastleSlotIndex` (`$A7E9`).
    pub slot_index: u8,
    /// OAM entries 0-3 (`$FFC4..$C7`): x, y, tile, attr.
    pub oam: [u8; 4],
    /// `sSfxRequest0`/`1`.
    pub sfx: [u8; 2],
    /// `sMusicRequest`.
    pub music: u8,
    /// `sGoalBell`.
    pub goal_bell: u8,
    /// The active script blob and its table blocks, copied at room entry.
    script: Vec<u8>,
    /// The `$4A81` blob: the shared `$4C4D`/`$4C51` preview tables live there
    /// for all three rooms.
    shared: Vec<u8>,
    tables: Vec<u8>,
    items: Vec<u8>,
    /// `sCastleScriptPtrLo/Hi` and its loop-back copy (bank-24 address).
    pub script_ptr: u16,
    pub script_ptr_copy: u16,
    /// The `$4A1A`/`$4A2F`/`$4A58` slot tables.
    bell_tables: [Vec<u8>; 3],
    /// `sMapPatchList`/`sMapPatch1Id` for the five `$0D` slots.
    pub patch_state: [u8; 5],
    pub patch_id: [u8; 5],
}

impl CastleRoom {
    /// A coin room as `$3F35A` stages it: Mario's OAM entry at `(156, 40)`.
    pub fn coin(scripts: &CastleScripts) -> Self {
        Self {
            kind: RoomKind::CoinRooms,
            state: 0,
            timer: 0,
            walk_idx: 0,
            reveal_progress: 0,
            reveal_tick: 0,
            item_index: 0,
            item_preview: 0,
            saved_powerup: 0,
            item_reached: 0,
            marker_col: 0,
            shuffle_phase: 0,
            walk_phase: 0,
            anim_count: 0,
            anim_step: 0,
            anim_accum: 0,
            powerup: 0,
            sprites: [0; SPRITE_BYTES],
            patches: [0; 48],
            slots: [RoomSlot::default(); 5],
            slot_index: 0,
            oam: [156, 40, 0, 0],
            sfx: [0; 2],
            music: 0,
            goal_bell: 0,
            script: scripts.blob("coin_room_script").unwrap_or(&[]).to_vec(),
            shared: scripts.blob("coin_room_script").unwrap_or(&[]).to_vec(),
            tables: scripts.blob("coin_room_tables").unwrap_or(&[]).to_vec(),
            items: scripts.blob("coin_room_items").unwrap_or(&[]).to_vec(),
            script_ptr: 0x4A1A,
            script_ptr_copy: 0x4A1A,
            bell_tables: [
                scripts.blob("bell_slots_0").unwrap_or(&[]).to_vec(),
                scripts.blob("bell_slots_1").unwrap_or(&[]).to_vec(),
                scripts.blob("bell_slots_2").unwrap_or(&[]).to_vec(),
            ],
            patch_state: [0; 5],
            patch_id: [0; 5],
        }
    }

    /// A bell room as `$3F4B1`/`$3F5A4` stage it.
    pub fn bell_a(scripts: &CastleScripts) -> Self {
        let mut room = Self::coin(scripts);
        room.kind = RoomKind::BellRoomA;
        room.script = scripts.blob("bell_room_a_script").unwrap_or(&[]).to_vec();
        room
    }

    /// The second bell room (`$4028`, screen 14).
    pub fn bell_b(scripts: &CastleScripts) -> Self {
        let mut room = Self::coin(scripts);
        room.kind = RoomKind::BellRoomB;
        room.script = scripts.blob("bell_room_b_script").unwrap_or(&[]).to_vec();
        room
    }

    /// The `$4A1A`/`$4A2F`/`$4A58` byte at a bank-24 address.
    fn table_byte(&self, addr: u16) -> u8 {
        for (index, base) in [0x4A1Au16, 0x4A2F, 0x4A58].into_iter().enumerate() {
            if addr >= base {
                if let Some(byte) = self.bell_tables[index].get((addr - base) as usize) {
                    return *byte;
                }
            }
        }
        0
    }

    /// One frame of the screen's bank-24 handler. `scripts` supplies the
    /// extracted tables (`coin_room_script`/`coin_room_tables`/`coin_room_items`).
    pub fn tick(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        self.powerup = rewards.powerup;
        match self.kind {
            RoomKind::CoinRooms => self.coin_tick(input, rewards),
            RoomKind::BellRoomA => self.bell_a_tick(input, rewards),
            RoomKind::BellRoomB => {}
        }
    }

    /// `sMapSpriteN*` as an absolute address in the 48-byte table.
    pub fn sprite(&self, addr: u16) -> u8 {
        self.sprites
            .get(addr.wrapping_sub(SPRITES_AT) as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn set_sprite(&mut self, addr: u16, value: u8) {
        if let Some(slot) = self.sprites.get_mut(addr.wrapping_sub(SPRITES_AT) as usize) {
            *slot = value;
        }
    }

    fn coin_tick(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if self.script.is_empty() || self.tables.is_empty() || self.items.is_empty() {
            return;
        }
        let script = std::mem::take(&mut self.script);
        let tables = std::mem::take(&mut self.tables);
        let items = std::mem::take(&mut self.items);
        {
            let (script, tables, items) = (&script[..], &tables[..], &items[..]);
            match self.state {
                0 => {
                    // `$4A81`: the room waits with the idle animation until
                    // the intro page counter reaches 7 and A is pressed.
                    if self.sprite(SPRITE2_PAGE) == 7 && input.keys_pressed & 0x01 != 0 {
                        self.state = 5;
                        self.walk_idx = 0;
                        self.sfx[0] = 1;
                        self.coin_state5(input, rewards, script, tables, items);
                    } else {
                        self.coin_idle_anim(input, script, tables);
                    }
                }
                1 => self.coin_state1(input, rewards, script, tables, items),
                2 => self.coin_state2(input, rewards, script, tables),
                4 => self.coin_state4(input, script, tables),
                5 => self.coin_state5(input, rewards, script, tables, items),
                6 => self.coin_state6(input, script, tables),
                _ => self.coin_commit(input, tables),
            }
        }
        self.script = script;
        self.tables = tables;
        self.items = items;
    }

    /// `$60AA7`: the page oscillation, the intro page step and the reveal
    /// countdown shared by states 0/1/5.
    fn coin_idle_anim(&mut self, input: RoomInput, script: &[u8], tables: &[u8]) {
        let mut page = self.sprite(SPRITE1_PAGE);
        self.anim_count = self.anim_count.wrapping_add(1);
        if self.anim_count < 0x28 {
            page = page.wrapping_add(1);
        } else if self.anim_count >= 0x50 {
            self.anim_count = 0;
            page = page.wrapping_add(1);
        } else {
            page = page.wrapping_sub(1);
        }
        self.set_sprite(SPRITE1_PAGE, page);
        if input.frame & 0x07 == 0 && self.sprite(SPRITE2_PAGE) != 7 {
            let p = self.sprite(SPRITE2_PAGE).wrapping_add(1);
            self.set_sprite(SPRITE2_PAGE, p);
        }
        self.reveal_progress = self.reveal_progress.wrapping_sub(2);
        self.reveal_tick = self.reveal_tick.wrapping_add(1);
        self.coin_commit(input, tables);
        let _ = script;
    }

    /// `$60D08`: step Mario's OAM x by the `$4FA8` walk table and set the
    /// preview tile. Returns without moving at the `$80` terminator.
    fn coin_walk_anim(&mut self, script: &[u8]) {
        let preview = self.preview_tile(script);
        self.oam[2] = preview;
        let Some(&delta) = script.get(TABLE_WALK_OFFSETS + self.walk_idx as usize) else {
            return;
        };
        if delta == 0x80 {
            return;
        }
        self.walk_idx = self.walk_idx.wrapping_add(1);
        self.oam[0] = self.oam[0].wrapping_add(delta);
        self.oam[2] = self.preview_tile(script).wrapping_add(8);
    }

    /// `$60C41`: `$4C4D[sCurPowerup]` — the preview tile for the current form.
    fn preview_tile(&self, script: &[u8]) -> u8 {
        script
            .get(POWERUP_PREVIEW_AT + self.powerup as usize)
            .copied()
            .unwrap_or(0)
    }

    /// `$60D32`: decode the reveal progress into the item table column.
    fn coin_reveal_decode(&mut self, input: RoomInput, tables: &[u8], items: &[u8]) {
        let mut index = (self.reveal_progress & 0x7C) >> 2;
        if index >= 0x1E {
            self.reveal_progress &= 0x77;
            index = 29;
        }
        let column = self.sprite(SPRITE1_PAGE).wrapping_sub(40) >> 3 & 0x0F;
        let column = tables
            .get(TABLE_ITEM_COLUMNS + (column.wrapping_add(index)) as usize)
            .copied()
            .unwrap_or(0);
        let node = input.node.wrapping_sub(110).wrapping_mul(2);
        let row_ptr = tables
            .get(TABLE_NODE_ITEM_PTRS + node as usize)
            .copied()
            .unwrap_or(0) as u16
            | ((tables
                .get(TABLE_NODE_ITEM_PTRS + node as usize + 1)
                .copied()
                .unwrap_or(0) as u16)
                << 8);
        let offset = row_ptr.wrapping_sub(0x4000).wrapping_add(column as u16) as usize;
        self.item_index = items.get(offset).copied().unwrap_or(0);
        if self.item_index != 0 {
            self.music = 22;
        }
    }

    /// `$60D80`: the item marker byte and its pickup sfx.
    fn coin_marker(&mut self) {
        if self.item_index != 0 {
            return;
        }
        self.marker_col = self.sprite(SPRITE1_PAGE).wrapping_sub(48) >> 3 & 0x0F;
        let mut column = self.sprite(SPRITE1_PAGE).wrapping_sub(40) >> 3 & 0x0F;
        column = column.wrapping_add(self.reveal_progress >> 2);
        while column >= 3 {
            column -= 3;
        }
        let delta = 2u8.wrapping_sub(column);
        self.marker_col = (self.marker_col.wrapping_add(delta)) | 0x80;
        self.sfx[0] = 17;
    }

    /// `$60DC7`: apply the item reward, the sfx and the coin-purse clamp.
    fn coin_award(&mut self, rewards: &mut RoomRewards, tables: &[u8]) {
        self.saved_powerup = rewards.powerup;
        self.item_preview = rewards.powerup;
        if self.item_index == 0 {
            return;
        }
        match self.item_index {
            1 => rewards.powerup = 1,
            2 => rewards.powerup = 2,
            3 => rewards.powerup = 3,
            4 => self.add_lives(rewards, 1),
            5 => self.add_lives(rewards, 3),
            6 => self.add_lives(rewards, 5),
            7 => self.add_lives(rewards, 80),
            8 => rewards.lives = 153,
            9 => rewards.coins_high = bcd_add(rewards.coins_high, 3).0,
            10 => rewards.coins_high = bcd_add(rewards.coins_high, 5).0,
            11 => self.add_lives(rewards, 32),
            _ => return,
        }
        self.item_preview = rewards.powerup;
        let sfx = tables
            .get(TABLE_ITEM_SFX + self.item_index as usize)
            .copied()
            .unwrap_or(0);
        if sfx == 0x14 {
            self.sfx[1] = sfx;
        } else {
            self.sfx[0] = sfx;
        }
        if rewards.coins_high & 0xF0 != 0 {
            rewards.coins_low = 153;
            rewards.coins_high = 9;
        }
        rewards.save = true;
    }

    fn add_lives(&mut self, rewards: &mut RoomRewards, amount: u8) {
        let (value, carry) = bcd_add(rewards.lives, amount);
        rewards.lives = if carry { 153 } else { value };
    }

    /// `$60CF9`: the per-frame commit. The VRAM tile copies (`$60EE6`,
    /// `$6106D`) and the bank-1 call (`$3F44`) are render-side; the RAM part is
    /// the sprite-3 frame byte from `$4F5A`.
    fn coin_commit(&mut self, input: RoomInput, tables: &[u8]) {
        let phase = ((self.reveal_tick & 0x0C) >> 2) as usize;
        let base = match input.node {
            0x6E => TABLE_SPRITE3_PHASE[0],
            0x6F => TABLE_SPRITE3_PHASE[1],
            0x70 => TABLE_SPRITE3_PHASE[2],
            _ => TABLE_SPRITE3_PHASE[3],
        };
        self.set_sprite(SPRITE3_ID, tables.get(base + phase).copied().unwrap_or(0));
    }

    /// `$60C59`: state 5 walks Mario in, then advances the intro page.
    fn coin_state5(
        &mut self,
        input: RoomInput,
        rewards: &mut RoomRewards,
        script: &[u8],
        tables: &[u8],
        items: &[u8],
    ) {
        self.coin_walk_anim(script);
        if self.oam[0] >= 0x6C || input.frame & 0x01 != 0 {
            self.coin_idle_anim(input, script, tables);
            return;
        }
        let id = self.sprite(SPRITE2_ID);
        if id >= 0x0A {
            self.state = 1;
            self.timer = 150;
            self.music = 8;
            self.coin_state1(input, rewards, script, tables, items);
        } else {
            self.set_sprite(SPRITE2_ID, id.wrapping_add(1));
            self.coin_commit(input, tables);
        }
    }

    /// `$60AEC`: the state-1 countdown and reveal.
    fn coin_state1(
        &mut self,
        input: RoomInput,
        rewards: &mut RoomRewards,
        script: &[u8],
        tables: &[u8],
        items: &[u8],
    ) {
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.state = 2;
            self.timer = 90;
            self.set_sprite(SPRITE1_X, 2);
            self.coin_reveal_decode(input, tables, items);
            self.set_sprite(SPRITE3_X, 56);
            self.coin_state2(input, rewards, script, tables);
            return;
        }
        self.coin_walk_anim(script);
        if self.timer < 0x78 {
            if self.timer & 0x01 != 0 {
                self.anim_step = self.anim_step.wrapping_add(1);
                if self.anim_step < 3 {
                    self.set_sprite(SPRITE1_X, self.anim_step.wrapping_add(2));
                } else {
                    self.anim_step = 3;
                    self.set_sprite(SPRITE1_X, 2);
                }
            }
            let amount = tables
                .get(TABLE_REVEAL_STEPS + self.timer as usize)
                .copied()
                .unwrap_or(0);
            if amount == 0 {
                self.coin_commit(input, tables);
                return;
            }
            self.reveal_progress = self.reveal_progress.wrapping_sub(amount);
            self.reveal_tick = self.reveal_tick.wrapping_add(1);
            self.anim_accum = self.anim_accum.wrapping_add(amount);
            if self.anim_accum < 3 {
                self.coin_commit(input, tables);
                return;
            }
            self.anim_accum -= 3;
            self.anim_step = 0;
            self.set_sprite(SPRITE1_X, 2);
        } else {
            let mut page = self.sprite(SPRITE1_PAGE);
            if page & 0x07 != 5 {
                self.anim_count = self.anim_count.wrapping_add(1);
                if self.anim_count < 0x28 {
                    page = page.wrapping_add(1);
                } else if self.anim_count >= 0x50 {
                    self.anim_count = 0;
                    page = page.wrapping_add(1);
                } else {
                    page = page.wrapping_sub(1);
                }
                self.set_sprite(SPRITE1_PAGE, page);
            }
            self.reveal_progress = self.reveal_progress.wrapping_sub(2);
            self.reveal_tick = self.reveal_tick.wrapping_add(1);
        }
        self.coin_commit(input, tables);
    }

    /// `$60BBF`: the state-2 item slide and the award trigger.
    fn coin_state2(
        &mut self,
        input: RoomInput,
        rewards: &mut RoomRewards,
        script: &[u8],
        tables: &[u8],
    ) {
        if input.frame & 0x01 != 0 {
            self.coin_commit(input, tables);
            return;
        }
        self.set_sprite(SPRITE3_Y, 255);
        if self.timer < 0x3C {
            self.set_sprite(SPRITE3_X, self.sprite(SPRITE3_X).wrapping_add(1));
            if self.sprite(SPRITE3_X) == 0x4E {
                self.coin_marker();
            }
            self.set_sprite(SPRITE3_PAGE, 32);
            self.set_sprite(SPRITE3_Y, self.item_index.wrapping_add(11));
        }
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            self.coin_commit(input, tables);
            return;
        }
        self.timer = 100;
        self.state = 6;
        self.set_sprite(SPRITE3_Y, 255);
        self.coin_award(rewards, tables);
        self.coin_commit(input, tables);
        let _ = script;
    }

    /// `$60C13`: state 4 slides the preview across the screen and back to 0.
    fn coin_state4(&mut self, input: RoomInput, script: &[u8], tables: &[u8]) {
        self.oam[2] = self.preview_tile(script).wrapping_add(16);
        self.oam[0] = self.oam[0].wrapping_sub(1);
        if self.oam[0] < 0x7D {
            self.state = 0;
            self.oam[2] = self.preview_tile(script);
            self.oam[3] = 1;
        }
        self.coin_commit(input, tables);
    }

    /// `$60C94`: state 6 flickers the item preview and ends the room.
    fn coin_state6(&mut self, input: RoomInput, script: &[u8], tables: &[u8]) {
        let flicker = input.frame & 0x0C;
        let preview =
            if self.timer >= 0x19 && self.item_index != 0 && self.item_index < 4 && flicker != 0 {
                match flicker {
                    0x04 => 0,
                    0x08 if self.saved_powerup != 0 => self.saved_powerup,
                    0x08 => self.item_preview,
                    _ => 0,
                }
            } else {
                self.item_preview
            };
        self.oam[2] = script
            .get(POWERUP_PREVIEW_AT + preview as usize)
            .copied()
            .unwrap_or(0);
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 {
            self.state = 3;
        }
        self.coin_commit(input, tables);
    }

    // ---- `$4512`: bell room A (screen 13) ---------------------------------

    fn bell_a_tick(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if self.script.is_empty() {
            return;
        }
        match self.state {
            0 => self.bell_a_state0(input, rewards),
            1 => self.bell_a_state1(input, rewards),
            2 => self.bell_a_state2(input, rewards),
            3 => self.bell_a_state3(input, rewards),
            4 => self.bell_a_state4(input, rewards),
            5 => self.bell_a_state5(input, rewards),
            6 => self.bell_a_state6(input, rewards),
            7 => self.bell_a_state7(input, rewards),
            8 => self.bell_a_state8(input, rewards),
            9 => self.bell_a_state9(input, rewards),
            _ => self.bell_a_commit(input, rewards),
        }
    }

    /// `$4539`: walk Mario up to `$20` and stage the preview tile.
    fn bell_a_state0(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x01 != 0 {
            self.bell_a_commit(input, rewards);
            return;
        }
        if self.oam[1] == 0x20 {
            self.state = 1;
            self.oam[2] = self.bell_preview();
            self.item_reached = 0;
            self.bell_a_state1(input, rewards);
            return;
        }
        self.oam[1] = self.oam[1].wrapping_add(1);
        if input.frame & 0x02 != 0 {
            self.walk_phase += 1;
            if self.walk_phase >= 3 {
                self.walk_phase = 0;
            }
        }
        self.oam[2] = self
            .bell_preview()
            .wrapping_add(self.walk_phase)
            .wrapping_add(1);
        self.bell_a_commit(input, rewards);
    }

    /// `$60582`: A starts the countdown; otherwise `$60800`'s walk cycle.
    fn bell_a_state1(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.keys_pressed & 0x01 != 0 {
            self.state = 2;
            self.timer = 24;
            self.music = 8;
            self.bell_a_state2(input, rewards);
            return;
        }
        self.bell_a_walk_cycle(input.frame);
        self.bell_a_commit(input, rewards);
    }

    /// `$60800`: the shared walk-cycle byte, complemented timer as direction.
    fn bell_a_walk_cycle(&mut self, frame: u8) {
        if frame & 0x03 != 0 {
            return;
        }
        let mut e = self.sprite(0xA6B8);
        if self.timer & 0x01 != 0 {
            if e < 0x50 {
                self.timer = !self.timer;
            } else {
                e = e.wrapping_sub(1);
            }
        } else if e >= 0x78 {
            self.timer = !self.timer;
        } else {
            e = e.wrapping_add(1);
        }
        for addr in [0xA6B8, 0xA6BB, 0xA6BE, 0xA6C1, 0xA6C4, 0xA6C7] {
            self.set_sprite(addr, e);
        }
    }

    /// `$605A0`: the 24-frame countdown, one sprite-1 id step every four.
    fn bell_a_state2(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x03 != 0 {
            self.bell_a_commit(input, rewards);
            return;
        }
        if self.timer == 0 {
            self.state = 3;
            self.timer = 0;
            self.bell_a_state3(input, rewards);
            return;
        }
        self.timer = self.timer.wrapping_sub(1);
        self.set_sprite(0xA6BC, self.sprite(0xA6BC).wrapping_add(1));
        self.bell_a_commit(input, rewards);
    }

    /// `$605CA`: step sprite 2's page to `$23`.
    fn bell_a_state3(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x03 != 0 {
            self.bell_a_commit(input, rewards);
            return;
        }
        if self.sprite(0xA6BD) >= 0x23 {
            self.state = 4;
            self.timer = 32;
            self.bell_a_state4(input, rewards);
            return;
        }
        self.set_sprite(0xA6BD, self.sprite(0xA6BD).wrapping_add(1));
        self.bell_a_commit(input, rewards);
    }

    /// `$605EE`: count the timer down, step sprite 1's id back and hand over to
    /// state 9 at `$48` if no item was reached.
    fn bell_a_state4(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x03 != 0 {
            self.bell_a_commit(input, rewards);
            return;
        }
        if self.timer == 0 {
            self.state = 5;
            self.bell_a_state5(input, rewards);
            return;
        }
        self.timer = self.timer.wrapping_sub(1);
        if self.timer == 0 && self.item_reached == 1 {
            self.music = 22;
        }
        self.set_sprite(0xA6BC, self.sprite(0xA6BC).wrapping_sub(1));
        if self.sprite(0xA6BC) == 0x48 && self.item_reached != 1 {
            self.state = 9;
            self.sfx[0] = 17;
        }
        self.bell_a_commit(input, rewards);
    }

    /// `$60639`: step sprite 0's id down below `$1D`.
    fn bell_a_state5(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x01 != 0 {
            self.bell_a_commit(input, rewards);
            return;
        }
        if self.sprite(0xA6B8) < 0x1D {
            self.state = 6;
            self.bell_a_state6(input, rewards);
            return;
        }
        self.set_sprite(0xA6B8, self.sprite(0xA6B8).wrapping_sub(1));
        self.set_sprite(0xA6BB, self.sprite(0xA6B8));
        self.bell_a_commit(input, rewards);
    }

    /// `$6065B`: step sprite 2's page down to `$21`.
    fn bell_a_state6(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x03 != 0 {
            self.bell_a_commit(input, rewards);
            return;
        }
        if self.sprite(0xA6BD) == 0x21 {
            self.state = 7;
            self.timer = 25;
            self.bell_a_state7(input, rewards);
            return;
        }
        self.set_sprite(0xA6BD, self.sprite(0xA6BD).wrapping_sub(1));
        self.bell_a_commit(input, rewards);
    }

    /// `$6067F`: a 25-frame hold, then the item reveal.
    fn bell_a_state7(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x03 != 0 {
            self.bell_a_commit(input, rewards);
            return;
        }
        if self.timer != 0 {
            self.timer = self.timer.wrapping_sub(1);
        } else {
            self.state = 9;
        }
        self.bell_a_commit(input, rewards);
    }

    /// `$469C` / `$606AA`: the preview flicker, then the award and the save.
    fn bell_a_state8(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x03 == 0 {
            self.bell_a_preview(input, rewards);
        }
        self.bell_a_commit(input, rewards);
    }

    fn bell_a_preview(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        let flicker = input.frame & 0x0C;
        let preview = if self.item_index < 3 {
            if flicker == 0 {
                self.item_preview
            } else {
                match flicker {
                    0x04 => 0,
                    0x08 if self.saved_powerup != 0 => self.saved_powerup,
                    0x08 => self.item_preview,
                    _ => 0,
                }
            }
        } else {
            // `$60711`: slot 9 (or slot 11 on screen 14) carries the icon.
            let c = if self.item_index == 3 { 29u8 } else { 30u8 };
            if self.kind == RoomKind::BellRoomB {
                self.set_sprite(0xA6E4, c);
                if self.sprite(0xA6E3) >= 0x62 {
                    self.set_sprite(0xA6E3, self.sprite(0xA6E3).wrapping_sub(1));
                    self.set_sprite(0xA6E2, self.oam[1].wrapping_sub(4));
                } else {
                    self.set_sprite(0xA6E4, 255);
                    self.set_sprite(0xA6E2, 176);
                }
            } else {
                self.set_sprite(0xA6DB, c);
                if self.sprite(0xA6DA) >= 0x60 {
                    self.set_sprite(0xA6DA, self.sprite(0xA6DA).wrapping_sub(1));
                } else {
                    self.set_sprite(0xA6DB, 255);
                }
            }
            self.item_preview
        };
        self.oam[2] = self.preview_for(preview);
        self.timer = self.timer.wrapping_sub(1);
        if self.timer != 0 {
            return;
        }
        rewards.powerup = self.item_preview;
        self.powerup = self.item_preview;
        self.oam[2] = self.preview_for(self.item_preview);
        self.state = 9;
        self.timer = 20;
        rewards.save = true;
    }

    /// `$4764` / `$4772`: walk Mario back down to `$E8`.
    fn bell_a_state9(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if input.frame & 0x01 == 0 {
            self.bell_a_commit(input, rewards);
            return;
        }
        if self.oam[1] == 0xE8 {
            self.state = 10;
            return;
        }
        self.oam[1] = self.oam[1].wrapping_sub(1);
        if input.frame & 0x02 != 0 {
            self.walk_phase += 1;
            if self.walk_phase >= 3 {
                self.walk_phase = 0;
            }
        }
        self.oam[2] = self
            .bell_preview()
            .wrapping_add(self.walk_phase)
            .wrapping_add(5);
        self.bell_a_commit(input, rewards);
    }

    /// `$607AA`: the slot sprites, five slot steps and the OAM build.
    fn bell_a_commit(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        self.bell_a_slot_sprites();
        for _ in 0..5 {
            self.bell_a_slot_step(input, rewards);
        }
    }

    /// `$607C5`: reveal the slot sprites as the walk byte passes each threshold.
    fn bell_a_slot_sprites(&mut self) {
        for addr in [0xA6C0, 0xA6C3, 0xA6C6, 0xA6C9] {
            self.set_sprite(addr, 255);
        }
        let walk = self.sprite(0xA6BC);
        for (threshold, addr) in [
            (0x3Eu8, 0xA6C0u16),
            (0x46, 0xA6C3),
            (0x4E, 0xA6C6),
            (0x56, 0xA6C9),
        ] {
            if walk < threshold {
                return;
            }
            self.set_sprite(addr, 32);
        }
    }

    /// `$6083C`: one of the five `$0D` slots.
    fn bell_a_slot_step(&mut self, input: RoomInput, rewards: &mut RoomRewards) {
        if self.slot_index >= 5 {
            self.slot_index = 0;
        }
        let s = self.slot_index as usize;
        match self.patch_state[s] {
            0 => {
                if input.frame & 0x01 != 0 {
                    self.bell_a_slot_commit();
                    return;
                }
                self.patch_id[s] = self.patch_id[s].wrapping_sub(1);
                if self.patch_id[s] < 0x32 {
                    self.patch_id[s] = 142;
                    self.patch_state[s] = 3;
                    self.slots[s].delay = 28;
                    self.bell_a_slot_patch(s, input);
                    return;
                }
                if self.state == 3 && self.slots[s].value < 5 {
                    let y = self.sprite(0xA6BB).wrapping_sub(12);
                    if y < self.patch_id[s] && y.wrapping_add(16) >= self.patch_id[s] {
                        self.patch_state[s] = 1;
                        self.item_reached = 1;
                    }
                }
                self.bell_a_slot_commit();
            }
            1 => {
                self.patch_id[s] = self.sprite(0xA6BB).wrapping_sub(4);
                self.slots[s].counter = self.sprite(0xA6BC).wrapping_add(8);
                if self.state == 7 {
                    self.patch_state[s] = 2;
                }
                self.bell_a_slot_commit();
            }
            2 => {
                self.slots[s].counter = self.slots[s].counter.wrapping_add(1);
                if self.slots[s].counter >= 0x68 {
                    self.patch_state[s] = 3;
                    self.slots[s].delay = 255;
                    self.state = 8;
                    self.timer = 20;
                    self.item_index = self.sprite(0xA6CCu16 + 3 * s as u16).wrapping_sub(24);
                    self.bell_a_award_preview(rewards);
                }
                self.bell_a_slot_commit();
            }
            3 => self.bell_a_slot_patch(s, input),
            _ => self.bell_a_slot_commit(),
        }
    }

    /// `$60965`: a slot's patch entry, gated on the `$4A1A`-family table.
    fn bell_a_slot_patch(&mut self, s: usize, input: RoomInput) {
        self.patch_id[s] = 142;
        self.slots[s].counter = 96;
        let mut byte = self.table_byte(self.script_ptr);
        if byte >= 6 {
            self.script_ptr = self.script_ptr_copy;
            byte = self.table_byte(self.script_ptr);
        }
        self.slots[s].value = byte;
        let sprite = 0xA6CCu16 + 3 * s as u16;
        self.set_sprite(sprite, 255);
        if input.frame & 0x01 != 0 {
            self.bell_a_slot_commit();
            return;
        }
        let delay = self.slots[s].delay;
        if delay == 255 {
            self.bell_a_slot_commit();
            return;
        }
        if delay != 0 {
            self.slots[s].delay = delay.wrapping_sub(1);
            self.bell_a_slot_commit();
            return;
        }
        if self.slots[s].value != 5 {
            self.set_sprite(sprite, self.slots[s].value.wrapping_add(24));
        }
        self.patch_state[s] = 0;
        self.script_ptr = self.script_ptr.wrapping_add(1);
        self.bell_a_slot_commit();
    }

    /// `$609E4`: copy the five slot bytes into sprites 5-8 and rotate.
    fn bell_a_slot_commit(&mut self) {
        for (s, addr) in [0xA6CAu16, 0xA6CD, 0xA6D0, 0xA6D3, 0xA6D6]
            .into_iter()
            .enumerate()
        {
            self.set_sprite(addr, self.patch_id[s]);
        }
        for (s, addr) in [0xA6CBu16, 0xA6CE, 0xA6D1, 0xA6D4, 0xA6D7]
            .into_iter()
            .enumerate()
        {
            self.set_sprite(addr, self.slots[s].counter);
        }
        self.slot_index = self.slot_index.wrapping_add(1);
    }

    /// `$60917`: the slot award, before the room's own preview plays.
    fn bell_a_award_preview(&mut self, rewards: &mut RoomRewards) {
        self.saved_powerup = rewards.powerup;
        self.item_preview = rewards.powerup;
        if self.item_index < 3 {
            self.item_preview = self
                .shared
                .get(ITEM_PREVIEW_AT + self.item_index as usize)
                .copied()
                .unwrap_or(0);
            self.sfx[1] = 20;
            return;
        }
        self.sfx[0] = 9;
        let amount = if self.item_index == 3 { 1 } else { 3 };
        let (value, carry) = bcd_add(rewards.lives, amount);
        rewards.lives = if carry { 153 } else { value };
    }

    fn bell_preview(&self) -> u8 {
        self.shared
            .get(POWERUP_PREVIEW_AT + self.powerup as usize)
            .copied()
            .unwrap_or(0)
    }

    fn preview_for(&self, index: u8) -> u8 {
        self.shared
            .get(POWERUP_PREVIEW_AT + index as usize)
            .copied()
            .unwrap_or(0)
    }
}

/// `add`/`daa`: BCD byte add with the carry the ROM's `daa` produces.
fn bcd_add(value: u8, add: u8) -> (u8, bool) {
    let mut lo = (value & 0x0F) + (add & 0x0F);
    let mut hi = (value >> 4) + (add >> 4);
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

    fn scripts() -> CastleScripts {
        CastleScripts::from_parts(
            include_bytes!("../../../extracted/map/castle_scripts.bin").to_vec(),
            include_str!("../../../extracted/map/castle_scripts.json"),
        )
        .expect("castle scripts asset")
    }

    fn input(frame: u8, keys: u8) -> RoomInput {
        RoomInput {
            keys_pressed: keys,
            frame,
            node: 0x70,
        }
    }

    /// `$4C4D`: the twelve preview tiles, pinned from the ROM.
    #[test]
    fn the_powerup_preview_table_is_the_rom_bytes() {
        let scripts = scripts();
        let table = &scripts.blob("coin_room_script").unwrap()
            [POWERUP_PREVIEW_AT..POWERUP_PREVIEW_AT + POWERUP_PREVIEW_LEN];
        assert_eq!(
            table,
            &[0x20, 0x00, 0x45, 0x70, 0x01, 0x03, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    /// `$4A81`: state 0 needs sprite 2 page 7 and A; the press starts the walk
    /// and applies the walk table's first offset.
    #[test]
    fn state_zero_gates_on_the_intro_page_and_a() {
        let scripts = scripts();
        let mut room = CastleRoom::coin(&scripts);
        let mut rewards = RoomRewards::default();
        room.set_sprite(SPRITE2_PAGE, 7);
        room.tick(input(0, 0x00), &mut rewards);
        assert_eq!(room.state, 0, "no press, no room");

        room.tick(input(0, 0x01), &mut rewards);
        assert_eq!(room.state, 5, "$4A81 sets state 5");
        assert_eq!(room.sfx[0], 1);
        assert_eq!(room.walk_idx, 1);
        assert_ne!(room.oam[0], 156, "Mario stepped toward the item");
    }

    /// `$60BBF`: the state-2 timer expiring awards the item, holds state 6 for
    /// 100 frames and then reports state 3 (`$3F4A0`'s return condition).
    #[test]
    fn the_coin_room_awards_and_returns() {
        let scripts = scripts();
        let mut room = CastleRoom::coin(&scripts);
        let mut rewards = RoomRewards::default();
        room.item_index = 1;
        room.state = 2;
        room.timer = 1;
        room.tick(input(0, 0), &mut rewards);
        assert_eq!(room.state, 6, "$60BF3 -> state 6");
        assert_eq!(room.timer, 100, "the reveal holds for 100 frames");
        assert_eq!(rewards.powerup, 1, "item 1 is the mushroom");
        assert_eq!(room.saved_powerup, 0, "saved before the award");
        assert_eq!(room.item_preview, 1);
        assert!(rewards.save, "$2934 is asked for a save");
        assert_eq!(room.sfx[1], 0x14, "$4F6A[1] is the second-slot sfx");

        let mut guard = 0;
        while room.state != 3 && guard < 200 {
            room.tick(input(guard as u8, 0), &mut rewards);
            guard += 1;
        }
        assert_eq!(room.state, 3);
        assert_eq!(guard, 100);
    }

    /// `$4FCD` is the per-frame reveal amount; index 0 and 89 are both 1, and
    /// `$4FA8` starts with the four `$FC` steps.
    #[test]
    fn the_reveal_and_walk_tables_are_the_rom_bytes() {
        let scripts = scripts();
        let table = scripts.blob("coin_room_tables").unwrap();
        assert_eq!(table[TABLE_REVEAL_STEPS], 1);
        assert_eq!(table[TABLE_REVEAL_STEPS + 89], 1);
        assert_eq!(
            &table[TABLE_WALK_OFFSETS..TABLE_WALK_OFFSETS + 4],
            &[0xFC; 4]
        );
    }

    /// `$60582`: A starts bell room A's countdown.
    #[test]
    fn bell_room_a_starts_its_countdown_on_a() {
        let scripts = scripts();
        let mut room = CastleRoom::bell_a(&scripts);
        let mut rewards = RoomRewards::default();
        room.state = 1;
        room.tick(input(0, 0x01), &mut rewards);
        assert_eq!(room.state, 2);
        assert_eq!(room.timer, 23, "state 2 decrements in the same frame");
        assert_eq!(room.music, 8);
        assert_eq!(room.sfx[0], 0, "no sfx on the start");
    }

    /// `$6065B`: the page walk reaches `$21` and hands over to the 25-frame
    /// hold in state 7.
    #[test]
    fn bell_room_a_page_walk_reaches_state_seven() {
        let scripts = scripts();
        let mut room = CastleRoom::bell_a(&scripts);
        let mut rewards = RoomRewards::default();
        room.state = 6;
        room.set_sprite(0xA6BD, 0x21);
        room.tick(input(0, 0), &mut rewards);
        assert_eq!(room.state, 7);
        assert_eq!(room.timer, 24, "state 7 decrements in the same frame");
    }

    /// `$60965`: a patch slot in state 3 with the delay expired writes the
    /// slot value (`$4A1A[0] = 3`) into sprite 5's id and advances the pointer.
    #[test]
    fn bell_room_a_slot_patch_advances_the_script() {
        let scripts = scripts();
        let mut room = CastleRoom::bell_a(&scripts);
        let mut rewards = RoomRewards::default();
        room.state = 3;
        room.patch_state[0] = 3;
        room.slots[0].delay = 0;
        room.slot_index = 0;
        room.script_ptr = 0x4A1A;
        room.tick(input(0, 0), &mut rewards);
        assert_eq!(room.slots[0].value, 3, "$4A1A[0]");
        assert_eq!(room.sprite(0xA6CC), 3 + 24, "sprite 5 id");
        assert_eq!(room.patch_state[0], 0);
        assert_eq!(room.script_ptr, 0x4A1B);
    }

    /// `$60917`: the slot award maps item 0-2 through `$4C51` and adds BCD
    /// lives for the higher ids.
    #[test]
    fn bell_room_a_awards_preview_and_lives() {
        let scripts = scripts();
        let mut room = CastleRoom::bell_a(&scripts);
        let mut rewards = RoomRewards {
            lives: 0x05,
            ..Default::default()
        };
        room.item_index = 1;
        room.state = 3;
        room.patch_state[0] = 2;
        room.slot_index = 0;
        room.slots[0].counter = 0x67;
        room.set_sprite(0xA6CC, 25); // item index 1
        room.tick(input(0, 0), &mut rewards);
        assert_eq!(room.state, 8);
        assert_eq!(room.timer, 20);
        assert_eq!(room.item_index, 1);
        assert_eq!(room.item_preview, 3, "$4C51[1]");
        assert_eq!(room.sfx[1], 20);

        let mut room = CastleRoom::bell_a(&scripts);
        let mut rewards = RoomRewards {
            lives: 0x05,
            ..Default::default()
        };
        room.item_index = 3;
        room.state = 3;
        room.patch_state[0] = 2;
        room.slot_index = 0;
        room.slots[0].counter = 0x67;
        room.set_sprite(0xA6CC, 27);
        room.tick(input(0, 0), &mut rewards);
        assert_eq!(room.sfx[0], 9);
        assert_eq!(rewards.lives, 0x06, "item 3 is +1 life, BCD");
    }

    /// `$60D32` reads the node's ten-byte row in the `$4000` item table.
    #[test]
    fn the_item_table_rows_are_the_rom_bytes() {
        let scripts = scripts();
        let items = scripts.blob("coin_room_items").unwrap();
        assert_eq!(&items[..10], &[2, 3, 1, 2, 3, 1, 2, 3, 1, 0]);
        assert_eq!(&items[20..30], &[6, 0, 11, 4, 5, 4, 6, 5, 4, 0]);
    }
}
