use crate::session::{FrameInput, Session, SessionAssets, SessionState};
use sml2_core::{objects::*, player_sim::PlayerKeys};

#[test]
fn witch_reward_and_coin_deposit_return_to_the_hub_node() {
    let mut session = Session::new(SessionAssets::load().unwrap());
    session.begin_level(9, 0).unwrap();
    let SessionState::InLevel(ctx) = &mut session.state else {
        panic!("level entry");
    };
    ctx.sim.map_node = 0x2B;
    ctx.sim.level_progress[2] = 8; // An unrelated path survives the deposit.
    ctx.sim.fade_timer = 0;
    ctx.cam.scene_state = 0xF0;
    // `$9688`: the reward's final ascent writes game state $1D at Y < 8.
    ctx.obj.slots[0].b[B_DEF] = 4;
    ctx.obj.slots[0].b[B_FSTATE] = 3;
    ctx.obj.slots[0].b[B_SCRY] = 8;
    ctx.obj.fff0 = 1;
    let mut deposit_seen = false;
    for _ in 0..1024 {
        let effects = session.advance(FrameInput {
            keys: PlayerKeys::default(),
            div: 0,
        });
        assert!(effects.diagnostics.is_empty(), "{:?}", effects.diagnostics);
        match &session.state {
            SessionState::Castle(_) => deposit_seen = true,
            SessionState::Overworld(map) => {
                assert!(deposit_seen, "coin deposit precedes map gameplay");
                assert_eq!((map.screen_id, map.node, map.cursor_move), (0, 0x1C, 8));
                // `$3C7CA` loads the big-endian node record before `$3F156`.
                assert_eq!((map.cam_x, map.cam_y), (0x0135, 0x003A));
                assert_eq!((map.cursor_x, map.cursor_y), (75, 76));
                assert_eq!(map.level_progress[9], 0x80);
                assert_eq!(map.level_progress[2], 8);
                return;
            }
            _ => {}
        }
    }
    panic!("boss reward never returned from the coin deposit");
}
