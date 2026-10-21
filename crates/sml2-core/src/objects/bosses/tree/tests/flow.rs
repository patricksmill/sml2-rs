//! Definition-0 arena lifecycle through collision, dispatch and map handoff.
use super::*;
use crate::{camera::CameraSim, player_sim::PlayerSim};

#[test]
fn tree_arena_all_divider_choices_defeat_reward_map_and_reentry() {
    assert!(crate::scene::boss_def_portable(0));
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
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
    for divider in 0..=3 {
        let mut player = PlayerSim::new();
        player.map_node = 0x20;
        let mut camera = CameraSim::new(76, 72);
        camera.scene_state = 1;
        camera.transition = 1;
        camera.step(&mut player);
        assert_eq!(camera.begin_scene_load(0x05), Some((32, 32)));
        let mut objects = ObjectSim::new(vec![], 80);
        objects.configure(0x05);
        objects.spawn_boss(crate::scene::boss_def_index(0x05, 0));
        objects.div = divider;
        let mut hits = 0;
        let mut sweep_states = [false; 5];
        let mut reward_seen = false;
        for frame in 0..6000 {
            player.tick_fade_timer();
            let boss = &objects.slots[0].b;
            // Let the source boss complete a sweep cycle before attacking.
            // Direct placement exercises the actual collision/damage path.
            let attack = frame >= 900 && boss[B_TYPE] == 69 && boss[B_DEFEAT] == 0 && hits < 3;
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
            if old_hits != objects.boss_hits_remaining {
                hits += 1;
                assert_eq!(objects.boss_hits_remaining, 6 - hits * 2);
                assert_eq!(
                    (objects.slots[0].b[B_FSTATE], objects.slots[0].b[B_SPRITE]),
                    (1, 0)
                );
            }
            for (seen, state) in sweep_states.iter_mut().zip(3..=7) {
                *seen |= objects.slots[0].b[B_FSTATE] == state;
            }
            // `$B224` appends after both initial definitions; anim 2 stays live.
            reward_seen |= objects.slots[2].b[B_DEF] == 4;
            player.apply_object_result(&mut objects, false);
            camera.apply_object_result(&mut objects);
            assert!(
                objects.unported.is_empty(),
                "DIV={divider}, frame={frame}: {:?}",
                objects.unported
            );
            if player.exit_completed {
                break;
            }
        }
        assert_eq!(hits, 3, "DIV={divider}");
        assert_eq!(
            sweep_states, [true; 5],
            "DIV={divider}: sweep state missing"
        );
        assert!(reward_seen, "DIV={divider}: reward never spawned");
        assert!(
            player.exit_completed,
            "DIV={divider}: terminal handoff missing"
        );
        assert_eq!(objects.remove_mode, 2);
        assert_eq!(
            (
                player.level_progress[5] & 0x80,
                player.golden_coin_count(),
                player.level_tally
            ),
            (0x80, 1, 1)
        );
        let handoff = player.map_handoff(&nodes);
        assert_eq!(
            (handoff.node, handoff.screen, handoff.saves),
            (0x19, Some(0), 1)
        );
        let record = player.save_record(0);
        assert_eq!(
            record[8 + 5] & 0x80,
            0x80,
            "reward survives save serialization"
        );
        player.apply_object_result(&mut objects, false);
        assert_eq!(player.level_tally, 1, "consumed reward cannot award twice");
        // Reuse the runtime's player/object/camera reset seams.
        player.respawn(46, 708);
        objects.reset_level_start();
        camera.reset_level_start(76, 72, player.y as u8);
        assert!(!camera.boss_mode());
        assert!(!player.boss_clear, "fresh level countdown must resume");
        assert_eq!(
            objects.boss_clear_flag, 0,
            "fresh objects cannot reapply a stale clear"
        );
        assert_eq!(objects.scene_slot_parity, 0);
        assert!(objects.slots.iter().all(|s| s.b == [0; 32]));
        assert_eq!(player.level_progress[5] & 0x80, 0x80);
        camera.scene_state = 1;
        camera.transition = 1;
        camera.step(&mut player);
        assert_eq!(camera.begin_scene_load(0x05), Some((32, 32)));
        objects.spawn_boss(0);
        assert_eq!(
            (objects.boss_hits_remaining, objects.boss_def_count),
            (6, 1)
        );
        assert_eq!(
            objects.slots[..2]
                .iter()
                .map(|s| s.b[B_DEF])
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        objects.sample_player_inputs(&player);
        objects.boss_engine(camera.x, camera.y, &layout, &tables);
        player.apply_object_result(&mut objects, false);
        assert!(!player.boss_clear);
        assert_eq!(objects.slots[0].b[B_FSTATE], 3);
        assert!(objects.unported.is_empty());
    }
}
