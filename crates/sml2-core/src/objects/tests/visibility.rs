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
