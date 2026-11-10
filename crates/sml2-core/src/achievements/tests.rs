//! Predicate tests for the native achievement tracker.
//!
//! Every boundary is exercised at the `observe` seam: one below the published
//! threshold fails, one above passes, and a duplicate event never re-emits.

use super::*;
use std::collections::BTreeSet;

fn level_clear(level_id: u8, powerup: u8, seconds_left: u16) -> GameEvent {
    GameEvent::LevelClear {
        level_id,
        powerup,
        seconds_left,
        checkpoint_used: false,
        secret_exit: false,
    }
}

fn boss(boss_id: u8, powerup: u8, checkpoint_used: bool, seconds_left: u16) -> GameEvent {
    GameEvent::BossDefeat {
        boss_id,
        powerup,
        checkpoint_used,
        seconds_left,
    }
}

fn observe_one(event: GameEvent) -> Vec<AchievementId> {
    AchievementTracker::new().observe(&event)
}

#[test]
fn ids_round_trip_and_unknown_ids_are_rejected() {
    for id in ALL_ACHIEVEMENTS.iter().copied() {
        assert_eq!(AchievementId::from_id(id.id()), Some(id));
        assert_eq!(AchievementId::try_from(id.id()), Ok(id));
        assert_eq!(u32::from(id), id.id());
    }
    assert_eq!(AchievementId::from_id(FIRST_ACHIEVEMENT_ID - 1), None);
    assert_eq!(AchievementId::from_id(LAST_ACHIEVEMENT_ID + 1), None);
    assert_eq!(AchievementId::from_id(0), None);
    assert!(AchievementId::try_from(0).is_err());
}

#[test]
fn speed_run_thresholds_are_strictly_over_the_listed_value() {
    let cases = [
        (level_id::TREE_ZONE_3, 340, AchievementId::TreeTrek),
        (
            level_id::PUMPKIN_ZONE_1,
            445,
            AchievementId::PumpkinPromenade1,
        ),
        (
            level_id::PUMPKIN_ZONE_2,
            320,
            AchievementId::PumpkinPromenade2,
        ),
        (level_id::SPACE_ZONE_1, 625, AchievementId::SpaceSprint),
        (level_id::MACRO_ZONE_3, 330, AchievementId::MacroMarch),
        (level_id::MARIO_ZONE_1, 340, AchievementId::GoGoMario),
        (level_id::TURTLE_ZONE_2, 530, AchievementId::TurtleTrot),
    ];
    for (level, threshold, achievement) in cases {
        assert!(
            observe_one(level_clear(level, POWERUP_SMALL, threshold)).is_empty(),
            "{achievement:?} must not unlock at exactly {threshold}"
        );
        assert_eq!(
            observe_one(level_clear(level, POWERUP_SMALL, threshold + 1)),
            vec![achievement],
            "{achievement:?} unlocks one second over the threshold"
        );
    }
}

#[test]
fn speed_runs_require_small_mario_and_the_right_level() {
    assert!(observe_one(level_clear(level_id::MARIO_ZONE_1, POWERUP_SMALL + 1, 500)).is_empty());
    assert!(observe_one(level_clear(level_id::MARIO_ZONE_2, POWERUP_SMALL, 999)).is_empty());
    assert!(observe_one(level_clear(level_id::TREE_ZONE_1, POWERUP_SMALL, 999)).is_empty());
    assert!(observe_one(level_clear(level_id::MUSHROOM_HOUSE, POWERUP_SMALL, 999)).is_empty());
}

#[test]
fn speed_run_duplicates_are_suppressed() {
    let mut tracker = AchievementTracker::new();
    let event = level_clear(level_id::TREE_ZONE_3, POWERUP_SMALL, 341);
    assert_eq!(tracker.observe(&event), vec![AchievementId::TreeTrek]);
    assert!(tracker.observe(&event).is_empty());
    assert!(tracker.is_unlocked(AchievementId::TreeTrek));
}

#[test]
fn each_boss_maps_to_its_challenge_and_needs_no_checkpoint() {
    let cases = [
        (boss_id::BIG_BIRD, AchievementId::BigBirdsChallenge),
        (boss_id::WITCH, AchievementId::WitchsChallenge),
        (boss_id::THREE_PIGS, AchievementId::ThreeLittlePigsChallenge),
        (boss_id::OCTOPUS, AchievementId::OctopusChallenge),
        (boss_id::TATANGA, AchievementId::TatangasChallenge),
        (boss_id::SEWER_RAT, AchievementId::SewerRatsChallenge),
    ];
    for (boss_id, achievement) in cases {
        assert_eq!(
            observe_one(boss(boss_id, POWERUP_SMALL, false, 0)),
            vec![achievement],
            "boss {boss_id}"
        );
        assert!(
            observe_one(boss(boss_id, POWERUP_SMALL, true, 0)).is_empty(),
            "boss {boss_id} with a checkpoint"
        );
        assert!(
            observe_one(boss(boss_id, POWERUP_SMALL + 1, false, 0)).is_empty(),
            "boss {boss_id} as powered-up Mario"
        );
    }
}

#[test]
fn wario_defeat_unlocks_the_castle_and_time_challenge() {
    assert_eq!(
        observe_one(boss(boss_id::WARIO, POWERUP_SMALL, false, 651)),
        vec![
            AchievementId::WarioIsNotWelcome,
            AchievementId::WariosChallenge
        ]
    );
    assert_eq!(
        observe_one(boss(boss_id::WARIO, POWERUP_SMALL, false, 650)),
        vec![AchievementId::WarioIsNotWelcome]
    );
    assert_eq!(
        observe_one(boss(boss_id::WARIO, POWERUP_SMALL + 1, false, 999)),
        vec![AchievementId::WarioIsNotWelcome]
    );
    assert_eq!(
        observe_one(GameEvent::WarioDefeated),
        vec![AchievementId::WarioIsNotWelcome]
    );

    // The castle has no goal bell in the ROM, but accept a level clear too.
    assert_eq!(
        observe_one(level_clear(level_id::WARIO_CASTLE, POWERUP_SMALL, 651)),
        vec![AchievementId::WariosChallenge]
    );

    let mut tracker = AchievementTracker::new();
    assert_eq!(
        tracker.observe(&GameEvent::WarioDefeated),
        vec![AchievementId::WarioIsNotWelcome]
    );
    assert!(tracker.observe(&GameEvent::WarioDefeated).is_empty());
}

#[test]
fn golden_coins_unlock_their_zone_in_rom_order() {
    let expected = [
        (0, AchievementId::BranchingOut),
        (1, AchievementId::TrickOrTreat),
        (2, AchievementId::ItsAMe),
        (3, AchievementId::ShellShock),
        (4, AchievementId::WhereNoPlumberGoneBefore),
        (5, AchievementId::TheBigHouse),
    ];
    let mut tracker = AchievementTracker::new();
    for (index, achievement) in expected {
        let event = GameEvent::GoldenCoinAwarded { coin_index: index };
        assert_eq!(tracker.observe(&event), vec![achievement], "coin {index}");
        assert!(tracker.observe(&event).is_empty(), "coin {index} duplicate");
    }
    assert!(tracker
        .observe(&GameEvent::GoldenCoinAwarded { coin_index: 6 })
        .is_empty());
}

#[test]
fn all_golden_coins_awards_the_landlord_once() {
    let mut tracker = AchievementTracker::new();
    assert_eq!(
        tracker.observe(&GameEvent::AllGoldenCoinsHeld),
        vec![AchievementId::HereComesTheLandlord]
    );
    assert!(tracker.observe(&GameEvent::AllGoldenCoinsHeld).is_empty());
}

#[test]
fn master_of_mario_land_needs_31_decimal_exits() {
    assert!(observe_one(GameEvent::ExitCountUpdated { tally: 29 }).is_empty());
    assert!(observe_one(GameEvent::ExitCountUpdated { tally: 30 }).is_empty());
    assert_eq!(
        observe_one(GameEvent::ExitCountUpdated { tally: 31 }),
        vec![AchievementId::MasterOfMarioLand]
    );
}

#[test]
fn prosperous_plumber_needs_the_counter_to_reach_999() {
    assert!(observe_one(GameEvent::CoinsUpdated { total_coins: 998 }).is_empty());
    let mut tracker = AchievementTracker::new();
    assert_eq!(
        tracker.observe(&GameEvent::CoinsUpdated { total_coins: 999 }),
        vec![AchievementId::ProsperousPlumber]
    );
    assert!(tracker
        .observe(&GameEvent::CoinsUpdated { total_coins: 999 })
        .is_empty());
}

#[test]
fn heart_to_heart_counts_lives_and_99up_poisons_the_run() {
    assert!(observe_one(GameEvent::LivesEarned {
        new_lives: 19,
        is_99up: false
    })
    .is_empty());
    assert_eq!(
        observe_one(GameEvent::LivesEarned {
            new_lives: 20,
            is_99up: false
        }),
        vec![AchievementId::HeartToHeart]
    );

    // A 99-up never satisfies the criterion. The inflated baseline stays
    // disqualified for the rest of this tracker, so even a later ordinary gain
    // to 20 or above must not unlock it.
    let mut tracker = AchievementTracker::new();
    assert!(tracker
        .observe(&GameEvent::LivesEarned {
            new_lives: 99,
            is_99up: true
        })
        .is_empty());
    for new_lives in [20, 21] {
        assert!(
            tracker
                .observe(&GameEvent::LivesEarned {
                    new_lives,
                    is_99up: false
                })
                .is_empty(),
            "ordinary gain to {new_lives} after a 99-up"
        );
    }
    assert!(!tracker.is_unlocked(AchievementId::HeartToHeart));

    // An increment that jumps over 20 also counts, and never re-emits.
    let mut tracker = AchievementTracker::new();
    assert_eq!(
        tracker.observe(&GameEvent::LivesEarned {
            new_lives: 42,
            is_99up: false
        }),
        vec![AchievementId::HeartToHeart]
    );
    assert!(tracker
        .observe(&GameEvent::LivesEarned {
            new_lives: 10,
            is_99up: false
        })
        .is_empty());

    // A 99-up after the achievement is already earned does not relock it.
    assert!(tracker
        .observe(&GameEvent::LivesEarned {
            new_lives: 99,
            is_99up: true
        })
        .is_empty());
    assert!(tracker.is_unlocked(AchievementId::HeartToHeart));
}

#[test]
fn inflating_the_counter_with_a_99up_cannot_earn_heart_to_heart() {
    // Main's review case: after the 99-up, an ordinary gain reports an
    // inflated total (98/99). It must not be credited as 20 earned lives.
    let mut tracker = AchievementTracker::new();
    assert!(tracker
        .observe(&GameEvent::LivesEarned {
            new_lives: 99,
            is_99up: true
        })
        .is_empty());
    for new_lives in [98, 99] {
        assert!(
            tracker
                .observe(&GameEvent::LivesEarned {
                    new_lives,
                    is_99up: false
                })
                .is_empty(),
            "inflated ordinary total {new_lives} must stay locked"
        );
    }
    assert!(!tracker.is_unlocked(AchievementId::HeartToHeart));
}

#[test]
fn make_em_see_stars_counts_99_enemies_once() {
    let mut tracker = AchievementTracker::new();
    let mut unlocked = Vec::new();
    for count in 1..=99 {
        let newly = tracker.observe(&GameEvent::EnemyDefeated);
        if count == 99 {
            assert_eq!(newly, vec![AchievementId::MakeEmSeeStars]);
            unlocked = newly;
        } else {
            assert!(newly.is_empty(), "enemy {count}");
        }
    }
    assert_eq!(unlocked, vec![AchievementId::MakeEmSeeStars]);
    assert!(tracker.observe(&GameEvent::EnemyDefeated).is_empty());
}

#[test]
fn six_unique_secret_levels_are_required() {
    let mut tracker = AchievementTracker::new();
    // Repeating one secret course never advances the count.
    let turtle = level_clear(level_id::SECRET_TURTLE, POWERUP_SMALL, 1);
    for _ in 0..6 {
        assert!(tracker.observe(&turtle).is_empty());
    }
    for level in [
        level_id::SECRET_PUMPKIN_4,
        level_id::SECRET_SPACE,
        level_id::SECRET_TREE,
        level_id::SECRET_MACRO,
    ] {
        assert!(tracker
            .observe(&level_clear(level, POWERUP_SMALL, 1))
            .is_empty());
    }
    assert_eq!(
        tracker.observe(&level_clear(level_id::SECRET_PUMPKIN_5, POWERUP_SMALL, 1)),
        vec![AchievementId::ShhhDontTell]
    );
    assert!(tracker.observe(&turtle).is_empty());

    // Non-secret variants and ordinary levels do not count.
    let mut tracker = AchievementTracker::new();
    for level in [
        level_id::TUTORIAL_SECRET_VARIANT,
        level_id::TURTLE_SECRET_VARIANT,
        level_id::TREE_ZONE_1,
        level_id::PUMPKIN_ZONE_1,
    ] {
        assert!(!is_secret_level(level));
        assert!(tracker
            .observe(&level_clear(level, POWERUP_SMALL, 1))
            .is_empty());
    }
    for level in SECRET_LEVEL_IDS {
        assert!(is_secret_level(level));
    }
}

#[test]
fn totaka_song_unlocks_misery_loves_melody_once() {
    let mut tracker = AchievementTracker::new();
    assert_eq!(
        tracker.observe(&GameEvent::TotakaSongPlayed),
        vec![AchievementId::MiseryLovesMelody]
    );
    assert!(tracker.observe(&GameEvent::TotakaSongPlayed).is_empty());
}

#[test]
fn restore_unlocked_prevents_duplicate_event_grants() {
    let mut tracker = AchievementTracker::new();
    tracker.restore_unlocked([AchievementId::GoGoMario, AchievementId::HeartToHeart]);
    assert!(tracker.is_unlocked(AchievementId::GoGoMario));
    assert!(tracker.is_unlocked(AchievementId::HeartToHeart));
    assert_eq!(tracker.unlocked().len(), 2);
    assert!(tracker
        .observe(&level_clear(level_id::MARIO_ZONE_1, POWERUP_SMALL, 500))
        .is_empty());
    assert!(tracker
        .observe(&GameEvent::LivesEarned {
            new_lives: 30,
            is_99up: false
        })
        .is_empty());
}

#[test]
fn restore_unlocked_ids_ignores_unknown_values_and_reports_recognised_entries() {
    let mut tracker = AchievementTracker::new();
    let recognised = tracker.restore_unlocked_ids([
        AchievementId::BranchingOut.id(),
        0,
        AchievementId::TreeTrek.id(),
        u32::MAX,
        AchievementId::TreeTrek.id(),
    ]);
    assert_eq!(recognised, 3);
    let expected: BTreeSet<AchievementId> = [AchievementId::BranchingOut, AchievementId::TreeTrek]
        .into_iter()
        .collect();
    assert_eq!(tracker.unlocked(), &expected);
    assert_eq!(tracker.restore_unlocked_ids([0, 42]), 0);
}
