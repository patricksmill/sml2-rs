//! Runtime achievement observation.
//!
//! [`SessionAchievements`] turns the native gameplay state into the typed
//! [`GameEvent`]s that [`sml2_core::achievements::AchievementTracker`] evaluates.
//! It never looks at RetroAchievements or an emulated memory bus, and it never
//! allocates while a frame merely passes without unlocking anything: the fixed
//! defeat baseline and the pending-unlock vector only grow when `observe`
//! reports a new id.

use sml2_core::achievements::{AchievementId, AchievementTracker, GameEvent};
use sml2_core::audio::driver::SoundDriver;

use super::{LevelContext, SessionState};

/// Offsets into `sLevelProgress` of the six golden coins in the ROM's
/// `PlayerSim::GOLDEN_COINS` fold order: Tree, Pumpkin, Mario, Turtle, Space,
/// Macro. This is the order core's `GoldenCoinAwarded::coin_index` uses.
const GOLDEN_COIN_FOLD: [usize; 6] = [5, 9, 29, 23, 17, 14];

/// Bank-4 song 7 (`SONG_POINTERS[6] = 0x5CB3`) ch2 section list starts at
/// `0x5CD6`: eight passes of the normal stream at `0x5D48`, then the Totaka
/// melody stream at `0x5D8E`. While that stream plays, `section_ptr` remains
/// `0x5CE8` (the next word is the `$FFFF` loop marker). Observing the section
/// pointer is the exact source-backed onset; song 12 is unrelated, and
/// `$287E` never requests a second song.
const TOTAKA_SECTION_PTR: u16 = 0x5CE8;

/// One session's achievement state: the core tracker plus the small amount of
/// cross-frame history needed to turn state transitions into events.
#[derive(Default)]
pub struct SessionAchievements {
    tracker: AchievementTracker,
    pending: Vec<AchievementId>,

    // Current level attempt.
    level_clear_emitted: bool,
    boss_event_emitted: bool,
    wario_event_emitted: bool,
    /// A checkpoint entry flag was set for this level. The ROM exposes this as
    /// `sLevelProgressFlags` bit 3 at level entry; it is not inferred from
    /// deaths.
    checkpoint_used: [bool; 32],
    /// Object-engine defeat count observed last frame. `ObjectSim::defeated`
    /// resets on level load, so the baseline resets with it.
    last_level_defeated: u32,

    // Baselines for deltas that span state changes.
    tally_initialized: bool,
    last_tally: u8,
    coins_initialized: bool,
    last_coins: u16,
    lives_initialized: bool,
    last_lives: u8,
    golden_initialized: bool,
    golden_mask: u8,
    all_coins_emitted: bool,

    // Game Over's hidden song.
    totaka_emitted: bool,
}

impl SessionAchievements {
    pub fn take_events(&mut self) -> Vec<AchievementId> {
        std::mem::take(&mut self.pending)
    }

    pub fn restore<I: IntoIterator<Item = u32>>(&mut self, ids: I) -> usize {
        self.tracker.restore_unlocked_ids(ids)
    }

    pub fn is_unlocked(&self, id: AchievementId) -> bool {
        self.tracker.is_unlocked(id)
    }

    /// A new run (save load or Game Over) starts with a fresh core tracker so
    /// `Heart to Heart`'s 99-up disqualification and its counters do not leak
    /// across runs. Unlocked ids are preserved.
    pub fn reset_run(&mut self) {
        let unlocked = self.tracker.unlocked().clone();
        self.tracker = AchievementTracker::new();
        self.tracker.restore_unlocked(unlocked);
        self.checkpoint_used = [false; 32];
        self.level_clear_emitted = false;
        self.boss_event_emitted = false;
        self.wario_event_emitted = false;
        self.last_level_defeated = 0;
        self.tally_initialized = false;
        self.coins_initialized = false;
        self.lives_initialized = false;
        self.golden_initialized = false;
        self.all_coins_emitted = false;
        self.totaka_emitted = false;
    }

    /// Seed the per-level baselines when a level becomes live. The player's
    /// current counters are passed so the first `advance` cannot mistake the
    /// entry value for a rise.
    pub fn begin_level(
        &mut self,
        level_id: u8,
        lives_bcd: u8,
        coins_low: u8,
        coins_high: u8,
        tally_bcd: u8,
    ) {
        self.level_clear_emitted = false;
        self.boss_event_emitted = false;
        self.wario_event_emitted = false;
        self.checkpoint_used[usize::from(level_id)] = false;
        self.last_level_defeated = 0;
        self.lives_initialized = true;
        self.last_lives = bcd_u8(lives_bcd);
        self.coins_initialized = true;
        self.last_coins = bcd_u16(coins_low, coins_high);
        self.tally_initialized = true;
        self.last_tally = tally_bcd;
    }

    fn observe(&mut self, event: GameEvent) {
        let newly = self.tracker.observe(&event);
        if !newly.is_empty() {
            self.pending.extend(newly);
        }
    }

    fn note_checkpoint(&mut self, level: u8, level_progress_flags: u8) {
        if level_progress_flags & 0x08 != 0 {
            self.checkpoint_used[usize::from(level)] = true;
        }
    }

    fn note_defeats(&mut self, defeated: u32) {
        if defeated < self.last_level_defeated {
            self.last_level_defeated = 0;
        }
        if defeated > self.last_level_defeated {
            let delta = defeated - self.last_level_defeated;
            for _ in 0..delta {
                self.observe(GameEvent::EnemyDefeated);
            }
            self.last_level_defeated = defeated;
        }
    }

    /// Observe one live level frame: checkpoint entry, real object defeats and
    /// boss-defeat transitions. Called before the session leaves the level, so
    /// the last frame's events are not lost.
    pub fn on_level_frame(&mut self, ctx: &LevelContext) {
        let level = ctx.manifest.level_id;
        let index = usize::from(level);

        // `sLevelProgressFlags` bit 3 is the checkpoint/entry flag carried by
        // the level entry. Do not infer this from deaths: dying is a level
        // exit, not a checkpoint acquisition.
        self.note_checkpoint(level, ctx.sim.level_progress_flags);

        // Count the native `ObjectSim::defeat()` increments. A live enemy that
        // is merely culled or transformed does not increment this counter.
        self.note_defeats(ctx.obj.defeated);

        // A boss hand-off sets the coin byte in the live `sLevelProgress`
        // before the session builds the castle reveal; observe it here so the
        // award cannot be lost while the session transfers state.
        self.observe_golden_mask(golden_mask(&ctx.sim.level_progress));

        let seconds_left = level_seconds(ctx);
        let checkpoint_used = self.checkpoint_used[index];
        let definition = ctx.boss_definition.unwrap_or(ctx.obj.boss_def_index);
        // Phase 3 is the ending-entry phase set by form3 state 10 after the
        // final Wario runs offscreen; spawning definition 8 alone is only the
        // start of the final fight and is not a victory.
        let wario_final = level == 24 && ctx.cam.boss_fight_phase >= 3;

        if ctx.obj.boss_clear_flag != 0 && !self.boss_event_emitted {
            // Normal bosses (0-5) and Wario's final form (8) are the only
            // defeats that satisfy an achievement. Wario forms 6/7 are the
            // intermediate phases and must not unlock the castle challenge.
            if definition < 6 || definition == 8 {
                self.boss_event_emitted = true;
                let boss_id = if definition >= 6 { 6 } else { definition };
                self.observe(GameEvent::BossDefeat {
                    boss_id,
                    powerup: ctx.sim.powerup,
                    checkpoint_used,
                    seconds_left,
                });
                if boss_id == 6 {
                    self.wario_event_emitted = true;
                    self.observe(GameEvent::WarioDefeated);
                }
            }
        }
        // Final-form Wario ends through the phase exit; keep the event reachable
        // even if the clear flag was already consumed on an earlier frame. The
        // fallback still emits the timed BossDefeat so WariosChallenge sees the
        // remaining-seconds condition, not just the WarioDefeated grant.
        if wario_final && !self.wario_event_emitted {
            self.wario_event_emitted = true;
            if !self.boss_event_emitted {
                self.boss_event_emitted = true;
                self.observe(GameEvent::BossDefeat {
                    boss_id: 6,
                    powerup: ctx.sim.powerup,
                    checkpoint_used,
                    seconds_left,
                });
            }
            self.observe(GameEvent::WarioDefeated);
        }
    }

    /// Observe a successful level exit. Called once per live level attempt.
    pub fn on_level_clear(&mut self, ctx: &LevelContext) {
        if self.level_clear_emitted {
            return;
        }
        let level = ctx.manifest.level_id;
        let index = usize::from(level);
        self.level_clear_emitted = true;
        self.observe(GameEvent::LevelClear {
            level_id: level,
            powerup: ctx.sim.powerup,
            seconds_left: level_seconds(ctx),
            checkpoint_used: self.checkpoint_used[index],
            // The `$4B` exit is the alternate/secret route. The six secret
            // courses themselves are identified by `level_id` in core.
            secret_exit: ctx.sim.exit_tile_flag != 0,
        });
        // The attempt is finished; the next entry starts checkpoint-free.
        self.checkpoint_used[index] = false;
    }

    /// Adopt the current state as the comparison baseline without emitting.
    /// Debug menu writes use this so editing coins, lives, tally or progress is
    /// not mistaken for gameplay and does not award achievements.
    pub(crate) fn rebaseline(
        &mut self,
        state: &SessionState,
        castle_progress: &[u8; 42],
        tally: u8,
        fallback_lives_bcd: u8,
        fallback_coins: (u8, u8),
    ) {
        self.tally_initialized = true;
        self.last_tally = tally;
        self.last_level_defeated = 0;
        if let Some((low, high)) = state_coins(state, fallback_coins) {
            self.coins_initialized = true;
            self.last_coins = bcd_u16(low, high);
        }
        self.lives_initialized = true;
        self.last_lives = bcd_u8(state_lives(state, fallback_lives_bcd));
        if let Some(mask) = state_golden_mask(state, castle_progress) {
            self.golden_initialized = true;
            self.golden_mask = mask;
            self.all_coins_emitted = mask.count_ones() == 6;
        }
    }

    /// Emit coin awards for `sLevelProgress` bits that appeared since the last
    /// observation. Called both for a live level and for the post-clear state.
    fn observe_golden_mask(&mut self, mask: u8) {
        if self.golden_initialized {
            let added = mask & !self.golden_mask;
            if added != 0 {
                for (bit, offset) in GOLDEN_COIN_FOLD.iter().enumerate() {
                    if added & (1 << bit) != 0 {
                        self.observe(GameEvent::GoldenCoinAwarded {
                            coin_index: coin_index_for_offset(*offset),
                        });
                    }
                }
            }
            if mask.count_ones() == 6 && !self.all_coins_emitted {
                self.all_coins_emitted = true;
                self.observe(GameEvent::AllGoldenCoinsHeld);
            }
        } else {
            self.golden_initialized = true;
            if mask.count_ones() == 6 {
                // A loaded save may already hold all six; the local unlock
                // cache is the source for that, not a new award.
                self.all_coins_emitted = true;
            }
        }
        self.golden_mask = mask;
    }

    /// Observe run-wide deltas after a session `advance`: tally, coins, lives,
    /// golden coins and the Game Over easter egg. The Game Over song itself is
    /// requested by the Game Over module in [`super::Session`], not here.
    pub fn after_advance(
        &mut self,
        state: &SessionState,
        castle_progress: &[u8; 42],
        tally: u8,
        sound: &SoundDriver,
        fallback_lives_bcd: u8,
        fallback_coins: (u8, u8),
    ) {
        // `sLevelTally` only moves upward through clears; a Game Over or a
        // load may lower it, which must not re-emit an old tally.
        let tally_value = tally_decimal(tally) as u8;
        if self.tally_initialized {
            if u16::from(tally_value) > tally_decimal(self.last_tally) {
                // Core's contract: `ExitCountUpdated` carries the decimal
                // count (a 0x31 BCD tally is sent as 31).
                self.observe(GameEvent::ExitCountUpdated { tally: tally_value });
            }
        } else {
            self.tally_initialized = true;
        }
        self.last_tally = tally;

        if let Some((coins_low, coins_high)) = state_coins(state, fallback_coins) {
            let coins = bcd_u16(coins_low, coins_high);
            if self.coins_initialized {
                if coins > self.last_coins {
                    self.observe(GameEvent::CoinsUpdated { total_coins: coins });
                }
            } else {
                self.coins_initialized = true;
            }
            self.last_coins = coins;
        }

        let lives_bcd = state_lives(state, fallback_lives_bcd);
        let lives = bcd_u8(lives_bcd);
        if self.lives_initialized {
            if lives > self.last_lives || state_room_99up(state) {
                self.observe(GameEvent::LivesEarned {
                    new_lives: lives,
                    // The bonus room's item 8 / carry sets BCD 99; this flag is
                    // the real 99-up producer, not a life-delta heuristic.
                    is_99up: state_room_99up(state),
                });
            }
        } else {
            self.lives_initialized = true;
        }
        self.last_lives = lives;

        if let Some(mask) = state_golden_mask(state, castle_progress) {
            self.observe_golden_mask(mask);
        }

        if matches!(state, SessionState::GameOver(_))
            && sound.active_song == 7
            && sound.music_sequencer.channels[1].section_ptr == TOTAKA_SECTION_PTR
            && !self.totaka_emitted
        {
            self.totaka_emitted = true;
            self.observe(GameEvent::TotakaSongPlayed);
        }
    }
}

fn level_seconds(ctx: &LevelContext) -> u16 {
    let high = u16::from(bcd_u8(ctx.sim.level_timer.high));
    let low = u16::from(bcd_u8(ctx.sim.level_timer.low));
    high * 100 + low
}

/// Decode one BCD byte (`0x45` -> 45).
fn bcd_u8(value: u8) -> u8 {
    (value >> 4) * 10 + (value & 0x0F)
}

/// Decode the ROM's two-byte BCD coin counter (`0x09`, `0x99` -> 999).
fn bcd_u16(low: u8, high: u8) -> u16 {
    u16::from(bcd_u8(high)) * 100 + u16::from(bcd_u8(low))
}

fn tally_decimal(tally: u8) -> u16 {
    u16::from(bcd_u8(tally))
}

fn coin_index_for_offset(offset: usize) -> u8 {
    match offset {
        5 => 0,  // Tree
        9 => 1,  // Pumpkin
        29 => 2, // Mario
        23 => 3, // Turtle
        17 => 4, // Space
        14 => 5, // Macro
        _ => 0,
    }
}

fn state_coins(state: &SessionState, fallback: (u8, u8)) -> Option<(u8, u8)> {
    match state {
        SessionState::InLevel(ctx) => Some((ctx.sim.coins_low, ctx.sim.coins_high)),
        SessionState::Overworld(map) => Some((map.coins_low, map.coins_high)),
        SessionState::GameOver(screen) => Some((screen.sim.coins_low, screen.sim.coins_high)),
        SessionState::Castle(_) => Some(fallback),
        _ => None,
    }
}

fn state_room_99up(state: &SessionState) -> bool {
    match state {
        SessionState::Overworld(map) => map.room_99up,
        _ => false,
    }
}

fn state_lives(state: &SessionState, fallback: u8) -> u8 {
    match state {
        SessionState::InLevel(ctx) => ctx.sim.lives,
        SessionState::Overworld(map) => map.lives,
        SessionState::GameOver(screen) => screen.sim.lives,
        SessionState::Castle(_) => fallback,
        _ => fallback,
    }
}

fn state_golden_mask(state: &SessionState, castle_progress: &[u8; 42]) -> Option<u8> {
    let progress = match state {
        SessionState::InLevel(ctx) => &ctx.sim.level_progress,
        SessionState::Overworld(map) => &map.level_progress,
        SessionState::GameOver(screen) => &screen.sim.level_progress,
        SessionState::Castle(_) => castle_progress,
        _ => return None,
    };
    Some(golden_mask(progress))
}

fn golden_mask(progress: &[u8; 42]) -> u8 {
    let mut mask = 0u8;
    for (bit, offset) in GOLDEN_COIN_FOLD.iter().enumerate() {
        if progress.get(*offset).is_some_and(|b| b & 0x80 != 0) {
            mask |= 1 << bit;
        }
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Session, SessionAssets};

    fn assets() -> SessionAssets {
        SessionAssets::load().expect("session assets must load")
    }

    fn level(level_id: u8) -> (Session, SessionAchievements) {
        let mut session = Session::new(assets());
        session.begin_level(level_id, 0).expect("level must load");
        let mut achievements = SessionAchievements::default();
        achievements.begin_level(
            level_id,
            session.lives,
            session.coins_low,
            session.coins_high,
            session.level_tally,
        );
        (session, achievements)
    }

    #[test]
    fn level_clear_bcd_timer_unlocks_go_go_mario() {
        let (mut session, mut achievements) = level(10);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        ctx.sim.level_timer.high = 0x03;
        ctx.sim.level_timer.low = 0x45; // 345 > 340
        achievements.on_level_clear(ctx);
        assert!(achievements
            .take_events()
            .contains(&AchievementId::GoGoMario));
    }

    #[test]
    fn level_clear_at_340_seconds_does_not_unlock_go_go_mario() {
        let (mut session, mut achievements) = level(10);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        ctx.sim.level_timer.high = 0x03;
        ctx.sim.level_timer.low = 0x40; // exactly 340, threshold is > 340
        achievements.on_level_clear(ctx);
        assert!(!achievements
            .take_events()
            .contains(&AchievementId::GoGoMario));
    }

    #[test]
    fn boss_clear_requires_no_checkpoint_flag() {
        let (mut session, mut achievements) = level(5);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        ctx.boss_definition = Some(0);
        ctx.obj.boss_clear_flag = 1;
        ctx.sim.powerup = 0;
        ctx.sim.level_timer.high = 0x04;
        ctx.sim.level_timer.low = 0x00;
        ctx.sim.level_progress_flags = 0x08;
        achievements.on_level_frame(ctx);
        assert!(!achievements
            .take_events()
            .contains(&AchievementId::BigBirdsChallenge));

        let (mut session, mut achievements) = level(5);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        ctx.boss_definition = Some(0);
        ctx.obj.boss_clear_flag = 1;
        ctx.sim.powerup = 0;
        ctx.sim.level_progress_flags = 0;
        achievements.on_level_frame(ctx);
        assert!(achievements
            .take_events()
            .contains(&AchievementId::BigBirdsChallenge));
    }

    #[test]
    fn wario_form_8_phase_2_is_not_a_victory() {
        let (mut session, mut achievements) = level(24);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        // Entering definition 8 (phase 2) is the start of the final fight,
        // not a victory.
        ctx.boss_definition = Some(8);
        ctx.obj.boss_clear_flag = 0;
        ctx.cam.boss_fight_phase = 2;
        achievements.on_level_frame(ctx);
        assert!(achievements.take_events().is_empty());
    }

    #[test]
    fn wario_final_phase_3_clear_flag_awards_both() {
        let (mut session, mut achievements) = level(24);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        ctx.boss_definition = Some(8);
        ctx.obj.boss_clear_flag = 1;
        ctx.cam.boss_fight_phase = 3;
        ctx.sim.powerup = 0;
        ctx.sim.level_timer.high = 0x09; // 900 > 650
        ctx.sim.level_timer.low = 0x00;
        achievements.on_level_frame(ctx);
        let events = achievements.take_events();
        assert!(events.contains(&AchievementId::WarioIsNotWelcome));
        assert!(events.contains(&AchievementId::WariosChallenge));
    }

    #[test]
    fn wario_final_phase_3_fallback_awards_both() {
        let (mut session, mut achievements) = level(24);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        // The fallback path has no clear flag; it must still emit the timed
        // BossDefeat so WariosChallenge can unlock.
        ctx.boss_definition = Some(8);
        ctx.obj.boss_clear_flag = 0;
        ctx.cam.boss_fight_phase = 3;
        ctx.sim.powerup = 0;
        ctx.sim.level_timer.high = 0x09;
        ctx.sim.level_timer.low = 0x00;
        achievements.on_level_frame(ctx);
        let events = achievements.take_events();
        assert!(events.contains(&AchievementId::WarioIsNotWelcome));
        assert!(events.contains(&AchievementId::WariosChallenge));
    }

    #[test]
    fn wario_form_7_is_not_a_victory() {
        let (mut session, mut achievements) = level(24);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        ctx.boss_definition = Some(7);
        ctx.obj.boss_clear_flag = 0;
        ctx.cam.boss_fight_phase = 2;
        achievements.on_level_frame(ctx);
        assert!(achievements.take_events().is_empty());
    }

    #[test]
    fn golden_coin_bit_unlocks_its_zone() {
        let (mut session, mut achievements) = level(5);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        achievements.on_level_frame(ctx); // establish the zero mask
        ctx.sim.level_progress[5] = 0x80;
        achievements.on_level_frame(ctx);
        assert!(achievements
            .take_events()
            .contains(&AchievementId::BranchingOut));
    }

    #[test]
    fn defeat_counter_in_ctx_unlocks_make_em_see_stars() {
        let (mut session, mut achievements) = level(0);
        let SessionState::InLevel(ctx) = &mut session.state else {
            panic!("level context");
        };
        ctx.obj.defeated = 99;
        achievements.on_level_frame(ctx);
        assert!(achievements
            .take_events()
            .contains(&AchievementId::MakeEmSeeStars));
    }

    #[test]
    fn song7_reaches_the_totaka_section_pointer() {
        let assets = assets();
        let mut sound = SoundDriver::with_music_bank(assets.music_bank.clone());
        sound.music_request = 7;
        let mut reached = false;
        // The normal ch2 stream uses several `$A3` durations whose zero table
        // entry wraps to a 256-frame hold; eight passes exceed 20k frames.
        for _ in 0..100_000 {
            sound.update_sound();
            if sound.music_sequencer.channels[1].section_ptr == TOTAKA_SECTION_PTR {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "song 7 must reach the Totaka section pointer at {TOTAKA_SECTION_PTR:#06X}"
        );

        let state = SessionState::GameOver(Box::new(crate::session::GameOverScreen {
            sim: sml2_core::player_sim::PlayerSim::new(),
            obp: [0, 0],
        }));
        let mut achievements = SessionAchievements::default();
        achievements.after_advance(&state, &[0; 42], 0, &sound, 5, (0, 0));
        assert!(achievements
            .take_events()
            .contains(&AchievementId::MiseryLovesMelody));
    }

    #[test]
    fn reset_run_keeps_unlocks_and_clears_disqualification() {
        let mut achievements = SessionAchievements::default();
        achievements.restore([AchievementId::GoGoMario.id()]);
        achievements.observe(GameEvent::LivesEarned {
            new_lives: 99,
            is_99up: true,
        });
        achievements.reset_run();
        assert!(achievements.is_unlocked(AchievementId::GoGoMario));
        achievements.observe(GameEvent::LivesEarned {
            new_lives: 20,
            is_99up: false,
        });
        assert!(achievements
            .take_events()
            .contains(&AchievementId::HeartToHeart));
    }
}
