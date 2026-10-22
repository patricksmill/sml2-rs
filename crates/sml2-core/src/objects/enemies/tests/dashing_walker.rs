use super::*;

fn fixture(state: u8, x: u16, y: u16) -> (ObjectSim, Staging) {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 1; // `$5B51A`: leave the sprite alone.
    sim.mario_ref_x = 200;
    let mut st = Staging::default();
    st.b = std::array::from_fn(|i| (i as u8).wrapping_mul(37) ^ 0xA5);
    set_x(&mut st.b, x);
    set_y(&mut st.b, y);
    st.b[B_FSTATE] = state;
    st.b[B_PHASE] = 4;
    st.b[B_T25] = 1;
    (sim, st)
}

/// `$59D7A/$59DA8` call `$3C44 -> $8532`; `$59D8B/$59DC7` call
/// `$3BE0 -> $8484`. Both use X +/- 19 and Y - 11, then Y - 5,
/// treating tiles below `$38` as walls. Cover patrol and charge in both
/// directions, including the lower probe and the nonsolid boundary.
#[test]
fn handler_0x59d5a_wide_wall_turns_preserve_the_complete_slot() {
    for state in 1u8..=4 {
        let left = state == 1 || state == 3;
        let probe_x = if left { 85 } else { 123 };
        for (upper, lower, hit, probe_y) in [
            (0x37, 0x60, true, 93),
            (0x60, 0x37, true, 99),
            (0x38, 0x38, false, 99),
            (0x60, 0x60, false, 99),
        ] {
            let mut layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
            layout.cells[5 * 256 + probe_x / 16] = upper;
            layout.cells[6 * 256 + probe_x / 16] = lower;
            let (mut sim, mut st) = fixture(state, 104, 104);
            let mut expected = st.b;
            if hit {
                expected[B_FLIP] ^= 0x20;
                expected[B_FSTATE] ^= if state <= 2 { 3 } else { 7 };
            } else {
                // Patrol is one pixel; charge phase 4 of `$74A4` is two.
                let speed = if state <= 2 { 1 } else { 2 };
                set_x(&mut expected, if left { 104 - speed } else { 104 + speed });
                if state >= 3 {
                    expected[B_PHASE] = 5;
                }
            }
            sim.handler_dashing_walker(&mut st, &layout);
            assert_eq!(
                st.b, expected,
                "state {state}, tiles {upper:02x}/{lower:02x}"
            );
            assert_eq!((st.px, st.py), (probe_x as u16, probe_y));
            assert_eq!(
                sim.move_flags,
                if left {
                    0x20 | u8::from(hit) * 2
                } else {
                    0x10 | u8::from(hit)
                }
            );
        }
    }
}

/// The level-14 wall at column 159 is `$2C`; column 160 is `$54`.
/// At (2578, 656), `$8532` sees the wall at X=2559. The narrow `$84EA`
/// probe would sample X=2571 and let the charging walker pass the turn.
#[test]
fn handler_0x59d5a_level14_wall_turns_the_charge_without_moving() {
    let layout = LevelLayout::from_bytes(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/levels/level15.decompressed.bin"
    )))
    .unwrap();
    assert_eq!(layout.get(159, 40), Some(0x2C));
    assert_eq!(layout.get(160, 40), Some(0x54));
    let (mut sim, mut st) = fixture(3, 2578, 656);
    let mut expected = st.b;
    expected[B_FLIP] ^= 0x20;
    expected[B_FSTATE] = 4;
    sim.handler_dashing_walker(&mut st, &layout);
    assert_eq!(st.b, expected);
    assert_eq!((st.px, st.py, sim.move_flags), (2559, 645, 0x22));
}
