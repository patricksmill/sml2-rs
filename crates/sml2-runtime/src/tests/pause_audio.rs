//! START in a level must park the sound engine (`sSoundMode = 1`) and START
//! again must hand it back (`sSoundMode = 2`), the way `$273E`/`$2756` do.
//!
//! The user-visible symptom these pin: the music kept playing over the pause
//! screen and no pause beep was ever heard, because the level handlers' two
//! `sSoundMode` writes were not modelled at all.

use crate::session::{FrameInput, Session, SessionAssets, SessionState};
use sml2_core::player_sim::PlayerKeys;

fn session_in_level(level: u8) -> Session {
    let mut session = Session::new(SessionAssets::load().expect("session assets must load"));
    session.begin_level(level, 0).expect("the level loads");
    assert!(matches!(session.state, SessionState::InLevel(_)));
    session
}

fn tick(session: &mut Session, keys: PlayerKeys) {
    session.advance(FrameInput { keys, div: 0x3D });
}

fn press_start(session: &mut Session) {
    tick(
        session,
        PlayerKeys {
            start: true,
            ..PlayerKeys::default()
        },
    );
}

#[test]
fn start_parks_the_sound_engine_and_start_again_resumes_it() {
    let mut session = session_in_level(0);
    for _ in 0..8 {
        tick(&mut session, PlayerKeys::default());
    }
    let playing = session.sound_driver.active_song;
    assert_ne!(playing, 0, "the level's own song is running");
    assert!(!session.sound_driver.sound_paused);

    press_start(&mut session);
    assert!(
        session.sound_driver.sound_paused,
        "the pause frame stops the engine"
    );
    assert_eq!(
        session.sound_driver.sound_mode, 0,
        "$10067 clears the one-shot mode"
    );
    assert_eq!(
        session.sound_driver.active_song, playing,
        "the song stays loaded: the ROM does not restart it"
    );

    // Paused frames hold the song and run only the beep sequence.
    let held = session.sound_driver.music_sequencer.clone();
    for _ in 0..40 {
        tick(&mut session, PlayerKeys::default());
        assert_eq!(
            session.sound_driver.music_sequencer, held,
            "the song does not advance while paused"
        );
    }
    assert!(
        session.sound_driver.pause_beep_index >= 5,
        "the beep sequence ran through its entries (index {})",
        session.sound_driver.pause_beep_index
    );

    press_start(&mut session);
    assert!(!session.sound_driver.sound_paused, "START again resumes");
    let resumed = session.sound_driver.music_sequencer.clone();
    for _ in 0..4 {
        tick(&mut session, PlayerKeys::default());
    }
    assert_ne!(
        session.sound_driver.music_sequencer, resumed,
        "the song advances again"
    );
}

#[test]
fn the_pause_frame_reaches_the_apu_with_silence_and_the_first_beep() {
    let mut session = session_in_level(0);
    for _ in 0..8 {
        tick(&mut session, PlayerKeys::default());
    }
    // The host ingests the driver after every tick, so the frame's writes are
    // only on the driver that the pause tick produced.
    let mut apu = sml2_core::audio::Apu::new();
    tick(&mut session, PlayerKeys::default());
    apu.ingest_driver(&session.sound_driver);
    press_start(&mut session);
    apu.ingest_driver(&session.sound_driver);

    assert!(
        session
            .sound_driver
            .pulse_writes
            .contains(&sml2_core::audio::PulseWrite::Ch2FrequencyLow(0xC1)),
        "the first beep's NR23 write is on the pause frame"
    );
    // `$13FE7` silences the channel and `$5970` immediately re-arms it, so the
    // APU ends the frame holding the beep rather than the old music note.
    assert_eq!(apu.ch2.volume, 10, "NR22 = $A1's envelope");
    assert_eq!(apu.ch2.freq, 0x7C1, "NR23/NR24 = $C1/$C7");
    assert!(apu.ch2.length_enabled, "NR24 bit 6");
    // Everything the music was sounding is now quiet: pulse 1 and noise at
    // volume 0, the wave DAC off. That is the "music stops" half of the report.
    assert_eq!(apu.ch1.volume, 0, "NR12 = 8 silences pulse 1");
    assert_eq!(apu.ch4.volume, 0, "NR42 = 8 silences noise");
    assert!(!apu.ch3.enabled, "NR30 = 0 turns the wave DAC off");

    // And the beep is actually audible: the paused frame still renders sound.
    let mut pcm = Vec::new();
    apu.generate_samples(44_100, 2_205, &mut pcm);
    let peak = pcm.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
    assert!(
        peak > 0.01,
        "the pause beep reaches the output (peak {peak})"
    );
}

#[test]
fn select_abandoning_a_cleared_stage_also_hands_the_engine_back() {
    let mut session = session_in_level(0);
    if let SessionState::InLevel(ctx) = &mut session.state {
        ctx.sim.level_progress_flags = 0x80;
    }
    press_start(&mut session);
    assert!(session.sound_driver.sound_paused);

    tick(
        &mut session,
        PlayerKeys {
            select: true,
            ..PlayerKeys::default()
        },
    );
    assert!(
        !session.sound_driver.sound_paused,
        "$2760's sSoundMode = 2 resumes the engine on the way out"
    );
}
