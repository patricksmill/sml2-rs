//! Host-facing debug controls: overworld warps, level warps, progress, purse
//! and power-up edits.
//!
//! Every function writes the fields the game's own handlers write — the map's
//! `sLevelProgress`, the player's copy inside a level, the BCD purse, the BCD
//! life counter — so a debugged session keeps running through the normal flows
//! (level entry, hand-off, save) instead of holding shadow state the play loop
//! would overwrite on the next transition.
//!
//! The state that has to move together is the reason these live in the runtime:
//! `sLevelProgress` is the map's array while the overworld is up, `PlayerSim`'s
//! copy while a level runs, and `Session`'s stash during the castle reveal.
//! Editing one copy only would desync the others on the next hand-off.

use super::{Session, SessionState};
use sml2_core::castle_sequence::GOLDEN_COIN_OFFSETS;
use sml2_core::map::progress_offset as po;
use sml2_core::map::MapNodes;
use sml2_core::map_sim::MapSim;

/// Highest `sMapScreenId` a warp accepts: screen 8 is the castle. Screens 9-14
/// are the bank-24 room screens, which need a script staged rather than a
/// cursor landing, so they are not warp targets.
pub const WARP_MAX_SCREEN: u8 = 8;

/// `sPendingItemId` values the `$3451` consumer turns into power-ups.
const ITEM_MUSHROOM: u8 = 1;
const ITEM_FLOWER: u8 = 2;
const ITEM_CARROT: u8 = 3;
const ITEM_STAR: u8 = 4;

/// The hub door (`$3C60B`) that loads each zone screen 1-8. Where a zone has
/// two, this is the one whose landing the debug panel names.
pub const ZONE_DOORS: [(u8, u8); 8] = [
    (1, 0x09),
    (2, 0x19),
    (3, 0x1B),
    (4, 0x12),
    (5, 0x67),
    (6, 0x1D),
    (7, 0x11),
    (8, 0x15),
];

impl Session {
    /// `sLevelProgress` as the live state holds it.
    ///
    /// The front end has no progress of its own, so it reports the stash the
    /// castle reveal and the save writer use.
    pub fn debug_progress(&self) -> [u8; 42] {
        match &self.state {
            SessionState::Overworld(map) => map.level_progress,
            SessionState::InLevel(ctx) => ctx.sim.level_progress,
            _ => self.castle_progress,
        }
    }

    /// Write a whole progress image into every live copy.
    ///
    /// `sLevelTally` moves with it, by the change in cleared stages: the game
    /// counts a stage on its first clear (`$3547`, `$2426`), so an edit that
    /// clears or un-clears stages must count them too or the file select and
    /// the launcher report a full save as zero stages.
    pub fn debug_set_progress(&mut self, progress: [u8; 42]) {
        let cleared = |image: &[u8; 42]| {
            self.debug_level_slots()
                .into_iter()
                .filter(|&slot| image.get(usize::from(slot)).is_some_and(|b| b & 0x80 != 0))
                .count() as i32
        };
        let delta = cleared(&progress) - cleared(&self.debug_progress());
        if delta != 0 {
            let tally = i32::from(bcd_to_u8(self.debug_tally()));
            self.debug_set_tally(tally.saturating_add(delta).clamp(0, 99) as u8);
        }
        self.castle_progress = progress;
        match &mut self.state {
            SessionState::Overworld(map) => {
                map.level_progress = progress;
                // The hub's six coin icons are staged from the flags at screen
                // load (`$3CE43`), so a live edit has to restage them or the
                // map keeps showing the old set until the next load.
                if map.screen_id == 0 {
                    map.seed_hub_coin_icons();
                }
            }
            SessionState::InLevel(ctx) => ctx.sim.level_progress = progress,
            SessionState::Castle(seq) => {
                for (index, off) in GOLDEN_COIN_OFFSETS.into_iter().enumerate() {
                    seq.coins[index] = progress[off];
                }
                seq.golden_count =
                    sml2_core::castle_sequence::CastleSequence::count_coins(&seq.coins);
            }
            _ => {}
        }
    }

    /// The `sLevelProgress` offsets that belong to a stage: every slot some
    /// node's `$650E` entry points at (31 of the ROM's 32 levels; level 24 has
    /// no node).
    ///
    /// These bytes are shared with the map's own state: bit 7 is the stage's
    /// clear flag — which is exactly what the map's gate tests read, so
    /// clearing stage 30 opens the Macro gate, 26 the Mario gate, and so on —
    /// while bit 0 is a secret-path flag. Clearing every stage therefore opens
    /// every gate, the same state a full play-through reaches.
    pub fn debug_level_slots(&self) -> Vec<u8> {
        (0..MapNodes::NODES as u8)
            .filter_map(|node| {
                let level = self.assets.nodes.level_byte(node)?;
                (level < 0xFE)
                    .then(|| self.assets.nodes.progress_slot(node))
                    .flatten()
            })
            .fold(Vec::new(), |mut slots, slot| {
                if !slots.contains(&slot) {
                    slots.push(slot);
                }
                slots
            })
    }

    /// The progress slot a level's stages write: the first node that reaches
    /// `level_id`, or `None` for a level no node points at.
    pub fn debug_level_slot(&self, level_id: u8) -> Option<u8> {
        (0..MapNodes::NODES as u8).find_map(|node| {
            (self.assets.nodes.level_byte(node) == Some(level_id))
                .then(|| self.assets.nodes.progress_slot(node))
                .flatten()
        })
    }

    /// Mark every stage cleared (`sLevelProgress[slot] |= $80`). Returns how
    /// many slots were set.
    ///
    /// The six golden-coin bytes are boss stages' clear bits, so this collects
    /// all six coins too — which is what the game's own boss clears do.
    pub fn debug_clear_all_levels(&mut self) -> usize {
        let slots = self.debug_level_slots();
        let mut progress = self.debug_progress();
        for &slot in &slots {
            if let Some(byte) = progress.get_mut(slot as usize) {
                *byte |= 0x80;
            }
        }
        self.debug_set_progress(progress);
        slots.len()
    }

    /// Set the six secret-path bits the zone loaders write on a secret clear.
    ///
    /// Only bit 0 (and `MACRO_PATH_0`'s own flag) is touched: the bytes' bit 7
    /// is the stage's clear flag, which [`Self::debug_clear_all_levels`] owns.
    /// `MACRO_PATH_2`'s bit 7 and the gate bytes are stage clears too — the map
    /// reads them as "this stage is done" — so they are not path edits.
    pub fn debug_unlock_routes(&mut self) {
        let mut progress = self.debug_progress();
        for offset in [
            po::TREE_PATH_1,
            po::PUMPKIN_PATH_1,
            po::PUMPKIN_PATH_2,
            po::MACRO_PATH_0,
            po::SPACE_PATH_0,
            po::TURTLE_PATH_1,
        ] {
            if let Some(byte) = progress.get_mut(offset) {
                *byte |= 0x01;
            }
        }
        self.debug_set_progress(progress);
    }

    /// Throw the save image back to a new game: no stage cleared, no path or
    /// gate bytes, no golden coins.
    pub fn debug_wipe_progress(&mut self) {
        self.debug_set_progress([0; 42]);
    }

    /// Which of the six golden coins are held.
    pub fn debug_golden_coins(&self) -> [bool; 6] {
        let progress = self.debug_progress();
        GOLDEN_COIN_OFFSETS.map(|off| progress[off] & 0x80 != 0)
    }

    /// Collect or remove golden coin `index` (0-5).
    ///
    /// A coin's byte *is* its boss stage's clear bit, so removing a coin also
    /// un-clears that stage — exactly the state the game can be in before the
    /// boss falls.
    pub fn debug_set_golden_coin(&mut self, index: usize, held: bool) {
        let Some(&offset) = GOLDEN_COIN_OFFSETS.get(index) else {
            return;
        };
        let mut progress = self.debug_progress();
        if held {
            progress[offset] |= 0x80;
        } else {
            progress[offset] &= !0x80;
        }
        self.debug_set_progress(progress);
    }

    /// `sLevelTally` as the live state holds it (BCD).
    pub fn debug_tally(&self) -> u8 {
        match &self.state {
            SessionState::InLevel(ctx) => ctx.sim.level_tally,
            SessionState::Castle(seq) => seq.level_tally,
            _ => self.level_tally,
        }
    }

    /// Write the stage tally, as a decimal count, into every live copy.
    fn debug_set_tally(&mut self, stages: u8) {
        let packed = u8_to_bcd(stages.min(99));
        self.level_tally = packed;
        match &mut self.state {
            SessionState::InLevel(ctx) => ctx.sim.level_tally = packed,
            SessionState::Castle(seq) => seq.level_tally = packed,
            _ => {}
        }
    }

    /// The live purse as a decimal value (`sCoinLow`/`sCoinHigh` are BCD).
    pub fn debug_coins(&self) -> u16 {
        let (low, high) = match &self.state {
            SessionState::Overworld(map) => (map.coins_low, map.coins_high),
            SessionState::InLevel(ctx) => (ctx.sim.coins_low, ctx.sim.coins_high),
            _ => (self.coins_low, self.coins_high),
        };
        bcd_to_u16(low, high)
    }

    /// Write the purse, clamped to the four BCD digits the ROM displays.
    pub fn debug_set_coins(&mut self, coins: u16) {
        let (low, high) = u16_to_bcd(coins);
        self.coins_low = low;
        self.coins_high = high;
        match &mut self.state {
            SessionState::Overworld(map) => {
                map.coins_low = low;
                map.coins_high = high;
            }
            SessionState::InLevel(ctx) => {
                ctx.sim.coins_low = low;
                ctx.sim.coins_high = high;
            }
            _ => {}
        }
    }

    /// Add (or subtract) coins, saturating at 0000 and 9999.
    pub fn debug_add_coins(&mut self, delta: i32) {
        let now = i32::from(self.debug_coins());
        self.debug_set_coins(now.saturating_add(delta).clamp(0, 9999) as u16);
    }

    /// The live life counter as a decimal value (`sLives` is BCD).
    pub fn debug_lives(&self) -> u8 {
        let lives = match &self.state {
            SessionState::Overworld(map) => map.lives,
            SessionState::InLevel(ctx) => ctx.sim.lives,
            _ => self.lives,
        };
        bcd_to_u8(lives)
    }

    /// Write the life counter, clamped to the two BCD digits.
    pub fn debug_set_lives(&mut self, lives: u8) {
        let packed = u8_to_bcd(lives.min(99));
        self.lives = packed;
        match &mut self.state {
            SessionState::Overworld(map) => map.lives = packed,
            SessionState::InLevel(ctx) => ctx.sim.lives = packed,
            _ => {}
        }
    }

    /// Add (or subtract) lives, saturating at 00 and 99.
    pub fn debug_add_lives(&mut self, delta: i32) {
        let now = i32::from(self.debug_lives());
        self.debug_set_lives(now.saturating_add(delta).clamp(0, 99) as u8);
    }

    /// The live `sCurPowerup`: 0 small, 1 super, 2 bunny, 3 fire.
    pub fn debug_powerup(&self) -> u8 {
        match &self.state {
            SessionState::Overworld(map) => map.powerup,
            SessionState::InLevel(ctx) => ctx.sim.powerup,
            _ => self.powerup,
        }
    }

    /// Put Mario in power-up tier `tier` (0-3).
    ///
    /// Inside a level a gain is staged as the matching item in
    /// `sPendingItemId`, so the next frame's `$3451` consumer grants it with
    /// the pickup's own sound and 64 frames of invincibility. Small has no
    /// item, so it is written directly. Elsewhere only the carried tier exists,
    /// and the next level entry picks it up.
    pub fn debug_set_powerup(&mut self, tier: u8) -> Result<(), String> {
        let item = match tier {
            0 => None,
            1 => Some(ITEM_MUSHROOM),
            2 => Some(ITEM_CARROT),
            3 => Some(ITEM_FLOWER),
            _ => return Err(format!("power-up tier {tier} does not exist (0..=3)")),
        };
        match &mut self.state {
            SessionState::InLevel(ctx) => match item {
                Some(item) => ctx.sim.pending_item = item,
                None => ctx.sim.powerup = 0,
            },
            SessionState::Overworld(map) => {
                map.powerup = tier;
                self.powerup = tier;
            }
            _ => self.powerup = tier,
        }
        Ok(())
    }

    /// Stage a star in `sPendingItemId`: the next frame's `$3489` starts the
    /// timer and the star music. Only a running level has a star.
    pub fn debug_give_star(&mut self) -> Result<(), String> {
        let SessionState::InLevel(ctx) = &mut self.state else {
            return Err("a star only works inside a level".into());
        };
        ctx.sim.pending_item = ITEM_STAR;
        Ok(())
    }

    /// Pull the live lives, purse and power-up into the session's carry
    /// fields before a warp.
    ///
    /// The session copies are only refreshed on the level/map hand-offs, so a
    /// warp that built its new state from them would drop whatever the player
    /// collected or spent since the last one.
    fn debug_sync_carry(&mut self) {
        let (lives, coins_low, coins_high, powerup) = match &self.state {
            SessionState::Overworld(map) => (map.lives, map.coins_low, map.coins_high, map.powerup),
            SessionState::InLevel(ctx) => {
                (ctx.sim.lives, ctx.sim.coins_low, ctx.sim.coins_high, ctx.sim.powerup)
            }
            _ => return,
        };
        self.lives = lives;
        self.coins_low = coins_low;
        self.coins_high = coins_high;
        self.powerup = powerup;
    }

    /// A warp leaves the level without `$2756`'s `sSoundMode = 2`, so hand the
    /// engine back here or a warp from the pause screen stays silent.
    fn debug_release_pause_audio(&mut self) {
        self.sound_driver.sound_paused = false;
        self.sound_driver.sound_mode = 0;
        self.sound_driver.pause_beep_timer = 0;
        self.sound_driver.pause_beep_index = 0;
    }

    /// Put the overworld on `screen` with the cursor on `node`.
    ///
    /// The landing is the loader's own (`load_screen` then `$3EEA4`'s node
    /// record), with the arrival latch set so the zone auto-walk (`$3C32D`)
    /// does not run the cursor off the node on the first frame.
    pub fn debug_warp_map(&mut self, screen: u8, node: u8) -> Result<(), String> {
        if screen > WARP_MAX_SCREEN {
            return Err(format!(
                "screen {screen} is not an overworld screen (0..={WARP_MAX_SCREEN})"
            ));
        }
        if self.assets.nodes.is_empty() {
            return Err("no overworld node table is loaded".into());
        }
        if node as usize >= MapNodes::NODES {
            return Err(format!(
                "node {node} is outside the node table (0..{})",
                MapNodes::NODES - 1
            ));
        }
        self.remember_level_state();
        self.debug_sync_carry();
        let mut map = MapSim::new();
        map.set_castle_scripts(self.assets.castle_scripts.clone());
        map.lives = self.lives;
        map.powerup = self.powerup;
        map.coins_low = self.coins_low;
        map.coins_high = self.coins_high;
        map.level_progress = self.debug_progress();
        map.node = node;
        // A warp is an arrival, not a fresh screen entry: `sMarioDead != 0`
        // stops the loader's exit-tile path bits and the arrival auto-walk.
        map.mario_dead = 0xFF;
        let music = map.load_screen(screen, 0, &self.assets.node_records);
        map.load_node(node, &self.assets.node_records);

        // The warp replaces any transition the host had queued.
        self.pending_level = None;
        self.map_screen_transition = None;
        self.map_pan_return = None;
        self.bell_entry = None;
        self.bell_fade = None;
        self.bell_return = None;
        self.castle_return = None;
        self.castle_saved_node = None;
        self.last_map_screen = Some(screen);
        self.map_wave = if screen == 0 {
            sml2_core::map::MapWave::seed(self.assets.overworld_tiles.screen_raw(0))
        } else {
            None
        };
        self.debug_release_pause_audio();
        self.sound_driver.music_request = music;
        self.state = SessionState::Overworld(map);
        Ok(())
    }

    /// Enter zone `screen` (1-8) the way the game does: stand on its hub
    /// door, press A, and let that frame's `$3C60B` dispatch run the door's
    /// loader and start the bank-15 walk-in. Nothing from the screen the warp
    /// left survives, because the hub is rebuilt first.
    pub fn debug_enter_zone(&mut self, screen: u8) -> Result<(), String> {
        let Some(&(_, door)) = ZONE_DOORS.iter().find(|(id, _)| *id == screen) else {
            return Err(format!("screen {screen} has no hub door (1..=8)"));
        };
        self.debug_warp_map(0, door)?;
        let SessionState::Overworld(map) = &mut self.state else {
            return Err("the hub did not load".into());
        };
        map.cursor_move = 0;
        let scripts = map.castle_scripts.take();
        let source = map.clone();
        map.castle_scripts = scripts;
        let entry = map.tick(
            sml2_core::map_sim::MapInput::new(0x01, 0x01, 0),
            &self.assets.nodes,
            &self.assets.walk,
            &self.assets.node_records,
        );
        let destination = map.screen_id;
        let Some(direction) = entry.transition else {
            return Err(format!("hub door ${door:02X} did not open"));
        };
        // The A press is consumed here; the host's held button must not read
        // as a fresh press once the walk-in ends.
        self.prev_keys_held = 0x01;
        self.begin_door_transition(source, direction, destination);
        Ok(())
    }

    /// Put `level_id` live immediately, carrying the map's progress and the
    /// node the player would exit back to.
    ///
    /// This is `begin_level` plus the state the map's entry path would have
    /// copied in: `sLevelProgress`, its working flags byte, and `sMapNode`, so
    /// a cleared level hands off exactly where the player came from.
    pub fn debug_warp_level(&mut self, level_id: u8) -> Result<(), String> {
        let progress = self.debug_progress();
        let flags = self
            .debug_level_slot(level_id)
            .map(|slot| progress[slot as usize])
            .unwrap_or(0);
        let node = match &self.state {
            SessionState::Overworld(map) => map.node,
            SessionState::InLevel(ctx) => ctx.sim.map_node,
            _ => 0,
        };
        self.debug_sync_carry();
        self.begin_level(level_id, flags)?;
        self.debug_release_pause_audio();
        let SessionState::InLevel(ctx) = &mut self.state else {
            return Err("the level context did not go live".into());
        };
        ctx.sim.level_progress = progress;
        ctx.sim.level_progress_flags = flags;
        ctx.sim.map_node = node;
        self.pending_level = None;
        self.map_screen_transition = None;
        self.map_pan_return = None;
        self.bell_entry = None;
        self.bell_fade = None;
        self.bell_return = None;
        Ok(())
    }
}

/// The decimal value of a four-digit BCD pair. A nibble above 9 is not a legal
/// digit; the ROM's `daa` would have corrected it, so the port saturates it.
fn bcd_to_u16(low: u8, high: u8) -> u16 {
    let digit = |nibble: u8| u16::from(nibble.min(9));
    digit(high >> 4) * 1000 + digit(high & 0x0F) * 100 + digit(low >> 4) * 10 + digit(low & 0x0F)
}

/// Pack a decimal value into the four BCD digits, clamped to 9999.
fn u16_to_bcd(value: u16) -> (u8, u8) {
    let v = value.min(9999);
    let low = ((v / 10 % 10) as u8) << 4 | (v % 10) as u8;
    let high = ((v / 1000) as u8) << 4 | ((v / 100 % 10) as u8);
    (low, high)
}

fn bcd_to_u8(value: u8) -> u8 {
    let digit = |nibble: u8| nibble.min(9);
    digit(value >> 4) * 10 + digit(value & 0x0F)
}

fn u8_to_bcd(value: u8) -> u8 {
    let v = value.min(99);
    (v / 10) << 4 | (v % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_round_trips_and_saturates() {
        assert_eq!(u16_to_bcd(1234), (0x34, 0x12));
        assert_eq!(bcd_to_u16(0x34, 0x12), 1234);
        assert_eq!(u16_to_bcd(9999), (0x99, 0x99));
        assert_eq!(u16_to_bcd(12345), (0x99, 0x99));
        assert_eq!(bcd_to_u16(0x9A, 0x00), 99, "an illegal nibble reads as 9");
        assert_eq!(u8_to_bcd(7), 0x07);
        assert_eq!(bcd_to_u8(0x42), 42);
        assert_eq!(u8_to_bcd(120), 0x99);
    }
}
