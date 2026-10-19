//! Source `$1675..$16AE` jump selection followed by `$14FA` moon gravity.
use crate::{
    level::{LevelLayout, LAYOUT_SIZE},
    player_sim::{PlayerKeys, PlayerSim},
};

#[test]
fn held_up_preserves_high_jump_across_the_walking_speed_band() {
    let mut cells = vec![0x60; LAYOUT_SIZE];
    cells[30 * 256..31 * 256].fill(0x18);
    let layout = LevelLayout::from_bytes(&cells).unwrap();
    // Source selects 75 outside [$48,$C0), or with Up; otherwise 48.
    // The same frame's gravity decrements 75 to 74 and snaps moon 48 to 47.
    for (hvel, ordinary, held_up) in [
        (0x47, 74, 74),
        (0x48, 47, 74),
        (0xBF, 47, 74),
        (0xC0, 74, 74),
    ] {
        for (up, expected) in [(false, ordinary), (true, held_up)] {
            let mut sim = PlayerSim::new();
            sim.x = 40;
            sim.y = 452;
            sim.hvel = hvel;
            sim.seed_moon_physics(8);
            sim.step(
                &PlayerKeys {
                    a: true,
                    up,
                    ..Default::default()
                },
                true,
                false,
                &layout,
            );
            assert_eq!(sim.vvel, expected, "hvel=${hvel:02X}, Up={up}");
            assert_eq!(sim.moon_float_timer, 50, "$104A ground budget");
            assert!(!sim.on_ground);
            assert!(sim.jump_flag);
            assert_eq!(sim.dead, 0);
        }
    }
}
