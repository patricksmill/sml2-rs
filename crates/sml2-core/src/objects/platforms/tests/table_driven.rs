use super::*;

// Slot bytes: +11 sprite, +13 flip, +15 state, +24 $FFE8 (Y cursor),
// +25 $FFE9 (X cursor), +26 $FFEA (sprite cycle), +27 $FFEB (knockback).

fn staged(state: u8, y_cursor: u8, x_cursor: u8) -> Staging {
    let mut st = Staging::default();
    set_x(&mut st.b, 0x0200);
    set_y(&mut st.b, 0x0100);
    st.b[11] = 50;
    st.b[15] = state;
    st.b[24] = y_cursor;
    st.b[25] = x_cursor;
    st
}

fn offset(st: &Staging) -> (i32, i32) {
    (i32::from(st.x()) - 0x0200, i32::from(st.y()) - 0x0100)
}

#[test]
fn handler_0x597bd_init_sets_contact_faces_mario_and_starts_both_cursors() {
    for (mario_x, state, flip) in [(0x0100, 1, 0x00), (0x0300, 2, 0x20)] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.mario_ref_x = mario_x;
        sim.fff0 = 1; // init ignores the frame parity
        let mut st = staged(0, 0x33, 0x44);
        sim.handler_0x597bd(&mut st);
        assert_eq!(st.b[18], handler_0x597bd::CONTACT);
        assert_eq!((st.b[15], st.b[13]), (state, flip));
        assert_eq!((st.b[24], st.b[25], st.b[27]), (1, 1, 0));
        assert_eq!((offset(&st), st.b[11]), ((0, 0), 50));
    }
}

#[test]
fn handler_0x597bd_traces_the_rom_figure_eight_in_twenty_call_legs() {
    // Leg sums of the ROM curves: $5938 = 16, $594E = 24, $5964 = 24,
    // $597A = 32, $5990 = 24.
    for (start, legs) in [
        (
            1,
            [(3, (-24, -16)), (5, (-48, 0)), (7, (-24, 32)), (2, (0, 0))],
        ),
        (
            2,
            [(4, (24, -16)), (6, (48, 0)), (8, (24, 32)), (1, (0, 0))],
        ),
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staged(start, 1, 1);
        let mut frame: u8 = 2;
        for (next, position) in legs {
            for _ in 0..20 {
                sim.fff0 = frame;
                sim.handler_0x597bd(&mut st);
                frame = frame.wrapping_add(2);
            }
            assert_eq!((st.b[15], offset(&st)), (next, position), "start {start}");
        }
        assert_eq!((st.b[24], st.b[25]), (1, 1));
        assert_eq!(st.b[13], 0x20, "one flip per lap");
        assert!(sim.unported.is_empty());
    }
}

#[test]
fn handler_0x597bd_moves_only_on_even_frames() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staged(1, 5, 4);
    sim.fff0 = 3;
    sim.handler_0x597bd(&mut st);
    assert_eq!((offset(&st), st.b[24], st.b[25]), ((0, 0), 5, 4));
    sim.fff0 = 6;
    sim.handler_0x597bd(&mut st);
    // $5938[5] = 2 up, $594E[4] = 1 left.
    assert_eq!((offset(&st), st.b[24], st.b[25]), ((-1, -2), 6, 5));
}

#[test]
fn handler_0x597bd_runs_the_sprite_cycle_every_fourth_frame() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staged(1, 1, 1);
    for (frame, cycle, sprite) in [
        (1, 0, 50),
        (4, 1, 51),
        (6, 1, 51),
        (8, 2, 50),
        (12, 3, 52),
        (16, 0, 50),
    ] {
        sim.fff0 = frame;
        sim.handler_0x597bd(&mut st);
        assert_eq!((st.b[26], st.b[11]), (cycle, sprite), "frame {frame}");
    }
}

#[test]
fn handler_0x597bd_keeps_source_cursor_clamps_and_wraps() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 2;

    // $58CE holds the Y cursor at 20: $5938[20] = 1 up, $594E[5] = 0.
    let mut st = staged(2, 20, 5);
    sim.handler_0x597bd(&mut st);
    assert_eq!((offset(&st), st.b[24], st.b[25]), ((0, -1), 20, 6));

    // $5919 wraps the Y cursor to 1: $597A[20] = 0, $5990[3] = 0.
    let mut st = staged(5, 20, 3);
    sim.handler_0x597bd(&mut st);
    assert_eq!((offset(&st), st.b[24], st.b[25]), ((0, 0), 1, 4));

    // $592A stops the Y cursor at 1: $597A[1] = 0, $5990[10] = 1 right.
    let mut st = staged(7, 1, 10);
    sim.handler_0x597bd(&mut st);
    assert_eq!((offset(&st), st.b[24], st.b[25]), ((1, 0), 1, 9));
}

#[test]
fn handler_0x597bd_state_three_keeps_the_y_cursor_but_state_four_resets_it() {
    let mut sim = ObjectSim::new(vec![], 0);
    sim.fff0 = 2;
    // $5938[5] = 2 down, $5964[20] = 1, then the X leg completes.
    for (state, dx, next, y_cursor) in [(3, -1, 5, 4), (4, 1, 6, 1)] {
        let mut st = staged(state, 5, 20);
        st.b[26] = 3;
        sim.handler_0x597bd(&mut st);
        assert_eq!((offset(&st), st.b[15]), ((dx, 2), next));
        assert_eq!(
            (st.b[24], st.b[25], st.b[26], st.b[13]),
            (y_cursor, 1, 0, 0x20)
        );
    }
}

#[test]
fn handler_0x597bd_reports_invalid_state_and_out_of_range_cursor() {
    for st in [staged(9, 1, 1), staged(1, 1, 200)] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = st;
        sim.fff0 = 2;
        sim.handler_0x597bd(&mut st);
        assert_eq!(sim.unported, vec![0x597BD]);
    }
}
