//! Native, offline achievement tracking for *Super Mario Land 2*.
//!
//! The definitions mirror the game's 28 current public achievement listings
//! (302 points), but every identifier here is **local**. The numeric values
//! deliberately borrow the contiguous namespace shown by an Exophase mirror of
//! that listing (`25136..=25163`); that is not the RetroAchievements
//! achievement-id namespace, and no value here is claimed to be an RA id or a
//! sync contract. They are stable keys for the local save file only.
//!
//! Criteria are evaluated from typed gameplay events rather than an emulated
//! memory bus. Level ids are the ROM's `sCurLevel` (the zero-based index
//! documented in `docs/route-matrix.md`); boss ids are the definition indices
//! from [`sml2_core::scene::boss_def_index`](crate::scene::boss_def_index).
//! Secret levels are the six course ids reached from the zones' alternate
//! exits, verified against the extracted map clusters and the public course
//! lists.
//!
//! No badge artwork ships with the crate; [`AchievementMetadata::badge`] is
//! `None` and the desktop UI is expected to render a text fallback.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// First local achievement id (`Branching Out`). The enum's discriminants are
/// contiguous from here to [`LAST_ACHIEVEMENT_ID`].
pub const FIRST_ACHIEVEMENT_ID: u32 = 25_136;

/// Last local achievement id (`Misery Loves Melody`).
pub const LAST_ACHIEVEMENT_ID: u32 = 25_163;

/// `MarioPowerup::Small as u8`: the only form that satisfies a "Small Mario"
/// criterion.
pub const POWERUP_SMALL: u8 = 0;

/// Enemies defeated for `Make 'Em See Stars`.
pub const ENEMIES_REQUIRED: u32 = 99;
/// Lives held (without a 99-up) for `Heart to Heart`.
pub const LIVES_REQUIRED: u8 = 20;
/// Coin counter value for `Prosperous Plumber`.
pub const COINS_REQUIRED: u16 = 999;
/// Exits cleared for `Master of Mario Land`.
pub const EXITS_REQUIRED: u8 = 31;
/// Number of secret courses for `Shhh... Don't Tell!`.
pub const SECRET_LEVEL_COUNT: usize = 6;

/// Zero-based ROM level ids (`sCurLevel`) used by the criteria. The names are
/// the extracted route-matrix labels, in zone order.
pub mod level_id {
    /// The tutorial stage (`Level1` / Mushroom House).
    pub const MUSHROOM_HOUSE: u8 = 0;
    pub const TREE_ZONE_1: u8 = 1;
    pub const TREE_ZONE_2: u8 = 2;
    pub const TREE_ZONE_3: u8 = 3;
    pub const TREE_ZONE_4: u8 = 4;
    /// Owl Course, the Tree Zone boss arena (def 0).
    pub const TREE_ZONE_5: u8 = 5;
    pub const PUMPKIN_ZONE_1: u8 = 6;
    pub const PUMPKIN_ZONE_2: u8 = 7;
    pub const PUMPKIN_ZONE_3: u8 = 8;
    /// Witch's Mansion, the Pumpkin Zone boss arena (def 1).
    pub const PUMPKIN_ZONE_4: u8 = 9;
    pub const MARIO_ZONE_1: u8 = 10;
    pub const MARIO_ZONE_2: u8 = 11;
    pub const MARIO_ZONE_3: u8 = 12;
    /// Block Course, the Mario Zone boss arena (def 2).
    pub const MARIO_ZONE_4: u8 = 13;
    pub const TURTLE_ZONE_1: u8 = 14;
    pub const TURTLE_ZONE_2: u8 = 15;
    /// Whale Course, the Turtle Zone boss arena (def 3).
    pub const TURTLE_ZONE_3: u8 = 16;
    /// A Turtle-tileset secret-course variant (`Secret Course 02`), not one of
    /// the six counted secret courses.
    pub const TURTLE_SECRET_VARIANT: u8 = 17;
    /// Moon Course, Space Zone 1.
    pub const SPACE_ZONE_1: u8 = 18;
    /// Star Course, Space Zone 2 and the Tatanga arena (def 4).
    pub const SPACE_ZONE_2: u8 = 19;
    pub const MACRO_ZONE_1: u8 = 20;
    pub const MACRO_ZONE_2: u8 = 21;
    pub const MACRO_ZONE_3: u8 = 22;
    /// Attic Course, the Macro Zone boss arena (def 5).
    pub const MACRO_ZONE_4: u8 = 23;
    /// Captured Mario's Castle; Wario's arena (def 6).
    pub const WARIO_CASTLE: u8 = 24;
    /// An autoscrolling tutorial-tileset variant (`Secret Course 01`), not one
    /// of the six counted secret courses.
    pub const TUTORIAL_SECRET_VARIANT: u8 = 25;
    /// Secret Course 6 (Turtle Zone).
    pub const SECRET_TURTLE: u8 = 26;
    /// Secret Course 4 (Pumpkin Zone).
    pub const SECRET_PUMPKIN_4: u8 = 27;
    /// Secret Course 2 (Space Zone).
    pub const SECRET_SPACE: u8 = 28;
    /// Secret Course 1 (Tree Zone).
    pub const SECRET_TREE: u8 = 29;
    /// Secret Course 3 (Macro Zone).
    pub const SECRET_MACRO: u8 = 30;
    /// Secret Course 5 (Pumpkin Zone).
    pub const SECRET_PUMPKIN_5: u8 = 31;
}

/// Boss definition ids, exactly as
/// [`crate::scene::boss_def_index`] assigns them from the boss level's
/// `sCurLevel`.
pub mod boss_id {
    /// Tree Zone: Big Bird.
    pub const BIG_BIRD: u8 = 0;
    /// Pumpkin Zone: the Witch.
    pub const WITCH: u8 = 1;
    /// Mario Zone: the Three Little Pigs.
    pub const THREE_PIGS: u8 = 2;
    /// Turtle Zone: the Octopus.
    pub const OCTOPUS: u8 = 3;
    /// Space Zone: Tatanga.
    pub const TATANGA: u8 = 4;
    /// Macro Zone: the Sewer Rat.
    pub const SEWER_RAT: u8 = 5;
    /// Wario's Castle: Wario (first form; phase exits stay in this id family).
    pub const WARIO: u8 = 6;
}

/// The six secret courses counted by `Shhh... Don't Tell!`: Tree 1, Space 2,
/// Macro 3, Pumpkin 4 and 5, Turtle 6.
pub const SECRET_LEVEL_IDS: [u8; SECRET_LEVEL_COUNT] = [
    level_id::SECRET_TREE,
    level_id::SECRET_SPACE,
    level_id::SECRET_MACRO,
    level_id::SECRET_PUMPKIN_4,
    level_id::SECRET_PUMPKIN_5,
    level_id::SECRET_TURTLE,
];

/// Whether `level_id` is one of the six secret courses.
pub const fn is_secret_level(level: u8) -> bool {
    matches!(
        level,
        level_id::SECRET_TREE
            | level_id::SECRET_SPACE
            | level_id::SECRET_MACRO
            | level_id::SECRET_PUMPKIN_4
            | level_id::SECRET_PUMPKIN_5
            | level_id::SECRET_TURTLE
    )
}

/// One locally tracked achievement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u32)]
pub enum AchievementId {
    BranchingOut = 25_136,
    WhereNoPlumberGoneBefore = 25_137,
    TheBigHouse = 25_138,
    TrickOrTreat = 25_139,
    ItsAMe = 25_140,
    ShellShock = 25_141,
    HereComesTheLandlord = 25_142,
    WarioIsNotWelcome = 25_143,
    MakeEmSeeStars = 25_144,
    HeartToHeart = 25_145,
    ProsperousPlumber = 25_146,
    ShhhDontTell = 25_147,
    MasterOfMarioLand = 25_148,
    TreeTrek = 25_149,
    BigBirdsChallenge = 25_150,
    SpaceSprint = 25_151,
    TatangasChallenge = 25_152,
    MacroMarch = 25_153,
    SewerRatsChallenge = 25_154,
    PumpkinPromenade1 = 25_155,
    PumpkinPromenade2 = 25_156,
    WitchsChallenge = 25_157,
    GoGoMario = 25_158,
    ThreeLittlePigsChallenge = 25_159,
    OctopusChallenge = 25_160,
    TurtleTrot = 25_161,
    WariosChallenge = 25_162,
    MiseryLovesMelody = 25_163,
}

impl AchievementId {
    /// The stable local id (also the persisted settings value).
    pub const fn id(self) -> u32 {
        self as u32
    }

    /// Parse a persisted local id. Unknown values are ignored, so an old save
    /// containing a removed id cannot corrupt the tracker.
    pub fn from_id(id: u32) -> Option<Self> {
        let index = id.checked_sub(FIRST_ACHIEVEMENT_ID)? as usize;
        ALL_ACHIEVEMENTS.get(index).copied()
    }

    /// All achievements, in id order.
    pub const fn all() -> &'static [AchievementId] {
        ALL_ACHIEVEMENTS
    }

    /// Title, description, point value and optional local badge key.
    pub fn metadata(self) -> &'static AchievementMetadata {
        &METADATA[(self.id() - FIRST_ACHIEVEMENT_ID) as usize]
    }

    /// Point value of this achievement.
    pub fn points(self) -> u32 {
        self.metadata().points
    }
}

/// Static achievement text. `badge` is an optional local asset key; the crate
/// bundles no artwork, so it is `None`. It is never an official badge name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AchievementMetadata {
    pub title: &'static str,
    pub description: &'static str,
    pub points: u32,
    pub badge: Option<&'static str>,
}

const METADATA: [AchievementMetadata; 28] = [
    AchievementMetadata {
        title: "Branching Out",
        description: "Complete the Tree Zone",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Where No Plumber Has Gone Before",
        description: "Complete the Space Zone",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "The Big House",
        description: "Complete the Macro Zone",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Trick or Treat!",
        description: "Complete the Pumpkin Zone",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "It's A Me!",
        description: "Complete the Mario Zone",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Shell Shock",
        description: "Complete the Turtle Zone",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Here Comes the Landlord",
        description: "Collect all 6 Golden Coins",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Wario Is Not Welcome",
        description: "Defeat Wario and get your castle back",
        points: 25,
        badge: None,
    },
    AchievementMetadata {
        title: "Make 'Em See Stars",
        description: "Defeat 99 enemies",
        points: 3,
        badge: None,
    },
    AchievementMetadata {
        title: "Heart to Heart",
        description: "Earn 20 lives without a 99up",
        points: 4,
        badge: None,
    },
    AchievementMetadata {
        title: "Prosperous Plumber",
        description: "Collect 999 coins",
        points: 5,
        badge: None,
    },
    AchievementMetadata {
        title: "Shhh... Don't Tell!",
        description: "Beat all 6 secret levels",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Master of Mario Land",
        description: "Complete all 31 exits",
        points: 25,
        badge: None,
    },
    AchievementMetadata {
        title: "Tree Trek",
        description:
            "Beat Tree Zone 3 as Small Mario with over 340 seconds left - the level is on the left!",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "The Big Bird's Challenge",
        description: "Beat Tree Zone's boss level checkpointless as Small Mario",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Space Sprint",
        description: "Beat Space Zone 1 as Small Mario with over 625 seconds left",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Tatanga's Challenge",
        description: "Beat Space Zone's boss level checkpointless as Small Mario",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Macro March",
        description: "Beat Macro Zone 3 as Small Mario with over 330 seconds left",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "The Sewer Rat's Challenge",
        description: "Beat Macro Zone's boss level checkpointless as Small Mario",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Pumpkin Promenade I",
        description: "Beat Pumpkin Zone 1 as Small Mario with over 445 seconds left",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Pumpkin Promenade II",
        description: "Beat Pumpkin Zone 2 as Small Mario with over 320 seconds left",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "The Witch's Challenge",
        description: "Beat Pumpkin Zone's boss level checkpointless as Small Mario",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Go Go Mario!",
        description: "Beat Mario Zone 1 as Small Mario with over 340 seconds left",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "The Three Little Pigs' Challenge",
        description: "Beat Mario Zone's boss level checkpointless as Small Mario",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "The Octopus' Challenge",
        description: "Beat Turtle Zone's boss level checkpointless as Small Mario",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Turtle Trot",
        description: "Beat Turtle Zone 2 as Small Mario with over 530 seconds left",
        points: 10,
        badge: None,
    },
    AchievementMetadata {
        title: "Wario's Challenge",
        description: "Beat Mario's Castle as Small Mario with over 650 seconds left",
        points: 25,
        badge: None,
    },
    AchievementMetadata {
        title: "Misery Loves Melody",
        description: "Listen to Totaka's Song",
        points: 5,
        badge: None,
    },
];

/// All achievements, in id order. Kept in lockstep with the enum by the
/// `metadata_table_matches_enum` test.
pub const ALL_ACHIEVEMENTS: &[AchievementId] = &[
    AchievementId::BranchingOut,
    AchievementId::WhereNoPlumberGoneBefore,
    AchievementId::TheBigHouse,
    AchievementId::TrickOrTreat,
    AchievementId::ItsAMe,
    AchievementId::ShellShock,
    AchievementId::HereComesTheLandlord,
    AchievementId::WarioIsNotWelcome,
    AchievementId::MakeEmSeeStars,
    AchievementId::HeartToHeart,
    AchievementId::ProsperousPlumber,
    AchievementId::ShhhDontTell,
    AchievementId::MasterOfMarioLand,
    AchievementId::TreeTrek,
    AchievementId::BigBirdsChallenge,
    AchievementId::SpaceSprint,
    AchievementId::TatangasChallenge,
    AchievementId::MacroMarch,
    AchievementId::SewerRatsChallenge,
    AchievementId::PumpkinPromenade1,
    AchievementId::PumpkinPromenade2,
    AchievementId::WitchsChallenge,
    AchievementId::GoGoMario,
    AchievementId::ThreeLittlePigsChallenge,
    AchievementId::OctopusChallenge,
    AchievementId::TurtleTrot,
    AchievementId::WariosChallenge,
    AchievementId::MiseryLovesMelody,
];

impl From<AchievementId> for u32 {
    fn from(id: AchievementId) -> Self {
        id.id()
    }
}

impl TryFrom<u32> for AchievementId {
    type Error = ();

    fn try_from(id: u32) -> Result<Self, Self::Error> {
        Self::from_id(id).ok_or(())
    }
}

/// A typed gameplay fact the tracker observes. Runtime emits these from native
/// state; it never inspects the tracker's internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEvent {
    /// A level was cleared. `level_id` is ROM `sCurLevel`; `powerup` is the raw
    /// `sCurPowerup`; `seconds_left` is the decimal in-level timer.
    LevelClear {
        level_id: u8,
        powerup: u8,
        seconds_left: u16,
        checkpoint_used: bool,
        secret_exit: bool,
    },
    /// A boss reached its defeat/reward state. `boss_id` is the
    /// `boss_def_index` value.
    BossDefeat {
        boss_id: u8,
        powerup: u8,
        checkpoint_used: bool,
        seconds_left: u16,
    },
    /// A zone's golden coin was awarded. `coin_index` follows the ROM's
    /// `GOLDEN_COINS` order: Tree, Pumpkin, Mario, Turtle, Space, Macro.
    GoldenCoinAwarded { coin_index: u8 },
    /// All six golden coins are held.
    AllGoldenCoinsHeld,
    /// Wario was defeated (the ending/reward path ran).
    WarioDefeated,
    /// The level-clear tally changed. `tally` is the **decimal** exit count;
    /// runtime decodes the ROM's BCD byte at the event boundary (`0x31` -> 31).
    ExitCountUpdated { tally: u8 },
    /// The coin counter reached `total_coins` (decimal, 0..=999, not lifetime).
    CoinsUpdated { total_coins: u16 },
    /// A life total changed. `new_lives` is decimal; `is_99up` marks a 99-up
    /// jump, which invalidates `Heart to Heart`.
    LivesEarned { new_lives: u8, is_99up: bool },
    /// A regular enemy was defeated. Boss parts are not counted.
    EnemyDefeated,
    /// The Game Over screen played Totaka's Song.
    TotakaSongPlayed,
}

/// Incremental achievement observer.
///
/// `observe` returns only achievements unlocked by that call, in a stable
/// order, and never returns an id twice. `restore_unlocked*` rebuilds a
/// tracker from persisted ids without re-emitting events.
#[derive(Debug, Clone, Default)]
pub struct AchievementTracker {
    unlocked: BTreeSet<AchievementId>,
    enemies_defeated: u32,
    lives_peak: u8,
    /// `Heart to Heart` is a per-run criterion. A 99-up inflates the counter, so
    /// no later ordinary `LivesEarned` event in this tracker's lifetime may
    /// satisfy it; a new run must start from a fresh tracker.
    heart_disqualified: bool,
    secret_levels_cleared: BTreeSet<u8>,
}

impl AchievementTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Achievements unlocked so far, in id order.
    pub fn unlocked(&self) -> &BTreeSet<AchievementId> {
        &self.unlocked
    }

    pub fn is_unlocked(&self, id: AchievementId) -> bool {
        self.unlocked.contains(&id)
    }

    /// Restore a persisted set of achievements.
    pub fn restore_unlocked<I: IntoIterator<Item = AchievementId>>(&mut self, ids: I) {
        self.unlocked.extend(ids);
    }

    /// Restore persisted `u32` ids, ignoring unknown values. Returns how many
    /// supplied entries resolved to a known achievement.
    pub fn restore_unlocked_ids<I: IntoIterator<Item = u32>>(&mut self, ids: I) -> usize {
        let mut recognised = 0;
        for id in ids {
            if let Some(achievement) = AchievementId::from_id(id) {
                self.unlocked.insert(achievement);
                recognised += 1;
            }
        }
        recognised
    }

    /// Evaluate `event`, returning the achievements this call newly unlocked.
    pub fn observe(&mut self, event: &GameEvent) -> Vec<AchievementId> {
        let mut newly = Vec::new();
        match *event {
            GameEvent::LevelClear {
                level_id: level,
                powerup,
                seconds_left,
                secret_exit: _,
                checkpoint_used: _,
            } => {
                if powerup == POWERUP_SMALL {
                    if let Some(id) = speed_run_achievement(level, seconds_left) {
                        self.grant(id, &mut newly);
                    }
                    // Wario's Castle has no goal bell, but some runtimes report
                    // the castle as a level clear; accept either path.
                    if level == level_id::WARIO_CASTLE && seconds_left > 650 {
                        self.grant(AchievementId::WariosChallenge, &mut newly);
                    }
                }
                if is_secret_level(level) {
                    self.secret_levels_cleared.insert(level);
                    if self.secret_levels_cleared.len() >= SECRET_LEVEL_COUNT {
                        self.grant(AchievementId::ShhhDontTell, &mut newly);
                    }
                }
            }
            GameEvent::BossDefeat {
                boss_id: boss,
                powerup,
                checkpoint_used,
                seconds_left,
            } => {
                let small = powerup == POWERUP_SMALL;
                let checkpointless = !checkpoint_used;
                let id = match boss {
                    boss_id::BIG_BIRD => Some(AchievementId::BigBirdsChallenge),
                    boss_id::WITCH => Some(AchievementId::WitchsChallenge),
                    boss_id::THREE_PIGS => Some(AchievementId::ThreeLittlePigsChallenge),
                    boss_id::OCTOPUS => Some(AchievementId::OctopusChallenge),
                    boss_id::TATANGA => Some(AchievementId::TatangasChallenge),
                    boss_id::SEWER_RAT => Some(AchievementId::SewerRatsChallenge),
                    _ => None,
                };
                if small && checkpointless {
                    if let Some(id) = id {
                        self.grant(id, &mut newly);
                    }
                }
                if boss == boss_id::WARIO {
                    self.grant(AchievementId::WarioIsNotWelcome, &mut newly);
                    if small && seconds_left > 650 {
                        self.grant(AchievementId::WariosChallenge, &mut newly);
                    }
                }
            }
            GameEvent::GoldenCoinAwarded { coin_index } => {
                if let Some(id) = zone_achievement(coin_index) {
                    self.grant(id, &mut newly);
                }
            }
            GameEvent::AllGoldenCoinsHeld => {
                self.grant(AchievementId::HereComesTheLandlord, &mut newly);
            }
            GameEvent::WarioDefeated => {
                self.grant(AchievementId::WarioIsNotWelcome, &mut newly);
            }
            GameEvent::ExitCountUpdated { tally } => {
                if tally >= EXITS_REQUIRED {
                    self.grant(AchievementId::MasterOfMarioLand, &mut newly);
                }
            }
            GameEvent::CoinsUpdated { total_coins } => {
                if total_coins >= COINS_REQUIRED {
                    self.grant(AchievementId::ProsperousPlumber, &mut newly);
                }
            }
            GameEvent::LivesEarned { new_lives, is_99up } => {
                if is_99up {
                    self.heart_disqualified = true;
                } else if !self.heart_disqualified {
                    self.lives_peak = self.lives_peak.max(new_lives);
                    if self.lives_peak >= LIVES_REQUIRED {
                        self.grant(AchievementId::HeartToHeart, &mut newly);
                    }
                }
            }
            GameEvent::EnemyDefeated => {
                self.enemies_defeated = self.enemies_defeated.saturating_add(1);
                if self.enemies_defeated >= ENEMIES_REQUIRED {
                    self.grant(AchievementId::MakeEmSeeStars, &mut newly);
                }
            }
            GameEvent::TotakaSongPlayed => {
                self.grant(AchievementId::MiseryLovesMelody, &mut newly);
            }
        }
        newly
    }

    fn grant(&mut self, id: AchievementId, newly: &mut Vec<AchievementId>) {
        if self.unlocked.insert(id) {
            newly.push(id);
        }
    }
}

/// The timer achievement for a regular level, if `seconds_left` clears its
/// "over N" threshold. Levels without a speed-run achievement return `None`.
fn speed_run_achievement(level: u8, seconds_left: u16) -> Option<AchievementId> {
    match level {
        level_id::PUMPKIN_ZONE_1 if seconds_left > 445 => Some(AchievementId::PumpkinPromenade1),
        level_id::PUMPKIN_ZONE_2 if seconds_left > 320 => Some(AchievementId::PumpkinPromenade2),
        level_id::TREE_ZONE_3 if seconds_left > 340 => Some(AchievementId::TreeTrek),
        level_id::SPACE_ZONE_1 if seconds_left > 625 => Some(AchievementId::SpaceSprint),
        level_id::MACRO_ZONE_3 if seconds_left > 330 => Some(AchievementId::MacroMarch),
        level_id::MARIO_ZONE_1 if seconds_left > 340 => Some(AchievementId::GoGoMario),
        level_id::TURTLE_ZONE_2 if seconds_left > 530 => Some(AchievementId::TurtleTrot),
        _ => None,
    }
}

/// The zone-completion achievement for a golden-coin index, in ROM order.
fn zone_achievement(coin_index: u8) -> Option<AchievementId> {
    match coin_index {
        0 => Some(AchievementId::BranchingOut),
        1 => Some(AchievementId::TrickOrTreat),
        2 => Some(AchievementId::ItsAMe),
        3 => Some(AchievementId::ShellShock),
        4 => Some(AchievementId::WhereNoPlumberGoneBefore),
        5 => Some(AchievementId::TheBigHouse),
        _ => None,
    }
}

#[cfg(test)]
#[path = "achievements/tests.rs"]
mod tests;
