use super::catalog::Pedestal;
use super::*;
#[test]
fn the_sign_spells_the_station_in_game_letters() {
    let settings = Settings::default();
    let debug = false;
    assert_eq!(catalog::sign(Pedestal::Zoom, &settings, debug).0, "ZOOM");
    assert_eq!(catalog::sign(Pedestal::Zoom, &settings, debug).1, "AUTO");
    assert_eq!(
        catalog::sign(Pedestal::Volume, &settings, debug),
        ("VOL".into(), "70".into())
    );
    assert_eq!(catalog::sign(Pedestal::Mute, &settings, debug).1, "OFF");
    let mut muted = settings.clone();
    catalog::cycle(Pedestal::Mute, &mut muted, &mut false);
    assert_eq!(catalog::sign(Pedestal::Mute, &muted, false).1, "ON");
    let mut hall = Hall::enter();
    hall.mario = pedestal_x(3);
    assert_eq!(sign_station(&hall), Some(3));
    hall.walk = -32;
    assert_eq!(
        sign_station(&hall),
        Some(4),
        "a left march shows the station he is walking toward"
    );
}

#[test]
fn the_walk_uses_the_file_select_frames() {
    let mut hall = Hall::enter();
    assert_eq!(sprite_frame(&hall), 4, "standing, facing left");
    let mut settings = Settings::default();
    let mut debug = false;
    step(&mut hall, true, false, false, &mut settings, &mut debug);
    assert_eq!(sprite_frame(&hall), 5, "left march frame 0");
    for _ in 0..4 {
        step(&mut hall, false, false, false, &mut settings, &mut debug);
    }
    assert_eq!(sprite_frame(&hall), 6, "walk_timer 4 advances the frame");
}

#[test]
fn a_held_direction_marches_one_pipe_and_stops_on_the_lattice() {
    let mut hall = Hall::enter();
    let mut settings = Settings::default();
    let mut debug = false;
    assert!(!step(
        &mut hall,
        true,
        false,
        false,
        &mut settings,
        &mut debug
    ));
    assert_eq!(hall.mario, DOOR, "the decision frame does not slide");
    assert_eq!(hall.walk, -STEP as i8);
    for _ in 0..31 {
        assert!(!step(
            &mut hall,
            false,
            false,
            false,
            &mut settings,
            &mut debug
        ));
    }
    assert_ne!(hall.walk, 0, "releasing does not cancel the march");
    assert!(!step(
        &mut hall,
        false,
        false,
        false,
        &mut settings,
        &mut debug
    ));
    assert_eq!(hall.mario, pedestal_x(0));
    assert_eq!(hall.walk, 0);
    let rested = hall.mario;
    assert!(!step(
        &mut hall,
        false,
        false,
        false,
        &mut settings,
        &mut debug
    ));
    assert_eq!(hall.mario, rested, "he stands on the pipe");
    assert!((0..160).contains(&(hall.mario - hall.camera)));
}

#[test]
fn every_march_stays_on_the_native_screen_and_a_click_hops_on_arrival() {
    let mut hall = Hall::enter();
    let mut settings = Settings::default();
    let mut debug = false;
    for _ in 0..catalog::HALL_PEDESTALS.len() {
        for _ in 0..33 {
            assert!(!step(
                &mut hall,
                true,
                false,
                false,
                &mut settings,
                &mut debug
            ));
            let screen = hall.mario - hall.camera;
            assert!(
                (0..160).contains(&screen),
                "screen x {screen} at mario {} camera {}",
                hall.mario,
                hall.camera
            );
        }
    }
    assert_eq!(hall.mario, furthest());
    assert_eq!(hall.walk, 0);
    step(&mut hall, true, false, false, &mut settings, &mut debug);
    assert_eq!(
        hall.mario,
        furthest(),
        "the last pipe does not march further"
    );
    assert_eq!(hall.walk, 0);

    let mut hall = Hall::enter();
    hall.goal = Some(0);
    step(&mut hall, false, false, false, &mut settings, &mut debug);
    for _ in 0..31 {
        step(&mut hall, false, false, false, &mut settings, &mut debug);
    }
    assert_eq!(hall.hop, 0);
    step(&mut hall, false, false, false, &mut settings, &mut debug);
    assert_eq!(hall.mario, pedestal_x(0));
    assert!(hall.hop > 0);
    assert_eq!(settings.zoom, crate::settings::Zoom::Integer(1));
}

#[test]
fn walking_left_reaches_volume_and_a_jump_raises_it() {
    let mut hall = Hall::enter();
    let mut settings = Settings::default();
    let mut debug = false;
    let volume_at = pedestal_x(
        catalog::PEDESTALS
            .iter()
            .position(|p| *p == Pedestal::Volume)
            .unwrap(),
    );
    for _ in 0..200 {
        if hall.mario == volume_at {
            break;
        }
        assert!(!step(
            &mut hall,
            true,
            false,
            false,
            &mut settings,
            &mut debug
        ));
    }
    assert_eq!(hall.mario, volume_at);
    assert!(!step(
        &mut hall,
        false,
        false,
        true,
        &mut settings,
        &mut debug
    ));
    assert!((settings.volume - 0.8).abs() < f32::EPSILON);
    assert!(hall.hop > 0);
    let during_hop = settings.volume;
    step(&mut hall, false, false, true, &mut settings, &mut debug);
    assert!(
        (settings.volume - during_hop).abs() < f32::EPSILON,
        "the hop ignores A"
    );
}

#[test]
fn walking_right_through_the_door_leaves() {
    let mut hall = Hall::enter();
    let mut settings = Settings::default();
    let mut debug = false;
    assert!(step(
        &mut hall,
        false,
        true,
        false,
        &mut settings,
        &mut debug
    ));
}

fn test_assets() -> sml2_core::front_end::FrontEndAssets {
    use sml2_core::front_end::{DEMO_TAPES_LEN, TILEMAP_LEN, VRAM_LEN};
    sml2_core::front_end::FrontEndAssets::new(
        vec![0u8; VRAM_LEN],
        vec![0u8; TILEMAP_LEN],
        vec![0u8; VRAM_LEN],
        vec![0u8; TILEMAP_LEN],
        vec![0u8; DEMO_TAPES_LEN],
    )
    .unwrap()
}

fn tables() -> sml2_core::object::ObjectTables {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extracted/objects.json"
    )))
    .expect("object tables")
}

#[test]
fn the_doorway_frame_is_palette_indices() {
    let hall = Hall::enter();
    assert_eq!(hall.mario, DOOR);
    let frame = pixels(
        &hall,
        &test_assets(),
        &tables(),
        &[],
        &Settings::default(),
        false,
    );
    let decoded: Vec<_> = frame
        .iter()
        .copied()
        .map(sml2_core::video::PalettePixel::from_token)
        .collect();
    assert_eq!(decoded.len(), frame.len());
}

#[test]
fn every_option_has_a_complete_sign_with_supported_game_glyphs() {
    let mut settings = Settings::default();
    for (pedestal, name, value) in [
        (Pedestal::Zoom, "ZOOM", "AUTO"),
        (Pedestal::Fullscreen, "FULL", "OFF"),
        (Pedestal::Filter, "FILTER", "GRAY"),
        (Pedestal::Volume, "VOL", "70"),
        (Pedestal::Mute, "MUTE", "OFF"),
        (Pedestal::DeadZone, "DEAD", "30"),
        (Pedestal::Widescreen, "WIDE", "OFF"),
        (Pedestal::CameraZoom, "CAM", "100"),
        (Pedestal::Motion, "SMOOTH", "OFF"),
        (Pedestal::DxMode, "DX", "OFF"),
        (Pedestal::Toolbar, "BAR", "ON"),
        (Pedestal::Debug, "DEBUG", "OFF"),
    ] {
        assert_eq!(
            catalog::sign(pedestal, &settings, false),
            (name.into(), value.into())
        );
    }
    for filter in crate::settings::Filter::ALL {
        settings.filter = filter;
        let (_, value) = catalog::sign(Pedestal::Filter, &settings, false);
        assert!(value
            .bytes()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == b' '));
        assert!(value.len() <= 6);
    }
    settings.filter = crate::settings::Filter::Sgb(0);
    assert_eq!(
        catalog::sign(Pedestal::Filter, &settings, false).1,
        "SGB 1A"
    );
    settings.filter = crate::settings::Filter::Sgb(31);
    assert_eq!(
        catalog::sign(Pedestal::Filter, &settings, false).1,
        "SGB 4H"
    );
    settings.zoom = crate::settings::Zoom::Integer(8);
    settings.volume = 1.0;
    settings.widescreen = crate::settings::Widescreen::Wide16x9;
    settings.camera_zoom = crate::settings::CameraZoom::Zoom200;
    assert_eq!(catalog::sign(Pedestal::Zoom, &settings, false).1, "8");
    assert_eq!(catalog::sign(Pedestal::Volume, &settings, false).1, "100");
    assert_eq!(
        catalog::sign(Pedestal::Widescreen, &settings, false).1,
        "169"
    );
    assert_eq!(
        catalog::sign(Pedestal::CameraZoom, &settings, false).1,
        "200"
    );
}

#[test]
fn signs_stay_visible_through_every_march_and_the_door_returns_to_its_map_position() {
    let mut hall = Hall::enter();
    let mut settings = Settings::default();
    let mut debug = false;
    for (left, right) in [(true, false), (false, true)] {
        for _ in 0..catalog::HALL_PEDESTALS.len() * 33 {
            assert!(!step(
                &mut hall,
                left,
                right,
                false,
                &mut settings,
                &mut debug
            ));
            if let Some(index) = sign_station(&hall) {
                let (name, value) = catalog::sign(catalog::HALL_PEDESTALS[index], &settings, debug);
                let center = pedestal_x(index) - hall.camera;
                for text in [name, value] {
                    let half = text.len() as i32 * 4;
                    assert!(
                        center - half >= 0 && center + half <= 160,
                        "clipped sign {text}"
                    );
                }
            }
        }
    }
    assert_eq!((hall.mario, hall.camera, hall.walk), (DOOR, 0, 0));
}

#[test]
fn signs_use_game_letters_and_file_select_digits_with_the_same_mask_in_dx() {
    use sml2_core::front_end::{FrontEndAssets, DEMO_TAPES_LEN, TILEMAP_LEN, VRAM_LEN};
    let title = vec![0; VRAM_LEN];
    let mut alphabet = vec![0xff; 0x300];
    let mut file_select = vec![0; VRAM_LEN];
    // The letter stencil is the high plane; the all-one low plane is background.
    for row in 0..8 {
        alphabet[0x11 * 16 + row * 2 + 1] = 0;
    }
    alphabet[0x11 * 16 + 1] = 0x80; // A: one pixel at the top-left
    file_select[0x87 * 16 + 1] = 0x01; // 7: one pixel at the top-right
    let assets = FrontEndAssets::new(
        title,
        vec![0; TILEMAP_LEN],
        file_select,
        vec![0; TILEMAP_LEN],
        vec![0; DEMO_TAPES_LEN],
    )
    .unwrap();
    let mut native = vec![0u8; 32 * 8];
    stamp_line(&mut native, 32, &assets, &alphabet, "A 7", 16, 0, 0, 3);
    assert_eq!(native.iter().filter(|&&v| v == 3).count(), 2);
    assert_eq!((native[4], native[27]), (3, 3));
    let mut rgb = vec![[9; 3]; 32 * 8];
    stamp_line(&mut rgb, 32, &assets, &alphabet, "A 7", 16, 0, 0, [1, 2, 3]);
    for (token, pixel) in native.iter().zip(rgb) {
        assert_eq!(pixel, if *token == 3 { [1, 2, 3] } else { [9; 3] });
    }
}

#[test]
fn the_loaded_game_alphabet_has_the_rom_letter_a_stencil() {
    let assets = sml2_runtime::session::SessionAssets::load().unwrap();
    let glyph = glyph(&assets.front_end, alphabet(&assets), b'A').unwrap();
    let rows = [0, 0x38, 0x7c, 0xc6, 0xc6, 0xfe, 0xc6, 0xc6];
    let expected: [u8; 16] = std::array::from_fn(|i| rows[i / 2]);
    assert_eq!(glyph, expected);
}
