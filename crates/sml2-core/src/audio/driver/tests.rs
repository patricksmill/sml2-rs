use super::*;

#[test]
fn music_uses_injected_bank_across_engine_reset() {
    // Song 1's header is at Bank 4 $605C. Synthetic transpose and empty
    // channel lists distinguish supplied content from the local ROM oracle.
    let mut bank = vec![0; 0x4000];
    bank[0x605C - 0x4000] = 7;
    let mut driver = SoundDriver::with_music_bank(bank);
    for _ in 0..2 {
        driver.reset_engine();
        driver.music_request = 1;
        driver.update_sound();
        assert_eq!(driver.music_sequencer.transpose, 7);
        assert_eq!(driver.active_song, 1);
    }
}

#[test]
fn test_pointer_lookup_contract() {
    // ID 0 is invalid
    assert_eq!(
        lookup_sfx_pointer(&SLOT0_START_TABLE, 0),
        Err(SfxLookupError::ZeroId)
    );

    // Slot 0 (Pulse 1) Start pointers: IDs 1..=24
    assert_eq!(lookup_sfx_pointer(&SLOT0_START_TABLE, 1), Ok(0x4127));
    assert_eq!(lookup_sfx_pointer(&SLOT0_START_TABLE, 6), Ok(0x41F3));
    assert_eq!(lookup_sfx_pointer(&SLOT0_START_TABLE, 7), Ok(0x4217));
    assert_eq!(lookup_sfx_pointer(&SLOT0_START_TABLE, 24), Ok(0x462C));

    // Slot 0 (Pulse 1) Tick pointers: IDs 1..=24
    assert_eq!(lookup_sfx_pointer(&SLOT0_TICK_TABLE, 1), Ok(0x412D));
    assert_eq!(lookup_sfx_pointer(&SLOT0_TICK_TABLE, 6), Ok(0x41F9));
    assert_eq!(lookup_sfx_pointer(&SLOT0_TICK_TABLE, 7), Ok(0x421D));
    assert_eq!(lookup_sfx_pointer(&SLOT0_TICK_TABLE, 24), Ok(0x463A));

    // Out of bounds ID 25
    assert_eq!(
        lookup_sfx_pointer(&SLOT0_START_TABLE, 25),
        Err(SfxLookupError::IdOutOfBounds { id: 25, max: 24 })
    );

    // Slot 1 (Wave) pointers: IDs 1..=24
    assert_eq!(lookup_sfx_pointer(&SLOT1_START_TABLE, 1), Ok(0x475A));
    assert_eq!(lookup_sfx_pointer(&SLOT1_START_TABLE, 20), Ok(0x4CE1));
    assert_eq!(lookup_sfx_pointer(&SLOT1_START_TABLE, 24), Ok(0x4DEB));
    assert_eq!(lookup_sfx_pointer(&SLOT1_TICK_TABLE, 1), Ok(0x4768));
    assert_eq!(lookup_sfx_pointer(&SLOT1_TICK_TABLE, 24), Ok(0x4D41));

    // Slot 2 (Noise) pointers: IDs 1..=11
    assert_eq!(lookup_sfx_pointer(&SLOT2_START_TABLE, 1), Ok(0x4EB3));
    assert_eq!(lookup_sfx_pointer(&SLOT2_START_TABLE, 11), Ok(0x511C));
    assert_eq!(lookup_sfx_pointer(&SLOT2_TICK_TABLE, 1), Ok(0x4EB9));
    assert_eq!(lookup_sfx_pointer(&SLOT2_TICK_TABLE, 11), Ok(0x5122));
    assert_eq!(
        lookup_sfx_pointer(&SLOT2_START_TABLE, 12),
        Err(SfxLookupError::IdOutOfBounds { id: 12, max: 11 })
    );
}

#[test]
fn test_busy_checks_and_exceptions() {
    let mut driver = SoundDriver::new();

    // Slot 0: busy blocks standard request ID 1
    driver.pulse_effect.busy = 1;
    driver.sfx_request_0 = 1;
    driver.update_sound();
    assert_eq!(driver.pulse_effect.active_effect, 0);

    // Slot 0: all 8 exception IDs bypass busy
    for &id in &SLOT0_BUSY_EXCEPTIONS {
        driver.pulse_effect.busy = 1;
        driver.pulse_effect.active_effect = 0;
        driver.stomp_chain = 1;
        driver.sfx_request_0 = id;
        driver.update_sound();
        assert_eq!(
            driver.pulse_effect.active_effect, id,
            "exception ID {id} must bypass busy"
        );
    }

    // Slot 0: vibrato timer bypasses busy
    driver.pulse_effect.busy = 1;
    driver.pulse_effect.active_effect = 0;
    driver.pulse_music.vibrato_timer = 3;
    driver.sfx_request_0 = 1;
    driver.update_sound();
    assert_eq!(driver.pulse_effect.active_effect, 1);
    driver.pulse_music.vibrato_timer = 0;

    // Slot 1: busy blocks ID 1
    driver.wave_slot.busy = 1;
    driver.sfx_request_1 = 1;
    driver.update_sound();
    assert_eq!(driver.wave_slot.active_effect, 0);

    // Slot 1: exception ID 20 (0x14) bypasses busy
    driver.sfx_request_1 = SLOT1_BUSY_EXCEPTION;
    driver.update_sound();
    assert_eq!(driver.wave_slot.active_effect, SLOT1_BUSY_EXCEPTION);

    // Slot 2: busy blocks all IDs (no exception)
    driver.noise_slot.busy = 1;
    driver.sfx_request_2 = 1;
    driver.update_sound();
    assert_eq!(driver.noise_slot.active_effect, 0);
}

#[test]
fn test_update_sound_clears_requests_on_exit() {
    let mut driver = SoundDriver::new();

    driver.sfx_request_0 = 7;
    driver.sfx_request_1 = 24;
    driver.sfx_request_2 = 2;
    driver.music_request = 0;
    driver.sound_mode = 0;

    driver.update_sound();

    // All request fields must be cleared per $10067
    assert_eq!(driver.sfx_request_0, 0);
    assert_eq!(driver.sfx_request_1, 0);
    assert_eq!(driver.sfx_request_2, 0);
    assert_eq!(driver.music_request, 0);
    assert_eq!(driver.sound_mode, 0);
    assert_eq!(driver.update_count, 1);
}

#[test]
fn test_master_volume_low_time_flag() {
    assert_eq!(MASTER_VOLUME_NORMAL, 0x77);
    assert_eq!(MASTER_VOLUME_LOW_TIME, 0x33);

    let mut driver = SoundDriver::new();

    // Normal volume
    driver.low_time_flag = false;
    driver.update_sound();
    assert_eq!(driver.master_volume, 0x77);

    // Low time volume
    driver.low_time_flag = true;
    driver.update_sound();
    assert_eq!(driver.master_volume, 0x33);
}

#[test]
fn test_pulse_effects_6_and_7_sequencing_through_driver() {
    let mut driver = SoundDriver::new();

    // Request effect 6 (castle transformation)
    driver.sfx_request_0 = 6;
    driver.update_sound();

    assert_eq!(driver.pulse_effect.active_effect, 6);
    assert!(driver.pulse_music.flags.effect_owns_channel());
    assert_eq!(
        driver.pulse_writes,
        vec![
            PulseWrite::Sweep(0x00),
            PulseWrite::DutyLength(0xBA),
            PulseWrite::Envelope(0xC0),
            PulseWrite::FrequencyLow(0xB6),
            PulseWrite::FrequencyControl(0xC6),
        ]
    );

    // Request effect 7 (burst / coin bounce)
    driver.pulse_writes.clear();
    driver.sfx_request_0 = 7;
    driver.update_sound();

    assert_eq!(driver.pulse_effect.active_effect, 7);
    assert_eq!(
        driver.pulse_writes,
        vec![
            PulseWrite::Sweep(0x00),
            PulseWrite::DutyLength(0x80),
            PulseWrite::Envelope(0xA7),
            PulseWrite::FrequencyLow(0x83),
            PulseWrite::FrequencyControl(0x87),
        ]
    );
}

#[test]
fn test_sound_modes_and_pause() {
    let mut driver = SoundDriver::new();

    // Mode 1 pauses sound
    driver.sound_mode = 1;
    driver.sfx_request_0 = 6;
    driver.update_sound();
    assert!(driver.sound_paused);
    assert_eq!(driver.sfx_request_0, 0);
    assert_eq!(driver.pulse_effect.active_effect, 0); // Not dispatched when paused

    // Paused state ignores new requests
    driver.sfx_request_0 = 7;
    driver.update_sound();
    assert_eq!(driver.sfx_request_0, 0);
    assert_eq!(driver.pulse_effect.active_effect, 0);

    // Mode 2 unpauses sound
    driver.sound_mode = 2;
    driver.update_sound();
    assert!(!driver.sound_paused);

    // Now requests dispatch normally
    driver.sfx_request_0 = 7;
    driver.update_sound();
    assert_eq!(driver.pulse_effect.active_effect, 7);
}

#[test]
fn test_noise_dispatch_and_lifecycle_in_driver() {
    let mut driver = SoundDriver::new();

    // Frame 1: Request Noise Effect 1
    driver.sfx_request_2 = 1;
    driver.update_sound();
    assert_eq!(driver.sfx_request_2, 0); // Request consumed per $10067
    assert_eq!(driver.noise_slot.active_effect, 1);
    assert_eq!(driver.noise_slot.timer, 2);
    assert!(driver.noise_music.effect_owns_channel());
    assert_eq!(driver.noise_writes.len(), 4);

    // Frame 2: Tick (timer 2 -> 1)
    driver.noise_writes.clear();
    driver.update_sound();
    assert_eq!(driver.noise_slot.active_effect, 1);
    assert_eq!(driver.noise_slot.timer, 1);
    assert!(driver.noise_writes.is_empty());

    // Frame 3: Step advance to 1 (timer reloaded to 2)
    driver.update_sound();
    assert_eq!(driver.noise_slot.step, 1);
    assert_eq!(driver.noise_slot.timer, 2);
    assert_eq!(driver.noise_writes.len(), 4);
}

#[test]
fn test_music_wave_pattern_is_kept_for_the_slot1_restore() {
    // $9D on channel 3 stores the wave-pattern pointer; the slot-1 stop
    // ($10E1D) restores wave RAM from it, so the driver must keep the music's
    // bytes or the restore silences the wave channel.
    let mut driver = SoundDriver::new();
    assert_eq!(driver.wave_music.wave_pattern, [0u8; 16]);

    driver.music_request = 1;
    driver.update_sound(); // start + first tick: song 1's ch3 $9D
    let pattern = driver.wave_music.wave_pattern;
    assert_ne!(pattern, [0u8; 16], "song 1 uploads a wave pattern");
    assert!(driver
        .music_writes
        .contains(&MusicWrite::Ch3WavePattern(pattern)));

    // The pointer (and so the pattern) survives later frames.
    driver.update_sound();
    assert_eq!(driver.wave_music.wave_pattern, pattern);
}

#[test]
fn test_reset_engine_clears_every_slot_and_flag() {
    let mut driver = SoundDriver::new();
    driver.music_request = 1;
    driver.update_sound();
    driver.update_sound();

    // Dirty every byte $13F6B clears.
    driver.pulse_effect.active_effect = 5;
    driver.pulse_effect.busy = 1;
    driver.pulse_effect.phase = 2;
    driver.pulse_music.flags.claim_for_effect();
    driver.pulse_music.vibrato_timer = 9;
    driver.wave_slot.active_effect = 4;
    driver.wave_slot.variant = 1;
    driver.wave_music.flags.claim_for_effect();
    driver.noise_slot.active_effect = 2;
    driver.noise_music.claim_for_effect();
    driver.sound_mode = 1;
    driver.low_time_flag = true;
    driver.music_sequencer.drum_mode = 1;
    driver.music_sequencer.channels[0].slide_state = 1;
    driver.music_sequencer.channels[0].slide_step = 3;
    driver.music_sequencer.channels[0].bend_flag = 1;

    driver.reset_engine();

    assert_eq!(driver.pulse_effect.active_effect, 0);
    assert_eq!(driver.pulse_effect.busy, 0);
    assert_eq!(driver.pulse_effect.phase, 0);
    assert!(!driver.pulse_music.flags.effect_owns_channel());
    assert_eq!(driver.pulse_music.vibrato_timer, 0);
    assert_eq!(driver.wave_slot.active_effect, 0);
    assert!(!driver.wave_music.flags.effect_owns_channel());
    assert_eq!(driver.noise_slot.active_effect, 0);
    assert!(!driver.noise_music.effect_owns_channel());
    assert_eq!(driver.sound_mode, 0);
    assert!(!driver.low_time_flag);
    assert_eq!(driver.music_sequencer.drum_mode, 0);
    assert_eq!(
        (
            driver.music_sequencer.channels[0].slide_state,
            driver.music_sequencer.channels[0].slide_step,
            driver.music_sequencer.channels[0].bend_flag
        ),
        (0, 0, 0)
    );
    // Not in $13F6B's list: these survive, like the ROM's bytes.
    assert_eq!(driver.wave_slot.variant, 1, "$A53E survives");
    assert_ne!(driver.active_song, 0, "sCurSong is $13F5E's job");
}

#[test]
fn test_slot0_reset_effects_stop_the_song() {
    // $42E2: effect 12 starts with `call $13F5E`, so the song and every slot
    // go away before the effect's own setup runs.
    let mut driver = SoundDriver::new();
    driver.music_request = 1;
    driver.update_sound();
    driver.update_sound();
    assert_ne!(driver.active_song, 0, "song 1 is playing");

    driver.wave_slot.active_effect = 4;
    driver.low_time_flag = true;
    driver.sfx_request_0 = 12;
    driver.update_sound();

    assert_eq!(driver.active_song, 0, "effect 12 stops the song");
    assert_eq!(driver.wave_slot.active_effect, 0, "other slots are cleared");
    assert!(!driver.low_time_flag);
    assert_eq!(driver.pulse_effect.active_effect, 12);
}

#[test]
fn test_song_start_frame_runs_the_sequencer() {
    // $10078 falls through to $10064 (call $11638): the frame that starts a
    // song still ticks it, so its first note is not delayed by one frame.
    let mut driver = SoundDriver::new();
    driver.music_request = 1;
    driver.update_sound();
    assert_eq!(driver.active_song, 1);
    assert!(
        driver
            .music_writes
            .iter()
            .any(|w| matches!(w, MusicWrite::Ch1FrequencyControl(_))),
        "song 1's first note must reach the registers on the start frame"
    );
}

/// `$273E` writes `sSoundMode = 1`. `$118F7` then silences every channel
/// through `$13FE7`, drops the three slot effects, flags `sSoundPaused`, loads
/// the first pause beep and clears the one-shot mode through `$10067`.
#[test]
fn pause_mode_stops_the_engine_and_starts_the_beep() {
    let mut driver = SoundDriver::new();
    driver.music_request = 1;
    driver.update_sound();
    driver.update_sound();
    assert_ne!(driver.active_song, 0, "a song is playing");
    let song = driver.music_sequencer.clone();

    driver.sound_mode = 1;
    driver.update_sound();

    assert!(driver.sound_paused, "the engine stops");
    assert_eq!(driver.sound_mode, 0, "$10067 clears the one-shot mode");
    assert_eq!(driver.active_song, song.cur_song, "the song stays loaded");
    // `$13FE7`'s eight register writes.
    assert!(driver.pulse_writes.contains(&PulseWrite::Envelope(8)));
    assert!(driver.pulse_writes.contains(&PulseWrite::Ch2Envelope(8)));
    assert!(driver
        .pulse_writes
        .contains(&PulseWrite::FrequencyControl(0x80)));
    assert!(driver
        .pulse_writes
        .contains(&PulseWrite::Ch2FrequencyControl(0x80)));
    assert!(driver.pulse_writes.contains(&PulseWrite::Sweep(0)));
    assert!(driver.noise_writes.contains(&NoiseWrite::Envelope(8)));
    assert!(driver.noise_writes.contains(&NoiseWrite::Control(0x80)));
    assert!(driver.wave_writes.contains(&WaveWrite::Enable(0)));
    // `$5970`'s beep, on pulse 2, with its 5-frame budget.
    assert!(driver
        .pulse_writes
        .contains(&PulseWrite::Ch2DutyLength(0x80)));
    assert!(driver.pulse_writes.contains(&PulseWrite::Ch2Envelope(0xA1)));
    assert!(driver
        .pulse_writes
        .contains(&PulseWrite::Ch2FrequencyLow(0xC1)));
    assert!(driver
        .pulse_writes
        .contains(&PulseWrite::Ch2FrequencyControl(0xC7)));
    assert_eq!(driver.pause_beep_timer, 5);
    assert_eq!(driver.pause_beep_index, 0);

    // Paused frames never touch the song.
    for _ in 0..3 {
        driver.update_sound();
        assert_eq!(driver.music_sequencer, song, "the song holds");
        assert!(driver.music_writes.is_empty());
    }
}

/// `$11936`: the paused engine ticks `sPauseBeepTimer`; on expiry it steps
/// `sPauseBeepIndex` and plays that entry, and index 6 parks a 240-frame hold.
#[test]
fn the_pause_beep_walks_its_five_entries_then_holds() {
    let mut driver = SoundDriver::new();
    driver.sound_mode = 1;
    driver.update_sound();
    // (index reached, timer, the NR23 byte that entry writes)
    let expected = [
        (1u8, 5u8, 0xBEu8), // $5975
        (2, 5, 0xC1),       // $5970
        (3, 12, 0xD1),      // $597A
        (4, 7, 0xB6),       // $597F
        (5, 15, 0xBA),      // $5984
    ];
    for (index, timer, low) in expected {
        let budget = driver.pause_beep_timer as usize;
        for frame in 0..budget {
            driver.update_sound();
            if frame + 1 < budget {
                assert_eq!(
                    (driver.pause_beep_index, driver.pause_beep_timer),
                    (index - 1, (budget - frame - 1) as u8),
                    "the beep holds its pitch until its budget runs out"
                );
            }
        }
        assert_eq!(driver.pause_beep_index, index);
        assert_eq!(driver.pause_beep_timer, timer);
        assert!(
            driver
                .pulse_writes
                .contains(&PulseWrite::Ch2FrequencyLow(low)),
            "entry {index} writes NR23 = ${low:02X}"
        );
    }
    // The fifth entry's expiry is the long hold (`$11959`).
    for _ in 0..driver.pause_beep_timer {
        driver.update_sound();
    }
    assert_eq!(driver.pause_beep_index, 5);
    assert_eq!(driver.pause_beep_timer, 240);
    assert!(
        !driver
            .pulse_writes
            .iter()
            .any(|w| matches!(w, PulseWrite::Ch2FrequencyLow(_))),
        "the hold writes no new beep"
    );
}

/// `$11926`: mode 2 unpauses, clears the three `sSfx*Busy` bytes and falls into
/// `$10044`, so a request pending on the resume frame still runs.
#[test]
fn resume_mode_unpauses_and_runs_the_same_frame() {
    let mut driver = SoundDriver::new();
    driver.sound_mode = 1;
    driver.update_sound();
    assert!(driver.sound_paused);

    driver.pulse_effect.busy = 1;
    driver.wave_slot.busy = 1;
    driver.noise_slot.busy = 1;
    driver.music_request = 1;
    driver.sound_mode = 2;
    driver.update_sound();

    assert!(!driver.sound_paused);
    assert_eq!(driver.sound_mode, 0);
    assert_eq!(
        (
            driver.pulse_effect.busy,
            driver.wave_slot.busy,
            driver.noise_slot.busy
        ),
        (0, 0, 0),
        "sSfx0Busy/sSfx1Busy/sSfx2Busy"
    );
    assert_ne!(
        driver.active_song, 0,
        "$11926 falls into $10044: the resume frame runs the music"
    );
}
