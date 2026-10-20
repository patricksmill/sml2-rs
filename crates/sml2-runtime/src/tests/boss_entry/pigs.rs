//! Source scene 16 in level $0D reaches definition 2 through runtime APIs.
use super::*;
use sml2_core::{
    camera::CameraSim,
    objects::{ObjectSim, B_DEF, B_FSTATE},
};

#[test]
fn pigs_boss_real_level_scene_entry_selects_portable_definition() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted/levels");
    let manifest: LevelManifest =
        serde_json::from_str(&fs::read_to_string(dir.join("level14.manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest.level_id, 13);
    let mut player = PlayerSim::new();
    seed_level_timer(&mut player, &manifest);
    // Direct placement at the source entrance cell: right edge of (14,0).
    player.x = 0x0EF1;
    player.y = 0x00D4;
    let mut camera = CameraSim::new(0x0EB0, 0x0000);
    camera.prev_mario_y_lo = player.y as u8;
    camera.update_enables(player.x); // Initial snapshot includes the preceding draw.
    camera
        .edge_map
        .copy_from_slice(&fs::read(dir.join("level14.edge.bin")).unwrap());
    camera.scene_map = fs::read(dir.join("level14.scene.bin")).unwrap();
    assert_eq!(camera.scene_map[14], 16);
    let scripts =
        SceneScripts::from_bytes(&fs::read(dir.join("../scenes/script_table.bin")).unwrap())
            .unwrap();
    assert_eq!(
        scripts
            .scripts()
            .into_iter()
            .find(|(id, _)| *id == 16)
            .unwrap()
            .1,
        vec![0x61, 0x20, 0x30, 0x60, 0x68, 0x40, 5, 0xFF]
    );
    let mut vm = SceneVm::default();
    let mut objects = ObjectSim::new(vec![], camera.x);
    objects.configure(manifest.level_id);
    let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap();
    let arena = LevelLayout::from_bytes(&fs::read(dir.join("../scenes/arena_layout.bin")).unwrap())
        .unwrap();
    let mut ran = vec![];
    let mut entered = false;
    let mut ending = EndingEntry::default();
    let mut owned_frames = 0;
    for _ in 0..128 {
        let owned = vm.owns_gameplay();
        let before_motion = (player.x, player.y, camera.x, camera.y, camera.transition);
        let before_timer = player.level_timer;
        let before_fade = player.fade_timer;
        let before_clock = (
            objects.phase,
            objects.odd,
            objects.fff0,
            objects.update_phase,
        );
        let before_slots = objects.slots.each_ref().map(|slot| slot.b);
        let frame = scene_frame_head(
            &mut vm,
            &mut ending,
            &mut player,
            &mut camera,
            EndingInput::default(),
        );
        assert_eq!(frame.is_none(), owned, "scene owns gameplay dispatch");
        let gameplay = matches!(frame, Some(EndingFrame::Gameplay(_)));
        if gameplay {
            player.tick_fade_timer();
            camera.step(&mut player);
            camera.update_enables(player.x); // `$54A7`: draw publishes the next edge latch.
            if camera.scene_load_pending {
                let definition = boss_def_index(manifest.level_id, camera.boss_fight_phase);
                assert_eq!(definition, 2);
                assert!(boss_def_portable(definition));
                let (music, song) = camera.begin_scene_load(manifest.level_id).unwrap();
                player.music_request = music;
                player.level_song = song;
                objects.spawn_boss(definition);
                entered = true;
            }
            if entered {
                objects.sample_player_inputs(&player);
                objects.step(
                    camera.x,
                    camera.y,
                    &arena,
                    &tables,
                    &build_contact(&player, &PlayerKeys::default(), 0),
                    camera.boss_mode(),
                );
                player.apply_object_result(&mut objects, false);
                camera.apply_object_result(&mut objects);
            }
        }
        if owned {
            owned_frames += 1;
            assert_eq!(
                (player.x, player.y, camera.x, camera.y, camera.transition),
                before_motion,
                "scene waits freeze normal motion and camera transition",
            );
            assert_eq!(
                player.level_timer, before_timer,
                "scene waits freeze countdown"
            );
            assert_eq!(
                player.fade_timer,
                before_fade.saturating_sub(1),
                "VBlank ticks fade once"
            );
            assert_eq!(
                (
                    objects.phase,
                    objects.odd,
                    objects.fff0,
                    objects.update_phase
                ),
                before_clock,
                "scene waits freeze the object clock",
            );
            assert_eq!(objects.slots.each_ref().map(|slot| slot.b), before_slots);
        }
        let step = if gameplay || frame.is_none() {
            vm.step(&mut player, &mut camera, &scripts)
        } else {
            SceneStep::Idle
        };
        if let SceneStep::Ran(op) = step {
            ran.push(op);
        }
        assert!(vm.unported().is_empty());
        assert!(objects.unported.is_empty());
        if entered {
            break;
        }
    }
    assert!(owned_frames > 0, "entry must exercise scene-owned frames");
    assert!(entered, "source entrance did not reach arena");
    assert_eq!(ran, vec![0x61, 0x20, 0x30, 0x40]);
    assert_eq!(player.level_gfx_index, 1);
    assert_eq!((player.music_request, player.level_song), (32, 32));
    assert!(camera.boss_mode());
    assert!(!camera.take_blocked_scene_load());
    assert_eq!(objects.boss_hits_remaining, 6);
    assert_eq!(objects.boss_def_count, 2);
    assert_eq!(
        objects.slots[..3]
            .iter()
            .map(|s| s.b[B_DEF])
            .collect::<Vec<_>>(),
        vec![12, 11, 10]
    );
    assert_eq!(objects.slots[0].b[B_FSTATE], 0, "fade gates boss intro");
}
