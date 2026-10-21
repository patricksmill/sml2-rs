//! Definition-1 lifecycle via real collision, boss dispatch and player handoff.
use super::*;
use crate::{camera::CameraSim, player_sim::PlayerSim};

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

#[test]
fn door_boss_all_phases_stomp_defeat_reward_and_map_handoff() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/scenes/arena_layout.bin"
        ))
        .unwrap(),
    )
    .unwrap();
    let nodes = crate::map::MapNodes::from_bytes(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/map/nodes.bin"
        ))
        .unwrap(),
    )
    .unwrap();
    for phase in 0..=3 {
        assert!(crate::scene::boss_def_portable(1));
        let mut player = PlayerSim::new();
        player.map_node = 0x2B;
        let mut camera = CameraSim::new(76, 72);
        camera.scene_state = 1;
        camera.transition = 1;
        camera.step(&mut player);
        assert!(camera.scene_load_pending);
        assert_eq!(camera.begin_scene_load(9), Some((32, 32)));
        assert!(camera.boss_mode());
        let mut objects = ObjectSim::new(vec![], 80);
        objects.configure(9);
        objects.spawn_boss(crate::scene::boss_def_index(9, 0));
        objects.div = phase;
        let mut hits = 0;
        let mut sequence_seen = false;
        let mut reward_seen = false;
        for frame in 0..6000 {
            player.tick_fade_timer();
            // Direct placement at an open boss's top edge exercises `$2D7D`
            // collision and `$9285` damage without manufacturing hit flags.
            let boss = &objects.slots[0].b;
            let attack = frame >= 350 && boss[B_TYPE] == 69 && hits < 3;
            let (sx, sy) = if attack {
                (
                    boss[B_SCRX],
                    boss[B_SCRY].wrapping_add(boss[B_HB_TOP]).wrapping_sub(20),
                )
            } else {
                (0, 0)
            };
            player.x = camera.x.wrapping_sub(96).wrapping_add(u16::from(sx));
            player.y = camera.y.wrapping_sub(98).wrapping_add(u16::from(sy));
            let contact = MarioContact {
                x: player.x,
                y: player.y,
                vvel: 0,
                invincibility: 255,
                ..Default::default()
            };
            objects.sample_player_inputs(&player);
            let old_hits = objects.boss_hits_remaining;
            objects.step(
                camera.x,
                camera.y,
                &layout,
                &tables,
                &contact,
                camera.boss_mode(),
            );
            if objects.boss_hits_remaining != old_hits {
                hits += 1;
                assert_eq!(objects.boss_hits_remaining, 6 - hits * 2);
                assert_eq!(objects.slots[0].b[B_SPRITE], 17);
            }
            sequence_seen |= objects.door_sequence_active != 0;
            reward_seen |= objects.slots.iter().any(|s| s.b[B_DEF] == 4);
            player.apply_object_result(&mut objects, false);
            camera.apply_object_result(&mut objects);
            assert!(
                objects.unported.is_empty(),
                "phase {phase}, frame {frame}: {:?}",
                objects.unported
            );
            if player.exit_completed {
                break;
            }
        }
        assert_eq!(hits, 3, "phase {phase}");
        assert_eq!(sequence_seen, phase != 0, "phase {phase}");
        assert!(reward_seen, "phase {phase}: reward never spawned");
        assert!(
            player.exit_completed,
            "phase {phase}: reward never handed off, slots {:?}",
            objects
                .slots
                .iter()
                .map(|s| (s.b[B_DEF], s.b[B_FSTATE], s.b[B_SCRY]))
                .collect::<Vec<_>>()
        );
        assert_eq!(objects.remove_mode, 2);
        assert_eq!(player.level_progress[9] & 0x80, 0x80);
        assert_eq!(player.golden_coin_count(), 1);
        assert_eq!(player.level_tally, 1);
        let handoff = player.map_handoff(&nodes);
        assert_eq!(handoff.node, 0x1C);
        assert_eq!(handoff.screen, Some(0));
        assert_eq!(handoff.saves, 1);
        assert!(player.map_handoff_done);
        player.apply_object_result(&mut objects, false);
        assert_eq!(player.level_tally, 1);
        // The same objects and camera are reset and reused on reentry.
        objects.reset_level_start();
        camera.reset_level_start(76, 72, 72);
        assert!(!camera.boss_mode());
        assert_eq!(objects.door_sequence_active, 0);
        assert!(objects.slots.iter().all(|s| s.b == [0; 32]));
        camera.scene_state = 1;
        camera.transition = 1;
        camera.step(&mut player);
        assert_eq!(camera.begin_scene_load(9), Some((32, 32)));
        objects.spawn_boss(1);
        assert_eq!(objects.boss_hits_remaining, 6);
        assert_eq!(objects.boss_def_count, 3);
        assert_eq!(objects.slots[2].b[B_DEF], 8);
        assert_eq!(objects.slots[2].b[B_FSTATE], 0);
    }
}
