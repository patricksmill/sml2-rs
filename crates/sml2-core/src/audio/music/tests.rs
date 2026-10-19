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

use super::rom_model::RomSeq;

const ROM_WALK_FRAMES: usize = 600;

/// `$7F52` is a real section (`$7F52 -> $7F58`, the shared `$9B $20 / $A5 $01 /
/// $9C` silent rest loop), not a "no channel" sentinel, so these songs' headers
/// point a channel at a stream the port currently refuses to start. Every other
/// song must match the ROM walk frame for frame; fixing `init_section` shrinks
/// this list, and the constant is updated with it.
const KNOWN_DIVERGENT: [u8; 7] = [8, 13, 27, 28, 30, 31, 40];

fn walk_mismatch(song: u8, frames: usize) -> Option<String> {
    let mut rom = RomSeq::new(song, song == 9);
    let mut port = MusicSequencer::new();
    port.start_song(song, SOUND_BANK04);
    let mut writes = Vec::new();
    for frame in 0..frames {
        rom.tick();
        writes.clear();
        port.tick(SOUND_BANK04, [0, 0, 0], &mut writes);
        if rom.cur != port.cur_song() {
            return Some(format!(
                "song {song} frame {frame}: sCurSong {} vs {}",
                rom.cur,
                port.cur_song()
            ));
        }
        if rom.transpose != port.transpose as u8 || rom.dur != port.dur_table_ptr {
            return Some(format!(
                "song {song} frame {frame}: transpose/dur {:02x}/{:04x} vs {:02x}/{:04x}",
                rom.transpose, rom.dur, port.transpose as u8, port.dur_table_ptr
            ));
        }
        for (i, (rc, pc)) in rom.channels.iter().zip(port.channels.iter()).enumerate() {
            if rc.active() != pc.active
                || rc.stream != pc.stream_ptr
                || rc.timer != pc.note_timer
                || rc.duration != pc.note_duration
                || rc.freq_lo != pc.freq_lo
                || rc.freq_hi != pc.freq_hi
                || rc.rest != u8::from(pc.rest)
            {
                return Some(format!(
                    "song {song} frame {frame} ch{}: rom active={} stream={:#06x} timer={} dur={} freq={:02x}{:02x} rest={} | port active={} stream={:#06x} timer={} dur={} freq={:02x}{:02x} rest={}",
                    i + 1,
                    u8::from(rc.active()), rc.stream, rc.timer, rc.duration, rc.freq_lo, rc.freq_hi, rc.rest,
                    u8::from(pc.active), pc.stream_ptr, pc.note_timer, pc.note_duration, pc.freq_lo, pc.freq_hi, u8::from(pc.rest),
                ));
            }
        }
    }
    None
}

#[test]
fn test_song_walk_matches_rom_except_known_silent_channel_songs() {
    let mut divergent = Vec::new();
    let mut details = Vec::new();
    for song in 1..=42u8 {
        if let Some(msg) = walk_mismatch(song, ROM_WALK_FRAMES) {
            divergent.push(song);
            details.push(msg);
        }
    }
    assert_eq!(
        divergent,
        KNOWN_DIVERGENT.to_vec(),
        "divergent songs {divergent:?} (expected {KNOWN_DIVERGENT:?}):\n{}",
        details.join("\n")
    );
    for &song in KNOWN_DIVERGENT.iter() {
        assert!(
            walk_mismatch(song, ROM_WALK_FRAMES).is_some(),
            "song {song} now matches the ROM walk; drop it from KNOWN_DIVERGENT"
        );
    }
}

/// Frames on which the console's `$115CD` fires: the ending channel's section
/// word has a zero high byte. Measured from the ROM walk over the real song
/// data; song 12 is the one the port used to never stop.
const ROM_END_FRAMES: [(u8, usize); 10] = [
    (2, 126),
    (3, 240),
    (4, 864),
    (8, 4000),
    (9, 168),
    (12, 3060),
    (15, 411),
    (22, 580),
    (25, 165),
    (26, 1095),
];

#[test]
fn test_song_end_frames_match_rom() {
    let mut writes = Vec::new();
    for (song, expected) in ROM_END_FRAMES {
        let mut rom = RomSeq::new(song, song == 9);
        let mut port = MusicSequencer::new();
        port.start_song(song, SOUND_BANK04);
        let mut rom_end = None;
        let mut port_end = None;
        for frame in 0..expected + 40 {
            rom.tick();
            writes.clear();
            port.tick(SOUND_BANK04, [0, 0, 0], &mut writes);
            if rom_end.is_none() && rom.cur == 0 {
                rom_end = Some(frame);
            }
            if port_end.is_none() && port.cur_song() == 0 {
                port_end = Some(frame);
            }
        }
        assert_eq!(rom_end, Some(expected), "song {song}: ROM walk end frame drifted");
        assert_eq!(port_end, Some(expected), "song {song}: port end frame");
    }
}

#[test]
fn test_noise_note_01_reads_its_record_like_the_rom() {
    // $11728 routes channel 4 to $1176A before the note-$01 rest test, so the
    // byte is an offset into $5C06 and the record's registers are written; the
    // rest flag is never touched.
    let mut bank = vec![0u8; 0x4000];
    bank[0x1000] = 0x01; // stream at CPU $5000
    let rec = 0x1C06 + 1; // CPU $5C06 + note 1
    bank[rec..rec + 5].copy_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55]);

    let mut ch = MusicChannel::new(4);
    ch.active = true;
    ch.section_ptr = 0x5000;
    ch.stream_ptr = 0x5000;
    ch.note_timer = 1;
    ch.note_duration = 0x18;
    ch.rest = true;

    let mut transpose = 0i8;
    let mut dur = 0x5C61u16;
    let mut drum = 0u8;
    let mut out = Vec::new();
    ch.tick(&bank, &mut transpose, &mut dur, &mut drum, false, &mut out);

    assert_eq!(
        out,
        vec![
            MusicWrite::Ch4Length(0x33),
            MusicWrite::Ch4Envelope(0x11),
            MusicWrite::Ch4Polynomial(0x44),
            MusicWrite::Ch4Control(0xD5),
        ]
    );
    assert!(ch.rest, "the noise note must not clear or set the rest flag");
    assert_eq!(ch.note_timer, 0x18);
}
