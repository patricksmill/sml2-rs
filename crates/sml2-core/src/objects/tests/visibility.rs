//! Real slot-loop checks for the zoomed view's asymmetric vertical reach.
use super::*;

#[test]
fn biased_view_promotes_slots_at_both_shifted_activation_edges() {
    let t = tables();
    let layout = blank_layout();
    // 150% view: native half-height 96 + 36, shifted upward 18.
    // At camera Y=512 the inclusive bounds are 362..=626.
    // At camera Y=8 they are 0..=122, not 0..=132.
    for (camera_y, y, promoted) in [
        (512, 361, false),
        (512, 362, true),
        (512, 626, true),
        (512, 627, false),
        (8, 0, true),
        (8, 122, true),
        (8, 123, false),
    ] {
        let mut obj = ObjectSim::new(vec![], 512);
        obj.view_margin = (112, 36);
        obj.view_bias_y = 18;
        obj.phase = true; // next step takes the even-frame promotion branch
        obj.standby = 1;
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 1;
        s.b[B_PARKED] = 0x4e;
        set_x(&mut s.b, 512);
        set_y(&mut s.b, y);
        obj.step(512, camera_y, &layout, &t, &MarioContact::default(), false);
        assert_eq!(
            obj.slots[0].b[B_STATE],
            if promoted { 2 } else { 1 },
            "camera_y={camera_y} y={y}"
        );
        assert_eq!(
            obj.camera_position.y, camera_y,
            "gameplay camera is unchanged"
        );
    }
}

#[test]
fn biased_view_culls_slots_beyond_both_shifted_alive_edges() {
    let t = tables();
    let layout = blank_layout();
    // Alive half-height 160 + 36, shifted upward 18: 298..=690.
    for (y, alive) in [(297, false), (298, true), (690, true), (691, false)] {
        let mut obj = ObjectSim::new(vec![], 512);
        obj.view_margin = (112, 36);
        obj.view_bias_y = 18;
        obj.phase = true;
        obj.active = 1;
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_RECHI] = 0xad; // no stream record: out-of-window slots are removed
        set_x(&mut s.b, 512);
        set_y(&mut s.b, y);
        obj.step(512, 512, &layout, &t, &MarioContact::default(), false);
        assert_eq!(obj.slots[0].b[B_STATE] != 0, alive, "y={y}");
    }
}

#[test]
fn bounded_view_promotes_slots_at_shifted_left_and_right_edges() {
    let t = tables();
    let layout = blank_layout();
    // 384px view at the level start: camera 80 is displayed at 192.
    // At the end: camera 4016 is displayed at 3904.
    for (camera_x, offset, x, promoted) in [
        (80, 112, 399, true),
        (80, 112, 400, true),
        (80, 112, 401, false),
        (4016, -112, 3695, false),
        (4016, -112, 3696, true),
    ] {
        let mut obj = ObjectSim::new(vec![], camera_x);
        obj.view_margin = (112, 36);
        obj.view_offset = (offset, 0);
        obj.phase = true;
        obj.standby = 1;
        let slot = &mut obj.slots[0];
        slot.b[B_STATE] = 1;
        slot.b[B_PARKED] = 0x4e;
        set_x(&mut slot.b, x);
        set_y(&mut slot.b, 448);
        obj.step(camera_x, 448, &layout, &t, &MarioContact::default(), false);
        assert_eq!(
            obj.slots[0].b[B_STATE],
            if promoted { 2 } else { 1 },
            "camera {camera_x}, object {x}"
        );
        assert_eq!(obj.camera_position.x, camera_x);
    }
}

#[test]
fn bounded_view_fills_records_visible_beyond_the_old_right_edge() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(vec![margin_record(360)], 80);
    obj.view_margin = (112, 36);
    obj.view_offset = (112, 0);
    obj.step(80, 448, &layout, &t, &MarioContact::default(), false);
    assert_eq!(obj.spawned, vec![true]);
    assert_eq!(obj.slots[0].b[B_STATE], 1);
}
