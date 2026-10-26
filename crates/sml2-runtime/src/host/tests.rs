use super::*;
use crate::session::SessionState;
use std::sync::atomic::{AtomicUsize, Ordering};

fn save_path(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "sml2_host_{name}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn runner(path: PathBuf) -> SessionRunner {
    SessionRunner::new(SessionAssets::load().unwrap(), path, 1, (160, 144), true)
}

#[test]
fn headless_autosave_failure_returns_the_destination_and_preserves_existing_data() {
    let blocked = save_path("blocked");
    fs::write(&blocked, b"existing data").unwrap();
    let destination = blocked.join("save.sav");
    let mut host = runner(destination.clone());
    let SessionState::Overworld(map) = &mut host.state else {
        panic!("map")
    };
    map.pending_save = true;
    let error = host.run_headless(1, None).unwrap_err().to_string();
    assert!(error.contains(&destination.display().to_string()));
    assert_eq!(fs::read(&blocked).unwrap(), b"existing data");
    fs::remove_file(blocked).unwrap();
}

#[test]
fn legacy_save_migrates_and_all_three_pipes_survive_explicit_and_automatic_saves() {
    use sml2_core::map_sim::MapSim;
    let path = save_path("slots");
    let record = MapSim::new().save_record(7, 0x12, 3);
    fs::write(&path, record).unwrap();
    let mut host = runner(path.clone());
    assert_eq!(host.active_slot, Some(0));
    assert_eq!(fs::read(&path).unwrap().len(), SAVE_IMAGE_LEN);
    host.session
        .slots
        .write_slot(1, &MapSim::new().save_record(8, 0x23, 4));
    host.session
        .slots
        .write_slot(2, &MapSim::new().save_record(9, 0x34, 5));
    for slot in 0..3 {
        host.session.active_slot = Some(slot);
        host.session.level_tally = 0x40 + slot;
        host.save_to_disk().unwrap();
        let image = fs::read(&path).unwrap();
        let persisted = SaveSlots::from_image(&image);
        assert_eq!(image.len(), SAVE_IMAGE_LEN);
        assert_eq!(persisted.slot(slot as usize).unwrap()[0x42], 0x40 + slot);
        for other in 0..3 {
            if other != slot as usize {
                assert_eq!(persisted.slot(other), host.slots.slot(other));
            }
        }
        host.session.slots = persisted;
        let SessionState::Overworld(map) = &mut host.state else {
            panic!("map")
        };
        map.pending_save = true;
        host.step_frame(PlayerKeys::default());
        assert_eq!(fs::read(&path).unwrap(), host.slots.image());
    }
    fs::remove_file(path).unwrap();
}

#[test]
fn loads_a_later_valid_pipe_and_leaves_malformed_files_untouched() {
    use sml2_core::map_sim::MapSim;
    let path = save_path("later_pipe");
    let record = MapSim::new().save_record(8, 0x23, 4);
    let mut slots = SaveSlots::new();
    slots.write_slot(2, &record);
    fs::write(&path, slots.image()).unwrap();
    let host = runner(path.clone());
    assert_eq!(host.active_slot, Some(2));
    assert_eq!(host.level_tally, 0x23);
    assert_eq!(host.slots.slot(2), Some(&record));
    for bytes in [b"bad".as_slice(), &[0; SLOT_LEN]] {
        fs::write(&path, bytes).unwrap();
        let mut host = runner(path.clone());
        assert!(!host.load_from_disk());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    fs::remove_file(path).unwrap();
}

#[test]
fn device_rate_drives_synthesis_and_recorded_wav_timing() {
    for rate in [44_100, 48_000] {
        let mut host = runner(save_path("audio_save"));
        let wav = save_path("audio_wav");
        host.audio_sync = Some(AudioClock::new(rate));
        host.audio_out_path = Some(wav.clone());
        host.session.sound_driver.music_request = 1;
        let mut expected_apu = host.apu.clone();
        for _ in 0..60 {
            host.step_frame(PlayerKeys::default());
            expected_apu.ingest_driver(&host.session.sound_driver);
            let mut expected_samples = Vec::new();
            expected_apu.generate_samples(rate, rate as usize / 60, &mut expected_samples);
            assert_eq!(host.audio_samples(), expected_samples);
            assert_eq!(host.apu, expected_apu);
        }
        host.save_recorded_audio().unwrap();
        let bytes = fs::read(&wav).unwrap();
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), rate);
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            rate * 4
        );
        assert_eq!(bytes.len(), 44 + rate as usize * 4);
        fs::remove_file(wav).unwrap();
    }
}

#[test]
fn headless_recording_retains_console_cadence_at_44100_hz() {
    let mut host = runner(save_path("headless_audio_save"));
    let wav = save_path("headless_audio_wav");
    host.audio_out_path = Some(wav.clone());
    host.run_headless(60, None).unwrap();
    let bytes = fs::read(&wav).unwrap();
    let frames = 60u64 * 70_224 * 44_100 / 4_194_304;
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        44_100
    );
    assert_eq!(bytes.len(), 44 + frames as usize * 4);
    fs::remove_file(wav).unwrap();
}

#[test]
fn host_view_updates_spawn_and_autoscroll_margins_before_each_tick() {
    for level in [0, 19] {
        let mut host = SessionRunner::new(
            SessionAssets::load().unwrap(),
            save_path("view"),
            1,
            (320, 224),
            true,
        );
        host.begin_level(level, 0).unwrap();
        for (view, margin, bias) in [
            ((320, 224), (80, 40), 20),
            ((160, 144), (0, 0), 0),
            ((480, 144), (160, 0), 0),
        ] {
            (host.vw, host.vh) = view;
            host.step_frame(PlayerKeys::default());
            let SessionState::InLevel(ctx) = &host.state else {
                panic!("level")
            };
            assert_eq!(ctx.obj.view_margin, margin);
            assert_eq!(ctx.obj.view_bias_y, bias);
            assert_eq!(
                ctx.sim.screen_margin,
                if level == 19 { margin.0 } else { 0 }
            );
            assert_eq!(host.render_frame().len(), view.0 * view.1);
        }
    }
}
