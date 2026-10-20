use super::*;

// Slot bytes: +11 sprite, +13 flip, +15 state, +18 contact, +26 $FFEA.

fn open_air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; crate::level::LAYOUT_SIZE]).unwrap()
}

fn wall_at(x: usize, y: usize) -> LevelLayout {
    let mut cells = vec![0x60u8; crate::level::LAYOUT_SIZE];
    cells[y * crate::level::LAYOUT_WIDTH + x] = 0x00;
    LevelLayout::from_bytes(&cells).unwrap()
}

fn staged(state: u8) -> Staging {
    let mut st = Staging::default();
    set_x(&mut st.b, 0x0200);
    set_y(&mut st.b, 0x0104);
    st.b[11] = 50;
    st.b[15] = state;
    st
}

#[test]
fn handler_0x59774_states_outside_one_to_three_arm_contact_and_wait() {
    for state in [0, 4, 9] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staged(state);
        sim.handler_0x59774(&mut st, &open_air());
        assert_eq!((st.b[18], st.b[15]), (handler_0x59774::CONTACT, 3));
        assert_eq!((st.x(), st.y()), (0x0200, 0x0104));
    }
}

#[test]
fn handler_0x59774_waits_glancing_every_eighth_frame_until_mario_is_within_48() {
    let layout = open_air();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 0x0200 + 48;
    let mut st = staged(3);
    for (frame, flip) in [(7, 0x00), (8, 0x20), (9, 0x20), (12, 0x20), (16, 0x00)] {
        sim.fff0 = frame;
        sim.handler_0x59774(&mut st, &layout);
        assert_eq!((st.b[15], st.b[13]), (3, flip), "frame {frame}");
    }
    sim.mario_ref_x = 0x0200 + 47;
    sim.fff0 = 17;
    sim.handler_0x59774(&mut st, &layout);
    assert_eq!((st.b[15], st.b[13], sim.facing), (2, 0x20, 1));
    assert_eq!(st.x(), 0x0200);
}

#[test]
fn handler_0x59774_runs_two_pixels_per_call_horizontally_only() {
    for (state, dx) in [(1, -20), (2, 20)] {
        let layout = open_air();
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staged(state);
        for frame in 0..10 {
            sim.fff0 = frame;
            sim.handler_0x59774(&mut st, &layout);
        }
        assert_eq!(i32::from(st.x()) - 0x0200, dx);
        assert_eq!((st.y(), st.b[15]), (0x0104, state));
        // $5B585 on frames 0,2,4,6,8: +1, -1, +2, -2, +1 from $75A2.
        assert_eq!((st.b[26], st.b[11]), (1, 51));
    }
}

#[test]
fn handler_0x59774_turns_at_walls_found_by_the_high_probe_retry() {
    // Y = $104: the first probe (Y - 19) reads row $0F, which is empty;
    // only the source retry at Y - 4 reaches row $10 (Y - 5 would not).
    for (state, cell_x, next) in [(1, 0x1F, 2), (2, 0x20, 1)] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staged(state);
        sim.handler_0x59774(&mut st, &wall_at(cell_x, 0x10));
        assert_eq!((st.b[15], st.b[13], st.x()), (next, 0x20, 0x0200));
    }
}
