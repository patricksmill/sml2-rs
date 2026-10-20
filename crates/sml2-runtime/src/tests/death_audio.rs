use crate::session::{FrameInput, Session, SessionAssets, SessionState};
use sml2_core::{audio::Apu, player_sim::PlayerKeys};

#[test]
fn death_fade_plays_the_jingle_through_the_apu() {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    session.begin_level(0, 0).expect("the level loads");
    let input = FrameInput {
        keys: PlayerKeys::default(),
        div: 0x3D,
    };
    session.advance(input);
    assert_ne!(session.sound_driver.active_song, 2);

    if let SessionState::InLevel(ctx) = &mut session.state {
        ctx.sim.dead = 0xFF;
        ctx.sim.vvel = 1;
        // The next $315C step adds 3, producing screen Y $B2.
        ctx.sim.y = (ctx.cam.y & 0xFF00) | u16::from((ctx.cam.y as u8).wrapping_add(77));
    } else {
        panic!("expected a level");
    }

    let mut apu = Apu::new();
    let mut pcm = Vec::new();
    for _ in 0..16 {
        session.advance(FrameInput {
            keys: PlayerKeys::default(),
            div: 0x3D,
        });
        assert_eq!(session.sound_driver.active_song, 2, "$3186 death jingle");
        if let SessionState::InLevel(ctx) = &session.state {
            assert!(ctx.sim.dead_fade);
            assert_eq!(ctx.sim.music_request, 0, "request consumed by the driver");
        } else {
            panic!("the fade is still running");
        }
        apu.ingest_driver(&session.sound_driver);
        apu.generate_samples(44_100, 735, &mut pcm);
    }
    assert!(
        pcm.iter().any(|sample| sample.abs() > 0.01),
        "the death jingle reaches the audio output"
    );
}
