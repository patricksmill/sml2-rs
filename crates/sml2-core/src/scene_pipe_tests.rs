use super::*;

#[test]
fn pipe_scene_waits_match_direction_streamers_and_keep_cpu_ownership() {
    let scripts =
        SceneScripts::from_bytes(include_bytes!("../../../extracted/scenes/script_table.bin"))
            .unwrap();
    // $2323 own wait + streamer waits + $21E8 tail wait. $246F (down)
    // streams four rows (three waits); other directions stream three strips.
    for (direction, waits) in [(0, 2), (1, 4), (2, 4), (4, 4), (8, 5)] {
        let mut scene = SceneVm::new();
        let mut cam = CameraSim::new(0x1B0, 0xC0);
        cam.transition = direction;
        cam.scene_id = 17;
        let mut sim = PlayerSim::new();
        sim.x = 0x1F1;
        sim.y = 0xA4;
        assert_eq!(
            scene.step(&mut sim, &mut cam, &scripts),
            SceneStep::Entering
        );
        assert!(scene.owns_gameplay());
        assert_eq!(
            scene.step(&mut sim, &mut cam, &scripts),
            SceneStep::Ran(0x40)
        );
        assert_eq!((sim.x, sim.y, cam.x, cam.y), (0x1F1, 0x2A4, 0x1B0, 0x2C0));
        assert_eq!(cam.scene_transition_flag, 0);
        for wait in 1..waits {
            assert_eq!(
                scene.step(&mut sim, &mut cam, &scripts),
                SceneStep::Entering
            );
            assert!(scene.owns_gameplay());
            assert_eq!(cam.scene_transition_flag, u8::from(wait == waits - 1));
        }
        assert_eq!(scene.step(&mut sim, &mut cam, &scripts), SceneStep::End);
        assert_eq!(
            (
                cam.scene_id,
                cam.scene_transition_flag,
                cam.scene_transition_latch
            ),
            (0, 0, 1)
        );
        assert!(!scene.owns_gameplay());
    }
}
