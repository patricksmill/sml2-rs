use super::*;

#[test]
fn test_song1_start_and_channel_initialization() {
    let mut seq = MusicSequencer::new();
    seq.start_song(1, SOUND_BANK04);

    assert_eq!(seq.cur_song(), 1);
    assert!(seq.is_playing());
    assert!(!seq.fast_music);
    assert_eq!(seq.transpose, 4);
    assert_eq!(seq.dur_table_ptr, 0x5C61);

    // All 4 channels active for Song 1
    assert!(seq.channels[0].active);
    assert!(seq.channels[1].active);
    assert!(seq.channels[2].active);
    assert!(seq.channels[3].active);

    // Initial stream pointers read from first word of each section pointer
    assert_eq!(seq.channels[0].stream_ptr, 0x60A7);
    assert_eq!(seq.channels[1].stream_ptr, 0x60AA);
    assert_eq!(seq.channels[2].stream_ptr, 0x60BF);
    assert_eq!(seq.channels[3].stream_ptr, 0x60D5);

    // Note timers initialized to 1 frame countdown
    assert_eq!(seq.channels[0].note_timer, 1);
    assert_eq!(seq.channels[1].note_timer, 1);
    assert_eq!(seq.channels[2].note_timer, 1);
    assert_eq!(seq.channels[3].note_timer, 1);
}

#[test]
fn test_fast_music_acceleration() {
    let mut seq = MusicSequencer::new();

    // Song 9 is in FAST_MUSIC_SONGS: activates fast_music and adds 4 to transpose
    seq.start_song(9, SOUND_BANK04);
    assert!(seq.fast_music);
    assert_eq!(seq.transpose, 20); // base 16 + 4

    // Song 2 explicitly resets fast_music to normal speed
    seq.start_song(2, SOUND_BANK04);
    assert!(!seq.fast_music);
    assert_eq!(seq.transpose, 4);

    // Song 1 is normal speed
    seq.start_song(1, SOUND_BANK04);
    assert!(!seq.fast_music);
    assert_eq!(seq.transpose, 4);
}

#[test]
fn test_stop_clears_playback_and_channels() {
    let mut seq = MusicSequencer::new();
    seq.start_song(1, SOUND_BANK04);
    assert!(seq.is_playing());

    seq.stop();
    assert_eq!(seq.cur_song(), 0);
    assert!(!seq.is_playing());
    assert!(!seq.channels.iter().any(|c| c.active));
}

#[test]
fn test_all_42_songs_start_and_tick_without_panic() {
    for song_id in 1..=42 {
        let mut seq = MusicSequencer::new();
        seq.start_song(song_id, SOUND_BANK04);

        assert_eq!(seq.cur_song(), song_id, "Song {song_id} failed to start");
        assert!(seq.is_playing(), "Song {song_id} should be playing");
        assert!(seq.channels.iter().any(|c| c.active), "Song {song_id} has no active channels");

        // Tick several frames through the sequencer
        let mut writes = Vec::new();
        for _ in 0..5 {
            writes.clear();
            seq.tick(SOUND_BANK04, [0, 0, 0], &mut writes);
        }
    }
}

#[test]
fn test_sfx_channel_masking() {
    let mut seq = MusicSequencer::new();
    seq.start_song(1, SOUND_BANK04);

    let mut unmasked_writes = Vec::new();
    seq.tick(SOUND_BANK04, [0, 0, 0], &mut unmasked_writes);

    // Restart and tick with SFX owning Channels 1, 3, 4
    let mut masked_seq = MusicSequencer::new();
    masked_seq.start_song(1, SOUND_BANK04);

    let mut masked_writes = Vec::new();
    masked_seq.tick(SOUND_BANK04, [1, 1, 1], &mut masked_writes);

    // Ch1, Ch3, and Ch4 register writes must be completely suppressed when masked
    for write in &masked_writes {
        match write {
            MusicWrite::Ch1Sweep(_)
            | MusicWrite::Ch1DutyLength(_)
            | MusicWrite::Ch1Envelope(_)
            | MusicWrite::Ch1FrequencyLow(_)
            | MusicWrite::Ch1FrequencyControl(_) => {
                panic!("Ch1 write emitted while SFX active: {write:?}");
            }
            MusicWrite::Ch3Enable(_)
            | MusicWrite::Ch3Length(_)
            | MusicWrite::Ch3Level(_)
            | MusicWrite::Ch3FrequencyLow(_)
            | MusicWrite::Ch3FrequencyControl(_)
            | MusicWrite::Ch3WavePattern(_) => {
                panic!("Ch3 write emitted while SFX active: {write:?}");
            }
            MusicWrite::Ch4Length(_)
            | MusicWrite::Ch4Envelope(_)
            | MusicWrite::Ch4Polynomial(_)
            | MusicWrite::Ch4Control(_)
            | MusicWrite::DrumTrigger => {
                panic!("Ch4 write emitted while SFX active: {write:?}");
            }
            _ => {} // Channel 2 writes are permitted
        }
    }
}

#[test]
fn test_music_sensitivity() {
    let mut seq = MusicSequencer::new();
    seq.start_song(1, SOUND_BANK04);

    let mut writes = Vec::new();
    seq.tick(SOUND_BANK04, [0, 0, 0], &mut writes);

    // Verify Channel 2 received its initial pitch write
    let ch2_freq = writes.iter().find_map(|w| match w {
        MusicWrite::Ch2FrequencyLow(lo) => Some(*lo),
        _ => None,
    });
    assert!(ch2_freq.is_some());
    let original_freq = ch2_freq.unwrap();

    // Mutating transpose by 1 semitone alters the frequency lookup output
    let mut mutated_seq = MusicSequencer::new();
    mutated_seq.start_song(1, SOUND_BANK04);
    mutated_seq.transpose += 1;

    let mut mutated_writes = Vec::new();
    mutated_seq.tick(SOUND_BANK04, [0, 0, 0], &mut mutated_writes);

    let mutated_freq = mutated_writes.iter().find_map(|w| match w {
        MusicWrite::Ch2FrequencyLow(lo) => Some(*lo),
        _ => None,
    });
    assert!(mutated_freq.is_some());
    assert_ne!(Some(original_freq), mutated_freq);
}
