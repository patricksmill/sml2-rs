//! `$FF97` advances only on frames that run the `$0300` frame wait.
use super::*;
use sml2_core::player_sim::PlayerKeys;

fn idle(session: &mut Session) -> StepOutcome {
    session
        .advance(FrameInput {
            keys: PlayerKeys::default(),
            div: 0,
        })
        .outcome
}

fn on_hub() -> Session {
    let mut session = Session::new(SessionAssets::load().unwrap());
    idle(&mut session);
    assert!(matches!(session.state, SessionState::Overworld(_)));
    session
}

/// Map entry: `$3E9FB`'s fade and save (50 frames) and the `$03` loader
/// (19) block; the `$0B`, `$02`, `$02` and first `$03` frames tick.
#[test]
fn map_entry_ticks_only_between_the_fade_and_the_loader() {
    let mut session = on_hub();
    let before = session.ff97();
    session.advance(FrameInput {
        keys: PlayerKeys {
            a: true,
            ..Default::default()
        },
        div: 0,
    });
    assert_eq!(session.ff97(), before.wrapping_add(1), "the A frame ticks");
    let mut ticks = Vec::new();
    for frame in 1..=MAP_ENTRY_FRAMES {
        let ff97 = session.ff97();
        let outcome = idle(&mut session);
        if session.ff97() != ff97 {
            ticks.push(frame);
        }
        if frame == MAP_ENTRY_FRAMES {
            assert!(matches!(outcome, StepOutcome::LevelEntered(_)));
        }
    }
    assert_eq!(ticks, [51, 52, 53, 54]);
    // Level frames tick, with the level's own phase origin untouched.
    let ff97 = session.ff97();
    idle(&mut session);
    assert_eq!(session.ff97(), ff97.wrapping_add(1));
    let SessionState::InLevel(ctx) = &session.state else {
        panic!("level")
    };
    assert_eq!(
        ctx.sim.frame_counter, 1,
        "the level counts from its own origin"
    );
}

/// Easy mode's `$17` card frames run the frame wait between the two blocks.
#[test]
fn the_easy_card_ticks_between_the_fade_and_the_loader() {
    let mut session = on_hub();
    session.easy_mode = true;
    let mut transition = MapTransition::from_map(
        Box::new(LevelContext::from_content(
            0,
            session.assets.level(0).unwrap(),
            0,
            5,
            0,
            0,
            0,
            0,
            session.assets.scripts.clone(),
            session.assets.tilesets.clone(),
            None,
            session.assets.anim_rows.clone(),
            session.assets.hud.clone(),
            session.assets.object_tables.clone(),
        )),
        0,
        true,
    );
    let ticking: Vec<u16> = (1..transition.live_at)
        .filter(|&frame| transition.frame_waits(frame))
        .collect();
    assert_eq!(ticking, (51..=118).collect::<Vec<_>>());
    transition.easy_card = None;
    assert!(!transition.frame_waits(55));
}

/// `$19` turns VBlank off from its first frame.
#[test]
fn the_file_select_load_blocks_every_frame() {
    let mut session = on_hub();
    session.state = SessionState::FileSelectLoading(FileSelectLoading::new());
    let ff97 = session.ff97();
    for _ in 0..6 {
        idle(&mut session);
    }
    assert!(matches!(session.state, SessionState::FileSelect(_)));
    assert_eq!(session.ff97(), ff97);
    idle(&mut session);
    assert_eq!(session.ff97(), ff97.wrapping_add(1));
}

/// Door walks and the hub reload block; a hold's last frame runs the map.
#[test]
fn door_pan_and_bell_holds_block_the_frame_wait() {
    let mut session = on_hub();
    let SessionState::Overworld(map) = &session.state else {
        unreachable!()
    };
    session.map_screen_transition = Some(MapScreenTransition::new(map.clone(), 0));
    assert!(!session.main_loop_waits());
    session.map_screen_transition = None;
    session.map_pan_return = Some(MapPanReturn::default());
    assert!(!session.main_loop_waits());
    session.map_pan_return = None;

    session.bell_entry = Some(BELL_ENTRY_HOLD - 1);
    assert!(!session.main_loop_waits());
    session.bell_entry = Some(BELL_ENTRY_HOLD);
    assert!(
        session.main_loop_waits(),
        "the hold's last frame runs the map"
    );
    session.bell_entry = None;

    session.bell_return_hold = 84;
    session.bell_fade = Some(81);
    assert!(!session.main_loop_waits());
    session.bell_fade = Some(82);
    assert!(session.main_loop_waits(), "83 of the hub's 84 frames block");
    session.bell_fade = None;
    assert!(session.main_loop_waits());
}

/// The map's zone routines sample the session's `$FF97`, which a blocked
/// frame leaves behind the host frame count.
#[test]
fn the_map_samples_the_session_frame_clock() {
    let mut session = on_hub();
    session.state = SessionState::FileSelectLoading(FileSelectLoading::new());
    for _ in 0..6 {
        idle(&mut session);
    }
    let save = MapSim::new().save_record(5, 0, 0);
    assert!(session.apply_save_record(&save));
    idle(&mut session);
    assert_ne!(session.ff97(), session.frame_n as u8);
    let SessionState::Overworld(map) = &session.state else {
        panic!("map")
    };
    assert_eq!(map.zone_animation_frame(), session.ff97());
}
