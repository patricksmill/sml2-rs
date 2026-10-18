//! Source scene 12 in level $09 reaches definition 1 through runtime APIs.
use super::*;
use sml2_core::{
    camera::CameraSim,
    objects::{ObjectSim, B_DEF, B_FSTATE},
};

#[test]
fn door_boss_real_level_scene_entry_selects_portable_definition() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted/levels");
    let manifest: LevelManifest =
        serde_json::from_str(&fs::read_to_string(dir.join("level10.manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest.level_id, 9);
    let mut player = PlayerSim::new();
    // Direct placement at the source entrance cell: right edge of (14,1).
    player.x = 0x0EF1;
    player.y = 0x01D4;
    let mut camera = CameraSim::new(0x0EB0, 0x01C0);
    camera.prev_mario_y_lo = player.y as u8;
    camera
        .edge_map
        .copy_from_slice(&fs::read(dir.join("level10.edge.bin")).unwrap());
    camera.scene_map = fs::read(dir.join("level10.scene.bin")).unwrap();
    assert_eq!(camera.scene_map[30], 12);
    let scripts =
        SceneScripts::from_bytes(&fs::read(dir.join("../scenes/script_table.bin")).unwrap())
            .unwrap();
    assert_eq!(
        scripts
            .scripts()
            .into_iter()
            .find(|(id, _)| *id == 12)
            .unwrap()
            .1,
        vec![0x62, 0x20, 0x30, 0x58, 0x60, 0x40, 0, 0xFF]
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
    for _ in 0..128 {
        player.tick_fade_timer();
        camera.step(&mut player);
        if camera.scene_load_pending {
            let definition = boss_def_index(manifest.level_id, camera.boss_fight_phase);
            assert_eq!(definition, 1);
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
        if let SceneStep::Ran(op) = vm.step(&mut player, &mut camera, &scripts) {
            ran.push(op);
        }
        assert!(vm.unported().is_empty());
        assert!(objects.unported.is_empty());
        if entered {
            break;
        }
    }
    assert!(entered, "source entrance did not reach arena");
    assert_eq!(ran, vec![0x62, 0x20, 0x30, 0x40]);
    assert_eq!(player.level_gfx_index, 2);
    assert_eq!((player.music_request, player.level_song), (32, 32));
    assert!(camera.boss_mode());
    assert!(!camera.take_blocked_scene_load());
    assert_eq!(objects.boss_hits_remaining, 6);
    assert_eq!(objects.boss_def_count, 3);
    assert_eq!(
        objects.slots[..4]
            .iter()
            .map(|s| s.b[B_DEF])
            .collect::<Vec<_>>(),
        vec![6, 7, 8, 7]
    );
    assert_eq!(objects.slots[0].b[B_FSTATE], 0, "fade gates boss intro");
}
