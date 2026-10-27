use super::*;
use crate::ending_scene::EndingAssets;

fn dummy_ending_assets() -> EndingAssets {
    EndingAssets::new(vec![0xAA; 0x1800], vec![0x00; 0x400]).unwrap()
}

#[test]
fn test_substates_0_to_4_advancement() {
    let mut controller = EndingSequenceController::new();
    let mut scene = EndingScene::default();
    scene.load(dummy_ending_assets(), &mut 4, &mut 0x100);

    // Initial state: substate 0
    assert_eq!(controller.substate, 0);

    // Substates 0..3 advance exactly once per frame
    for expected in 1..=4 {
        let res = controller.step(&mut EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res.substate, expected);
        assert!(!res.terminal);
        assert!(!res.scanline_split_enabled);
    }

    // Substate 4 initializes scene fields and advances to 5
    let res = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res.substate, 5);
    assert_eq!(controller.fade_timer, 0x0100);
    assert_eq!(scene.credits_sprite_y, 44);
}

#[test]
fn test_substates_5_and_6_walk_and_credits_y() {
    let mut controller = EndingSequenceController::new();
    let mut scene = EndingScene::default();
    scene.load(dummy_ending_assets(), &mut 4, &mut 0x100);
    controller.substate = 5;
    controller.fade_timer = 2;

    // Substate 5: steps with fade decrement
    let res1 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: true,
    });
    assert_eq!(res1.substate, 5);
    assert_eq!(controller.fade_timer, 1);
    assert!(res1.sound_updated);

    // Fade reaches 0 -> advances to 6 with fade_timer = 128
    let res2 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: true,
    });
    assert_eq!(res2.substate, 6);
    assert_eq!(controller.fade_timer, 0x0080);

    // Substate 6: credits_sprite_y advances when (fade_timer & 3) == 0
    let start_y = scene.credits_sprite_y;
    controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: true, // 128 -> 127
    });
    // 127 & 3 != 0, so no movement
    assert_eq!(scene.credits_sprite_y, start_y);

    // Directly set fade_timer = 1, then step to zero -> advances to 7 with fade=32, raster_y=13
    controller.fade_timer = 1;
    let res_to_7 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: true,
    });
    assert_eq!(res_to_7.substate, 7);
    assert_eq!(controller.fade_timer, 0x0020);
    assert_eq!(controller.raster_y, 13);
}

#[test]
fn test_substates_7_to_10_raster_wave_and_transfer_2() {
    let mut controller = EndingSequenceController::new();
    let mut scene = EndingScene::default();
    controller.substate = 7;
    controller.raster_y = 137;
    controller.frame = 0; // Even frame: raster_y will not advance on even frame

    controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    // Frame incremented to 1 (odd) -> raster_y increments 137 -> 138
    assert_eq!(controller.raster_y, 138);
    assert_eq!(controller.substate, 7);

    // Frame incremented to 2 (even) -> raster_y remains 138
    controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(controller.raster_y, 138);

    // Frame incremented to 3 (odd) -> raster_y becomes 139 -> advances to 8 and schedules transfer 2
    let res8 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res8.substate, 8);
    assert!(controller.vram_transfer.is_some());
    let t2 = controller.vram_transfer.as_ref().unwrap();
    assert_eq!(t2.source_bank, 0x1A);
    assert_eq!(t2.source_addr, 0x5096);
    assert_eq!(t2.dest_addr, 0x9800);
    assert_eq!(t2.remaining_bytes, 0x0400);

    // Substate 8: when transfer is complete, advances to 9
    controller.vram_transfer.as_mut().unwrap().active = false;
    let res9 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res9.substate, 9);

    assert_eq!(controller.fade_timer, 0x0200);
    let background = controller.vram_transfer.as_ref().unwrap();
    assert_eq!(
        (
            background.source_bank,
            background.source_addr,
            background.dest_addr,
            background.remaining_bytes
        ),
        (0x10, 0x6000, 0x8800, 0x1000)
    );
    // Fade zero cannot bypass an active background transfer.
    controller.fade_timer = 0;
    let blocked = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(blocked.substate, 9);
    controller.vram_transfer.as_mut().unwrap().active = false;
    controller.fade_timer = 400;
    let res10 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res10.substate, 10);

    // Substate 10: retracts raster_y on odd frames until 13
    controller.raster_y = 14;
    controller.frame = 0; // next frame will be 1 (odd) -> raster_y becomes 13
    let res11 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res11.substate, 11);
    assert_eq!(controller.raster_y, 13);
    assert_eq!(controller.fade_timer, 0x00C0);
}

#[test]
fn test_substates_11_and_12_logo_and_burst() {
    let mut controller = EndingSequenceController::new();
    let mut scene = EndingScene::default();
    scene.load(dummy_ending_assets(), &mut 4, &mut 0x100);

    controller.substate = 11;
    controller.fade_timer = 0;

    // Substate 11 on fade zero sets ending_tile_flag = 0xFF, fade = 0x0180, advances to 12
    let res12 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res12.substate, 12);
    assert_eq!(controller.fade_timer, 0x0180);
    assert_eq!(controller.ending_tile_flag, 0xFF);

    // Next step: VBlank applies logo tiles (offsets 328 and 360) and clears flag
    controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(controller.ending_tile_flag, 0);
    let tm = scene.assets().unwrap().tilemap();
    assert_eq!(&tm[328..331], &ENDING_LOGO_ROW1_TILES);
    assert_eq!(&tm[360..363], &ENDING_LOGO_ROW2_TILES);

    // Substate 12 on odd frame requests SFX1 = 24
    controller.frame = 0; // step will make frame 1 (odd)
    controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(controller.sfx_request_1, 24);

    // Substate 12 when fade reaches 0: sets fade = 64, sfx0 = 7, credits_y = 44, advances to 13
    controller.fade_timer = 0;
    let res13 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res13.substate, 13);
    assert_eq!(controller.fade_timer, 0x0040);
    assert_eq!(controller.sfx_request_0, 7);
    assert_eq!(scene.credits_sprite_y, 44);
}

#[test]
fn test_substates_13_to_15_secondary_and_transfer_1() {
    let mut controller = EndingSequenceController::new();
    let mut scene = EndingScene::default();

    controller.substate = 13;
    controller.fade_timer = 0;

    // Substate 13 on fade zero sets fade = 0x0200, advances to 14
    let res14 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res14.substate, 14);
    assert_eq!(controller.fade_timer, 0x0200);

    // Substate 14: schedules transfer 1 ($4656 -> $8000, len 0x0800), advances to 15
    let res15 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res15.substate, 15);
    let t1 = controller.vram_transfer.as_ref().unwrap();
    assert_eq!(t1.source_bank, 0x1A);
    assert_eq!(t1.source_addr, 0x4656);
    assert_eq!(t1.dest_addr, 0x8000);
    assert_eq!(t1.remaining_bytes, 0x0800);

    // Substate 15: waits for transfer idle, then sets SFX0 = 6, fade = 64, advances to 16
    controller.vram_transfer.as_mut().unwrap().active = false;
    let res16 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res16.substate, 16);
    assert_eq!(controller.sfx_request_0, 6);
    assert_eq!(controller.fade_timer, 0x0040);
}

#[test]
fn test_substates_16_to_19_scroll_text_and_split() {
    let mut controller = EndingSequenceController::new();
    let mut scene = EndingScene::default();
    scene.load(dummy_ending_assets(), &mut 4, &mut 0x100);

    controller.substate = 16;
    controller.fade_timer = 0;

    // Substate 16 on fade zero sets fade = 0x0180, advances to 17
    let res17 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res17.substate, 17);
    assert_eq!(controller.fade_timer, 0x0180);

    // Substate 17: updates scroll_y:
    // formula: t = (fade_timer >> 4) as u8; scroll_y = (!t).wrapping_add(25)
    // When scroll_y == 24, advances to 18 and seeds credits text
    // Let's set fade_timer such that scroll_y == 24:
    // (!t) + 25 == 24 -> (!t) == 255 -> t == 0
    controller.fade_timer = 0; // t = 0 -> !t = 255 -> 255 + 25 = 24!
    let res18 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res18.substate, 18);
    assert_eq!(controller.scroll_y, 24);
    assert_eq!(scene.scroll_y, 24);
    assert_eq!(scene.credits_sprite_y, 44 - 24);
    assert!(res18.scanline_split_enabled);
    assert!(controller.credits_text.is_some());
    assert_eq!(controller.credits_text.as_ref().unwrap().phase, 4);

    // Substate 18: when text is done, advances to terminal substate 19
    controller.credits_text.as_mut().unwrap().done = true;
    let res19 = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res19.substate, 19);
    assert!(res19.terminal);
    assert!(res19.scanline_split_enabled);

    // Substate 19 is terminal: subsequent steps stay at 19
    let res19_next = controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res19_next.substate, 19);
    assert!(res19_next.terminal);
    assert!(res19_next.sound_updated);
}

#[test]
fn background_transfer_waits_for_all_64_byte_bursts_and_writes_vram() {
    let assets = dummy_ending_assets()
        .with_background_transfer((0..0x1000).map(|i| (i / 64) as u8).collect())
        .unwrap();
    let mut scene = EndingScene::default();
    scene.load(assets, &mut 4, &mut 0);
    let mut controller = EndingSequenceController::new();
    controller.substate = 8;
    controller.step(&mut EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(controller.substate, 9);
    for burst in 0..64usize {
        controller.step(&mut EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        let vram = scene.assets().unwrap().tiles();
        assert_eq!(
            &vram[0x800 + burst * 64..0x840 + burst * 64],
            &[burst as u8; 64]
        );
        assert_eq!(&vram[..0x800], &[0xAA; 0x800], "font/OBJ VRAM is retained");
        assert_eq!(controller.substate, if burst == 63 { 10 } else { 9 });
        assert_eq!(
            controller.fade_timer, 0x0200,
            "transfer completion owns the gate"
        );
    }
}
