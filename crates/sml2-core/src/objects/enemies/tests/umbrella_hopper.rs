use super::*;

// Slot bytes: +11 sprite, +15 state, +16 $FFE0 phase, +18 contact,
// +24 $FFE8 (Y cursor), +25 $FFE9 (sway cursor), +27 $FFEB.

fn open_air() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; crate::level::LAYOUT_SIZE]).unwrap()
}

fn ground_at(x: usize, y: usize) -> LevelLayout {
    let mut cells = vec![0x60u8; crate::level::LAYOUT_SIZE];
    cells[y * crate::level::LAYOUT_WIDTH + x] = 0x00;
    LevelLayout::from_bytes(&cells).unwrap()
}

fn staged(state: u8) -> Staging {
    let mut st = Staging::default();
    set_x(&mut st.b, 0x0200);
    set_y(&mut st.b, 0x0100);
    st.b[11] = 0x96;
    st.b[15] = state;
    st
}

fn offset(st: &Staging) -> (i32, i32) {
    (i32::from(st.x()) - 0x0200, i32::from(st.y()) - 0x0100)
}

fn run(sim: &mut ObjectSim, st: &mut Staging, layout: &LevelLayout, calls: usize) {
    for _ in 0..calls {
        sim.handler_0x599a6(st, layout);
    }
}

#[test]
fn handler_0x599a6_zero_state_arms_contact_and_state_five() {
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staged(0);
    sim.handler_0x599a6(&mut st, &open_air());
    assert_eq!((st.b[18], st.b[15]), (handler_0x599a6::CONTACT, 5));
    assert_eq!(offset(&st), (0, 0));
}

#[test]
fn handler_0x599a6_launches_64_up_settles_9_down_then_curves_8_down_and_faces_mario() {
    let layout = open_air();
    let mut sim = ObjectSim::new(vec![], 0);
    sim.mario_ref_x = 0x0100; // Mario to the left
    let mut st = staged(5);

    // $599C4: two pixels up per call until $FFE8 equals 64.
    run(&mut sim, &mut st, &layout, 31);
    assert_eq!((st.b[15], st.b[24], offset(&st)), (5, 62, (0, -62)));
    run(&mut sim, &mut st, &layout, 1);
    assert_eq!((st.b[15], st.b[24], offset(&st)), (6, 9, (0, -64)));

    // $599DA: three pixels down per call until 9 counts to zero.
    run(&mut sim, &mut st, &layout, 3);
    assert_eq!((st.b[15], st.b[11], offset(&st)), (7, 0x97, (0, -55)));

    // $599EE: $74AB = 2,1,2,1,1,1,0,0 then face Mario at phase 8.
    run(&mut sim, &mut st, &layout, 7);
    assert_eq!((st.b[15], st.b[16], offset(&st)), (7, 7, (0, -47)));
    run(&mut sim, &mut st, &layout, 1);
    assert_eq!(
        (st.b[15], st.b[16], st.b[13], offset(&st)),
        (1, 0, 0x00, (0, -47))
    );
    assert_eq!(sim.facing, 255);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x599a6_sways_24_pixels_each_way_while_gliding_8_down_per_leg() {
    // $5A9D sums to 24 and $5A8D to 8 over a 16-call leg.
    let layout = open_air();
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staged(1);
    for (state, cursor, position) in [
        (3, 15, (-24, 8)),
        (2, 0, (0, 16)),
        (4, 15, (24, 24)),
        (1, 0, (0, 32)),
    ] {
        run(&mut sim, &mut st, &layout, 16);
        assert_eq!(
            (st.b[15], st.b[25], st.b[24], offset(&st)),
            (state, cursor, 0, position)
        );
    }
    assert_eq!(st.b[27], 0);
    assert!(sim.unported.is_empty());
}

#[test]
fn handler_0x599a6_landing_resets_cursors_and_relaunches() {
    // The ground probe reads cell (x >> 4, y >> 4) under the hopper.
    let layout = ground_at(0x1F, 0x10);
    let mut sim = ObjectSim::new(vec![], 0);
    let mut st = staged(1);
    st.b[11] = 0x97;
    st.b[16] = 5;
    st.b[24] = 9;
    st.b[25] = 2;
    sim.handler_0x599a6(&mut st, &layout);
    // The sway step ($5A9D[2] = 3 left) lands before the ground probe.
    assert_eq!(offset(&st), (-3, 0));
    assert_eq!(
        (st.b[15], st.b[11], st.b[16], st.b[24], st.b[25]),
        (5, 0x96, 0, 0, 0)
    );
}

#[test]
fn handler_0x599a6_reports_invalid_state_and_cursor() {
    for (state, cursor) in [(8, 0), (2, 16)] {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = staged(state);
        st.b[25] = cursor;
        sim.handler_0x599a6(&mut st, &open_air());
        assert_eq!(sim.unported, vec![0x599A6]);
    }
}
