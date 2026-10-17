use super::*;

fn dirty_bytes(seed: u8) -> [u8; 32] {
    std::array::from_fn(|i| seed.wrapping_add((i as u8).wrapping_mul(7)))
}

fn request() -> BossSpawnRequest {
    BossSpawnRequest {
        animation: 0x19,
        sprite: 0xFE,
        flip: SpriteFlipFlags::from_bits(0xA3),
    }
}

/// ROM writes definition at 5, copies five bytes from parent 6..=10, writes
/// sprite at 11, skips defeat at 12, then writes attributes at 13.
fn expected_child(mut child: [u8; 32], parent: &[u8; 32]) -> [u8; 32] {
    child[5] = 0x19;
    child[6..=10].copy_from_slice(&parent[6..=10]);
    child[11] = 0xFE;
    child[13] = 0xA3;
    child
}

#[test]
fn helper_0x0b224_native_dirty_pool_exact_byte_writes() {
    let parent_bytes = dirty_bytes(0x81);
    let parent = Actor::from_raw_slot(&parent_bytes);
    for initial_count in [0u8, 14, 15] {
        let original: [[u8; 32]; 16] = std::array::from_fn(|i| dirty_bytes(i as u8));
        let mut actors = original.map(|bytes| Actor::from_raw_slot(&bytes));
        let mut count = initial_count;
        let mut request = request();
        let result = spawn_child(&parent, &mut actors, &mut count, &mut request).unwrap();
        let mut expected = original;
        if initial_count < 15 {
            let index = usize::from(initial_count) + 1;
            expected[index] = expected_child(expected[index], &parent_bytes);
            assert_eq!(result, Some(index));
            assert_eq!(count, initial_count + 1);
        } else {
            assert_eq!(result, None);
            assert_eq!(count, 15);
        }
        assert_eq!(actors.map(|actor| actor.to_raw_slot()), expected);
        assert_eq!(parent.to_raw_slot(), parent_bytes);
        assert_eq!(request, BossSpawnRequest::default());
    }
}

#[test]
fn helper_0x0b224_native_invalid_count_is_explicit_and_preserves_all_inputs() {
    let original: [[u8; 32]; 16] = std::array::from_fn(|i| dirty_bytes(i as u8));
    let parent = Actor::from_raw_slot(&dirty_bytes(0x81));
    for invalid in 16..=255u8 {
        let mut actors = original.map(|bytes| Actor::from_raw_slot(&bytes));
        let mut count = invalid;
        let mut request = request();
        let original_request = request;
        assert_eq!(
            spawn_child(&parent, &mut actors, &mut count, &mut request),
            Err(BossSpawnError { count: invalid })
        );
        assert_eq!(count, invalid);
        assert_eq!(request, original_request);
        assert_eq!(actors.map(|actor| actor.to_raw_slot()), original);
    }
}

#[test]
fn helper_0x0b224_adapter_success_capacity_and_invalid_world_bytes() {
    for initial_count in [0u8, 14, 15, 16, 0xFE, 0xFF] {
        let mut sim = ObjectSim::new(vec![], 0);
        for (i, slot) in sim.slots.iter_mut().enumerate() {
            slot.b = dirty_bytes(i as u8);
        }
        sim.boss_def_count = initial_count;
        sim.spawn_anim = 0x19;
        sim.spawn_sprite = 0xFE;
        sim.spawn_attr = 0xA3;
        sim.done = 7;
        sim.boss_hits_remaining = 0x91;
        sim.boss_transition = 3;
        sim.slot_done = [true; 16];
        let original = sim.slots.clone().map(|slot| slot.b);
        let mut expected = original;
        let mut parent = Staging::default();
        parent.b = dirty_bytes(0x81);
        let parent_before = parent.b;
        if initial_count < 15 {
            let index = usize::from(initial_count) + 1;
            expected[index] = expected_child(expected[index], &parent.b);
        }
        sim.boss_spawn_child(&parent);
        assert_eq!(parent.b, parent_before);
        assert_eq!(sim.slots.clone().map(|slot| slot.b), expected);
        assert_eq!(
            (sim.done, sim.boss_hits_remaining, sim.boss_transition),
            (7, 0x91, 3)
        );
        assert_eq!(sim.slot_done, [true; 16]);
        assert!(sim.records.is_empty());
        if initial_count <= 15 {
            assert_eq!(
                sim.boss_def_count,
                if initial_count == 15 {
                    15
                } else {
                    initial_count + 1
                }
            );
            assert_eq!(
                (sim.spawn_anim, sim.spawn_sprite, sim.spawn_attr),
                (0, 0, 0)
            );
            assert!(sim.unported.is_empty());
        } else {
            assert_eq!(sim.boss_def_count, initial_count);
            assert_eq!(
                (sim.spawn_anim, sim.spawn_sprite, sim.spawn_attr),
                (0x19, 0xFE, 0xA3)
            );
            assert_eq!(sim.unported, vec![0xB224]);
        }
    }
}
