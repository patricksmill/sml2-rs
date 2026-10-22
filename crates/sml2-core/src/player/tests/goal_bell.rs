use super::*;
use crate::level::{LevelLayout, LAYOUT_SIZE};

/// `$181C`: sGoalBell selects MarioSpeedTableH before the swim/moon
/// branches. `$17C5` sets $B8 on the ground; friction leaves $B4.
/// Expected displacements are ROM `$1AB3..$1ABA` and `$1BB3..$1BBA`.
#[test]
fn goal_bell_coast_uses_normal_speed_in_water_and_moon_zones() {
    let normal = [1u8, 1, 1, 0, 1, 1, 1, 1];
    let slow = [0u8, 0, 0, 0, 1, 0, 0, 0];
    for (tile, swimming, moon) in [(0x60, false, 0), (0x4C, true, 0), (0x54, false, 2)] {
        let layout = LevelLayout::from_bytes(&[tile; LAYOUT_SIZE]).unwrap();
        for goal in [false, true] {
            for phase in 0..8 {
                let mut sim = PlayerSim::new();
                sim.x = 100;
                sim.y = 200;
                sim.camx = 133;
                sim.camy = 200;
                sim.on_ground = true;
                sim.on_ground_obj = true;
                sim.hvel = 0xB8;
                sim.swim = swimming;
                sim.moon = moon;
                sim.goal_bell = goal;
                sim.phase = phase;
                let dx = if goal || tile == 0x60 {
                    normal[phase as usize]
                } else {
                    slow[phase as usize]
                };
                sim.step(&PlayerKeys::default(), false, false, &layout);
                assert_eq!(
                    sim.x.to_le_bytes(),
                    [100 + dx, 0],
                    "tile {tile:02x}, goal {goal}, phase {phase}"
                );
                assert_eq!(sim.y.to_le_bytes(), [200, 0]);
                assert_eq!([sim.hvel, sim.x_sub, sim.moon], [0xB4, dx, moon]);
                assert_eq!(sim.swim, swimming);
                assert_eq!(sim.goal_bell, goal);
                assert!(sim.on_ground);
            }
        }
    }
}

/// Level 14's moon floor below the bell: after landing at Y=628,
/// the coast advances from X=2968 rather than waiting on the slow table.
#[test]
fn goal_bell_coast_advances_on_the_level14_moon_floor() {
    let layout = LevelLayout::from_bytes(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/levels/level15.decompressed.bin"
    )))
    .unwrap();
    let mut sim = PlayerSim::new();
    sim.x = 2968;
    sim.y = 628;
    sim.camx = 2992;
    sim.camy = 637;
    sim.powerup = 1;
    sim.on_ground = true;
    sim.hvel = 0xB4;
    sim.vvel = 0;
    sim.moon = 2;
    sim.goal_bell = true;
    sim.phase = 0;
    sim.step(&PlayerKeys::default(), false, false, &layout);
    assert_eq!(sim.x.to_le_bytes(), [0x99, 0x0B]); // 2969
    assert_eq!(sim.y.to_le_bytes(), [0x74, 0x02]); // 628
    assert_eq!([sim.hvel, sim.vvel, sim.x_sub, sim.moon], [0xB4, 0, 1, 2]);
    assert!(sim.on_ground && sim.goal_bell);
}
