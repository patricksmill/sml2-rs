use super::*;
use crate::actor::Actor;
use crate::objects::{ObjectSim, Staging, B_FLAGS, B_FSTATE, B_SPRITE, B_T28, B_TYPE};

fn effects(flags: u8, primary_slot: bool, hits: u8, definition: u8) -> DamageEffects {
    damage(
        InteractionFlags::from_bits(flags),
        DamageContext {
            primary_slot,
            remaining_hits: hits,
            definition,
        },
    )
}

#[test]
fn native_boss_damage_source_boundaries() {
    for flags in [0, 4, 8, 0xfc] {
        assert_eq!(effects(flags, true, 1, 5), DamageEffects::default());
    }
    // Explicit source results, including DEC's distinct zero-count wrap.
    for (flags, counts) in [
        (
            1,
            [
                (0, 254, true),
                (1, 255, true),
                (2, 0, true),
                (3, 1, false),
                (255, 253, false),
            ],
        ),
        (
            2,
            [
                (0, 255, false),
                (1, 0, true),
                (2, 1, false),
                (3, 2, false),
                (255, 254, false),
            ],
        ),
    ] {
        for (before, after, defeated) in counts {
            for definition in [5, 6] {
                assert_eq!(
                    effects(flags, true, before, definition),
                    DamageEffects {
                        processed_hit: true,
                        latch_stomp: flags == 1,
                        remaining_hits: Some(after),
                        sound: Some(if flags == 1 {
                            DamageSound::Stomp
                        } else {
                            DamageSound::Projectile
                        }),
                        defeat: if !defeated {
                            None
                        } else if definition == 5 {
                            Some(Defeat::ClearWithFade)
                        } else {
                            Some(Defeat::Remove)
                        },
                    }
                );
            }
        }
    }
    for primary in [false, true] {
        assert_eq!(effects(3, primary, 3, 5), effects(1, primary, 3, 5));
    }
    for flags in [1, 2, 3] {
        assert_eq!(
            effects(flags, false, 0, 5),
            DamageEffects {
                processed_hit: true,
                latch_stomp: flags != 2,
                ..DamageEffects::default()
            }
        );
    }
}

// Persistent source destinations in source order; sentinels detect accidental
// clearing of fields that the selected source branch does not write.
fn world_bytes(sim: &ObjectSim) -> [u8; 9] {
    let fade = sim.fade_timer_set.unwrap().to_be_bytes();
    [
        sim.done,
        sim.boss_hit_latch,
        sim.boss_hits_remaining,
        sim.sfx0,
        sim.sfx1,
        sim.remove_mode,
        fade[0],
        fade[1],
        sim.boss_clear_flag,
    ]
}

fn fixture(flags: u8, primary: bool, hits: u8, definition: u8) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.done = 0x31;
    sim.boss_hit_latch = 0x32;
    sim.boss_hits_remaining = hits;
    sim.sfx0 = 0x33;
    sim.sfx1 = 0x34;
    sim.remove_mode = 0x35;
    sim.fade_timer_set = Some(0x3637);
    sim.boss_clear_flag = 0x38;
    sim.boss_def_index = definition;
    sim.boss_slot_ptr_lo = if primary { 0 } else { 0x20 };
    let mut st = Staging {
        b: std::array::from_fn(|i| (i as u8).wrapping_mul(7)),
        px: 0x1234,
        py: 0xabcd,
    };
    st.b[B_FLAGS] = flags;
    (sim, st)
}

#[test]
fn native_boss_damage_adapter_preserves_actor_and_unwritten_world_bytes() {
    for flags in [0, 4, 0xfc, 1, 2, 3] {
        for primary in [false, true] {
            for (hits, stomp_after, projectile_after) in [
                (0, 254, 255),
                (1, 255, 0),
                (2, 0, 1),
                (3, 1, 2),
                (255, 253, 254),
            ] {
                for definition in [5, 6] {
                    let (mut sim, mut st) = fixture(flags, primary, hits, definition);
                    let actor = Actor::from_raw_slot(&st.b);
                    let slot = st.b;
                    let mut expected = world_bytes(&sim);
                    if flags & 3 != 0 {
                        expected[0] = 1;
                        let stomp = flags & 1 != 0;
                        if stomp {
                            expected[1] = 1;
                        }
                        if primary {
                            expected[2] = if stomp { stomp_after } else { projectile_after };
                            expected[if stomp { 3 } else { 4 }] = if stomp { 19 } else { 9 };
                            if (stomp && hits <= 2) || (!stomp && hits == 1) {
                                expected[5] = 1;
                                if definition == 5 {
                                    expected[6..9].copy_from_slice(&[2, 128, 1]);
                                }
                            }
                        }
                    }
                    sim.boss_damage_pass(&mut st);
                    assert_eq!(
                        world_bytes(&sim),
                        expected,
                        "flags={flags} primary={primary} hits={hits} def={definition}"
                    );
                    assert_eq!(st.b, slot);
                    assert_eq!(Actor::from_raw_slot(&st.b), actor);
                    assert_eq!((st.px, st.py), (0x1234, 0xabcd));
                    assert_eq!(
                        (sim.boss_def_index, sim.boss_slot_ptr_lo),
                        (definition, if primary { 0 } else { 0x20 })
                    );
                }
            }
        }
    }
}

#[test]
fn native_boss_damage_projectile_at_zero_wraps_without_removal() {
    let (mut sim, mut st) = fixture(2, true, 0, 5);
    sim.done = 0;
    sim.boss_hit_latch = 0;
    sim.remove_mode = 0;
    sim.fade_timer_set = None;
    sim.boss_clear_flag = 0;
    sim.boss_damage_pass(&mut st);
    assert_eq!((sim.done, sim.boss_hits_remaining, sim.sfx1), (1, 255, 9));
    assert_eq!(
        (sim.remove_mode, sim.fade_timer_set, sim.boss_clear_flag),
        (0, None, 0)
    );
}

#[test]
fn native_boss_damage_existing_wario_caller_consumes_hit_effects() {
    let tables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let (mut sim, mut st) = fixture(3, true, 3, 6);
    sim.done = 0;
    sim.boss_hit_latch = 0;
    sim.boss_timer_reload = 0x71;
    st.b[B_SPRITE] = 0x4f;
    st.b[B_FSTATE] = 7;
    st.b[B_TYPE] = 0x41;
    let mut expected = st.b;
    expected[B_SPRITE] = 0x52;
    expected[B_T28] = 0x71;
    expected[B_FSTATE] = 1;
    expected[B_TYPE] = 0;
    sim.handler_wario20(&mut st, &tables);
    assert_eq!(st.b, expected);
    assert_eq!(
        (sim.spawn_sprite, sim.saved_frame_state, sim.saved_type),
        (0x4f, 7, 0x41)
    );
    assert_eq!(
        world_bytes(&sim),
        [1, 1, 1, 19, 0x34, 0x35, 0x36, 0x37, 0x38]
    );
}
