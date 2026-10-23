use super::*;

fn tables() -> ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .unwrap()
}

/// `$FFEB` (+27) is the hit-reaction latch: `$5802B` sends any slot with a
/// non-zero value to the `$58101` knockback arc instead of its own handler.
/// These routines keep their leg/phase counters in `$FFE8` (+24) and never
/// touch `$FFEB`, so running them from spawn must leave the latch clear.
#[test]
fn ffe8_counter_handlers_never_set_the_knockback_latch() {
    let tables = tables();
    let layout = LevelLayout::from_bytes(&[0x60; crate::level::LAYOUT_SIZE]).unwrap();
    for handler in [
        0x59AAD, // blinking drifter
        0x5B177, // bobbing flyer
        0x5AFD7, // hopping ant
        0x599A6, // hover patroller
        0x59E5B, // rising walker
        0x5A355, // curved platform
        0x58D34, // diagonal platform
        0x58CE0, // diagonal patrol platform
        0x5A545, // flying platform
        0x58C41, // long horizontal platform
        0x597BD, // table-driven platform
        0x595FE, // vertical oscillator
    ] {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.mario_ref_x = 0x0180;
        sim.mario_ref_y = 0x00A0;
        let mut st = Staging::default();
        set_x(&mut st.b, 0x0100);
        set_y(&mut st.b, 0x0080);
        for frame in 0..600u32 {
            sim.fff0 = frame as u8;
            sim.dispatch_handler(handler, &mut st, &layout, &tables);
            assert_eq!(
                st.b[B_HITREACT], 0,
                "${handler:05X} set $FFEB on frame {frame} (state {})",
                st.b[B_FSTATE]
            );
        }
        assert!(
            sim.unported.is_empty(),
            "${handler:05X}: {:X?}",
            sim.unported
        );
    }
}
