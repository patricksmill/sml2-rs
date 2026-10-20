use super::*;
use crate::level::{LevelLayout, LAYOUT_SIZE};

/// `$0F71` calls `$13FE` BEFORE testing the travelling direction. `$13FE`
/// clears $A202/$A218/$A283 (XSub, XPushed, PipeAlignTarget), even when
/// `$0FBB` then replaces normal physics with `$10EC` pipe travel.
#[test]
fn vertical_pipe_travel_clears_previous_horizontal_push_staging() {
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    for (direction, y_after) in [(0x10, 130), (0x20, 126)] {
        for previous_delta in [1, 0xFF] {
            let mut sim = PlayerSim::new();
            sim.x = 128;
            sim.y = 128;
            sim.pipe_dir = direction;
            sim.x_sub = previous_delta;
            sim.x_pushed = true;
            sim.pipe_align_target = 7;
            sim.hvel = 0x80;
            sim.vvel = 0x18;
            sim.step(
                &PlayerKeys {
                    right: true,
                    a: true,
                    ..Default::default()
                },
                true,
                false,
                &layout,
            );
            assert_eq!((sim.x, sim.y), (128, y_after));
            assert_eq!(
                [sim.x_sub, sim.x_pushed as u8, sim.pipe_align_target],
                [0, 0, 0],
                "$13FE must run before the travelling early return"
            );
            assert_eq!(
                [sim.hvel, sim.vvel],
                [0x80, 0x18],
                "normal physics is still bypassed"
            );
        }
    }
}

/// `$1136`/$1160 publish a fresh right/left camera delta after the same reset.
#[test]
fn horizontal_pipe_travel_replaces_previous_push_with_its_own_delta() {
    let layout = LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap();
    for (direction, x_after, delta) in [(0x30, 130, 1), (0x40, 126, 0xFF)] {
        let mut sim = PlayerSim::new();
        sim.x = 128;
        sim.y = 128;
        sim.pipe_dir = direction;
        sim.x_sub = 23;
        sim.x_pushed = true;
        sim.pipe_align_target = 7;
        sim.step(&PlayerKeys::default(), false, false, &layout);
        assert_eq!((sim.x, sim.y), (x_after, 128));
        assert_eq!(
            [sim.x_sub, sim.x_pushed as u8, sim.pipe_align_target],
            [delta, 0, 0]
        );
    }
}
