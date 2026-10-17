use super::*;

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
