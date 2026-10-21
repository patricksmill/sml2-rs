use super::*;

/// `$5366..$5398`: a primary defeat waits 208 updates before publishing
/// remove mode 2. `$98C8` reads that byte later in the same slot pass to
/// turn the witch's central platform into the golden-coin reward.
#[test]
fn primary_fanfare_delays_the_witch_reward_for_all_208_updates() {
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
    let mut objects = ObjectSim::new(Vec::new(), 0);
    objects.configure(9);
    objects.spawn_boss(1);
    objects.remove_mode = 1; // `$92C4`: final damage has already landed.
    objects.slots[0].b[B_DEF] = 3;
    objects.slots[0].b[B_FSTATE] = 2;
    objects.slots[0].b[B_SCRY] = 208;
    objects.slots[2].b[B_FSTATE] = 2;
    objects.boss_engine(0, 0, &layout, &tables);
    assert_eq!(objects.slots[0].b[B_FSTATE], 3);
    assert_eq!(objects.slots[0].b[B_T28], 208);
    assert_eq!(objects.music_request, Some(12));

    for remaining in (1..=208).rev() {
        assert_eq!(objects.slots[0].b[B_T28], remaining);
        assert_eq!(objects.remove_mode, 1);
        assert_eq!(objects.slots[2].b[B_DEF], 8);
        assert_eq!(objects.slots[2].b[B_FSTATE], 2);
        assert_eq!(objects.slots[2].b[B_SCRY], 116);
        assert!(!objects.boss_clear_handoff);
        objects.boss_engine(0, 0, &layout, &tables);
    }
    assert_eq!(objects.slots[0].b, [0; 32]);
    assert_eq!(objects.remove_mode, 2);
    assert_eq!(objects.slots[2].b[B_DEF], 4);
    assert_eq!(objects.slots[2].b[B_FSTATE], 0);
    assert!(
        !objects.boss_clear_handoff,
        "the reward still has its own arc"
    );
    assert!(objects.unported.is_empty());
}
