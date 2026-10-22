use crate::camera::CameraSim;
use crate::player_sim::PlayerSim;
use crate::scene::{SceneScripts, SceneStep, SceneVm};

#[test]
fn tile_transfer_waits_match_rom_lengths_without_marking_a_screen_transition() {
    for (opcode, bursts) in [(0xB0, 14), (0xB1, 24), (0xB2, 14), (0xB3, 16), (0xBF, 14)] {
        let mut blob = vec![0; SceneScripts::BLOB_LEN];
        blob[2..4].copy_from_slice(&0x4165u16.to_le_bytes());
        blob[0x40..0x45].copy_from_slice(&[opcode, 0x1E, 0, 0x6C, 0xFF]);
        let scripts = SceneScripts::from_bytes(&blob).unwrap();
        let mut vm = SceneVm::new();
        let mut cam = CameraSim::new(1616, 704);
        cam.scene_id = 1;
        let mut sim = PlayerSim::new();
        sim.x = 1536;
        sim.y = 724;
        assert_eq!(vm.step(&mut sim, &mut cam, &scripts), SceneStep::Entering);
        assert_eq!(
            vm.step(&mut sim, &mut cam, &scripts),
            SceneStep::Unported(opcode)
        );
        assert_eq!(vm.pc.to_le_bytes(), [0x69, 0x41]);
        assert_eq!(vm.opcode_wait, bursts);
        for remaining in (0..bursts).rev() {
            assert_eq!(vm.step(&mut sim, &mut cam, &scripts), SceneStep::Entering);
            assert!(vm.owns_gameplay());
            assert_eq!(vm.opcode_wait, remaining);
            assert_eq!([cam.scene_id, cam.scene_transition_flag], [1, 0]);
            assert_eq!((sim.x, sim.y, cam.x, cam.y), (1536, 724, 1616, 704));
        }
        assert_eq!(vm.step(&mut sim, &mut cam, &scripts), SceneStep::End);
        assert!(!vm.owns_gameplay());
        assert_eq!(
            [
                cam.scene_id,
                cam.scene_transition_flag,
                cam.scene_transition_latch
            ],
            [0, 0, 0]
        );
        assert_eq!(vm.unported(), &[opcode]);
    }
}

#[test]
fn castle_scene4_keeps_gameplay_blocked_for_all_fifteen_transfer_waits() {
    let scripts =
        SceneScripts::from_bytes(include_bytes!("../../../extracted/scenes/script_table.bin"))
            .unwrap();
    let mut vm = SceneVm::new();
    let mut cam = CameraSim::new(1616, 704);
    cam.scene_id = 4;
    cam.transition = 2;
    let mut sim = PlayerSim::new();
    sim.x = 1536;
    sim.y = 724;
    assert_eq!(vm.step(&mut sim, &mut cam, &scripts), SceneStep::Entering);
    assert_eq!(
        vm.step(&mut sim, &mut cam, &scripts),
        SceneStep::Unported(0xB2)
    );
    for _ in 0..14 {
        assert!(vm.owns_gameplay());
        assert_eq!(vm.step(&mut sim, &mut cam, &scripts), SceneStep::Entering);
        assert_eq!(sim.x.to_le_bytes(), [0, 6]);
        assert_eq!(sim.y.to_le_bytes(), [0xD4, 2]);
        assert_eq!(
            [cam.scene_id, cam.transition, cam.scene_transition_flag],
            [4, 2, 0]
        );
    }
    assert_eq!(vm.step(&mut sim, &mut cam, &scripts), SceneStep::End);
    assert_eq!(vm.pc.to_le_bytes(), [0x74, 0x41]);
    assert!(!vm.owns_gameplay());
    assert_eq!(cam.scene_transition_latch, 0);
}

#[test]
fn tile_transfer_copies_one_burst_per_wait_into_bg_and_obj_tile_views() {
    use crate::scene::{SceneTransferAssets, SceneTransferBlock};
    let mut blob = vec![0; SceneScripts::BLOB_LEN];
    blob[2..4].copy_from_slice(&0x4165u16.to_le_bytes());
    blob[0x40..0x45].copy_from_slice(&[0xB2, 0x1E, 0, 0x6C, 0xFF]);
    let mut scripts = SceneScripts::from_bytes(&blob).unwrap();
    let bytes: Vec<u8> = (0..0x380u16).map(|i| (i * 7 + 1) as u8).collect();
    scripts.set_transfer_assets(SceneTransferAssets {
        schema: "sml2-scene-transfers/1".into(),
        blocks: vec![SceneTransferBlock {
            bank: 0x1E,
            address: 0x6C00,
            bytes: bytes.clone(),
        }],
    });
    let mut vm = SceneVm::new();
    let mut cam = CameraSim::new(1616, 704);
    cam.scene_id = 1;
    let mut sim = PlayerSim::new();
    let (mut bg, mut obj) = (vec![0u8; 0x1000], vec![0u8; 0x1000]);
    assert_eq!(vm.step(&mut sim, &mut cam, &scripts), SceneStep::Entering);
    assert!(vm.take_transfer_write().is_none());
    assert_eq!(vm.step(&mut sim, &mut cam, &scripts), SceneStep::Ran(0xB2));
    let mut bursts = 0;
    loop {
        if let Some(write) = vm.take_transfer_write() {
            assert_eq!(write.address, 0x8B00 + bursts * 0x40);
            assert_eq!(write.bytes.len(), 0x40);
            write.apply_tile_blocks(&mut bg, Some(&mut obj)).unwrap();
            bursts += 1;
        }
        if vm.step(&mut sim, &mut cam, &scripts) == SceneStep::End {
            break;
        }
    }
    assert_eq!(bursts, 14);
    assert_eq!(&bg[0x300..0x680], &bytes[..]);
    assert_eq!(&obj[0xB00..0xE80], &bytes[..]);
    assert!(bg[..0x300].iter().chain(&bg[0x680..]).all(|&b| b == 0));
    assert!(obj[..0xB00].iter().chain(&obj[0xE80..]).all(|&b| b == 0));
    assert!(vm.unported().is_empty());
}
