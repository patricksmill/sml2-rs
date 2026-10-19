use super::*;

#[test]
fn secret_route_entry_consumes_map_progress_and_reloads_stored_start() {
    use sml2_core::level::LevelStart;
    use sml2_core::map::{MapNodeRecords, MapNodes, MapWalk};
    use sml2_core::map_sim::{MapInput, MapSim};
    let (mut manifest, _, _) = level_files("level12.manifest.json", "level12.bgblock.bin");
    manifest.secret_entry = Some(LevelStart::from_bytes([
        0x94, 0x01, 0x31, 0x08, 0xBA, 0x01, 0x52, 0x08,
    ]));
    let mut table = vec![0; MapNodes::BLOB_LEN];
    table[2] = 11;
    table[MapNodes::NODES + 2] = 1;
    let nodes = MapNodes::from_bytes(&table).unwrap();
    for progress in [0, 0x80, 8, 0x88] {
        let mut map = MapSim::new();
        map.screen_id = 1;
        map.node = 2;
        map.level_progress[1] = progress;
        let transition = map.tick(
            MapInput {
                keys_pressed: 1,
                ..Default::default()
            },
            &nodes,
            &MapWalk::default(),
            &MapNodeRecords::default(),
        );
        assert_eq!(transition.level, Some(manifest.level_id));
        let entry = manifest
            .header
            .entry(transition.progress_flags, manifest.secret_entry);
        let mut sim = PlayerSim::new();
        let mut cam = sml2_core::camera::CameraSim::new(0, 0);
        sim.secret_route = 255; // old pickup/previous level must not leak.
        apply_level_entry(&mut sim, &mut cam, entry);
        let expected = if progress & 8 != 0 {
            [0x94, 0x01, 0x31, 0x08, 0xBA, 0x01, 0x52, 0x08]
        } else {
            manifest.header.entry(0, None).start.to_bytes()
        };
        let observed = |sim: &PlayerSim, cam: &sml2_core::camera::CameraSim| {
            LevelStart {
                mario_y: sim.y,
                mario_x: sim.x,
                camera_y: cam.y,
                camera_x: cam.x,
            }
            .to_bytes()
        };
        assert_eq!(observed(&sim, &cam), expected, "$A800-$A807 consumers");
        assert_eq!(sim.secret_route, if progress & 8 != 0 { 255 } else { 0 });
        assert_eq!(sim.level_progress_flags, progress);
        let mut obj = sml2_core::objects::ObjectSim::new(Vec::new(), cam.x);
        sample_level_player_inputs(&mut obj, &sim);
        assert_eq!(obj.secret_route, progress & 8 != 0);
        // Item $07 changes only the route latch. Death reload consumes
        // the stored start, rather than choosing from that mutable latch.
        sim.secret_route = 255;
        sim.respawn(entry.start.mario_x, entry.start.mario_y);
        apply_level_entry(&mut sim, &mut cam, entry);
        assert_eq!(observed(&sim, &cam), expected);
        assert_eq!(sim.secret_route, entry.secret_route);
    }
}

#[test]
fn ending_entry_frame_head_keeps_trigger_player_and_objects_then_dispatches() {
    let (_, layout, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
    let tables = pan_sprites(None).tables;
    let mut sim = PlayerSim::new();
    sim.x = 162;
    sim.y = 224;
    sim.fade_timer = 2;
    sim.boss_clear = true;
    sim.level_timer = sml2_core::level_timer::LevelTimer::new(0x03);
    let mut cam = sml2_core::camera::CameraSim::new(80, 224);
    cam.boss_fight_phase = 3;
    let mut obj = sml2_core::objects::ObjectSim::new(Vec::new(), cam.x);
    let mut ending = EndingEntry::default();
    let action = ending_frame_head(
        &mut ending,
        &mut sim,
        &mut cam,
        EndingInput {
            held: PlayerKeys {
                left: true,
                start: true,
                select: true,
                ..PlayerKeys::default()
            },
            pressed: PlayerKeys {
                a: true,
                b: true,
                start: true,
                select: true,
                ..PlayerKeys::default()
            },
        },
    );
    assert_eq!(ending.state() as u8, 35);
    assert_eq!(sim.fade_timer, 2, "player owns trigger-frame fade tick");
    let EndingFrame::Gameplay(input) = action else {
        panic!("trigger frame still runs gameplay");
    };
    assert!(input.held.right && input.held.a);
    assert!(!input.held.left && !input.held.start && !input.held.select);
    assert!(!input.pressed.a && !input.pressed.b && !input.pressed.start && !input.pressed.select);
    sim.step(&input.held, input.pressed.a, input.pressed.b, &layout);
    assert_eq!(sim.fade_timer, 1);
    assert!(
        sim.hvel != 0,
        "scripted Right reaches the real player update"
    );
    cam.step(&mut sim);
    let before_object_phase = obj.phase;
    let contact = build_contact(&sim, &input.held, cam.scroll_pan);
    obj.sample_player_inputs(&sim);
    obj.step(cam.x, cam.y, &layout, &tables, &contact, cam.boss_mode());
    assert_ne!(
        obj.phase, before_object_phase,
        "trigger frame runs the actual object engine"
    );
    sim.apply_object_result(&mut obj, input.held.a);
    cam.apply_object_result(&mut obj);
    assert!(
        !sim.exit_phase && !sim.exit_completed,
        "ending entry never rewards an ordinary level clear"
    );
    let timer_before_fade = sim.level_timer;
    assert!(matches!(
        ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default()),
        EndingFrame::Fade(_)
    ));
    assert_eq!(ending.state() as u8, 18);
    assert_eq!(
        sim.fade_timer, 255,
        "one tick reaches zero, then state35 resets fade"
    );
    assert_eq!(sim.level_timer, timer_before_fade);
    assert_eq!(cam.boss_fight_phase, 3);
    assert!(matches!(
        ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default()),
        EndingFrame::SequenceInitialized
    ));
    assert_eq!(
        (
            ending.state() as u8,
            ending.ending_state(),
            cam.boss_fight_phase,
            sim.fade_timer
        ),
        (19, 0, 0, 254)
    );
    assert!(matches!(
        ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default()),
        EndingFrame::SequencePending
    ));
    assert_eq!(sim.fade_timer, 253);
    assert!(!sim.exit_phase && !sim.exit_completed);
}

#[test]
fn ending_sequence_runs_from_entry_through_all_substates_to_terminal_completion() {
    let mut sim = PlayerSim::new();
    sim.x = 162;
    sim.y = 224;
    sim.fade_timer = 2;
    sim.boss_clear = true;
    let mut cam = sml2_core::camera::CameraSim::new(80, 224);
    cam.boss_fight_phase = 3;
    let mut ending = EndingEntry::default();

    // 1. Scripted trigger frame
    let f1 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
    assert!(matches!(f1, EndingFrame::Gameplay(_)));
    assert_eq!(ending.state() as u8, 35);

    // 2. Fade frame -> sets state 18 when fade reaches zero
    sim.fade_timer = 1;
    let f2 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
    assert!(matches!(f2, EndingFrame::Fade(_)));
    assert_eq!(ending.state() as u8, 18);

    // 3. Initialize frame -> sets state 19
    let f3 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
    assert!(matches!(f3, EndingFrame::SequenceInitialized));
    assert_eq!(ending.state() as u8, 19);

    // 4. State 19 sequence controller takes over
    let mut seq = sml2_core::ending_sequence::EndingSequenceController::new();
    let mut scene = sml2_core::ending_scene::EndingScene::default();
    let dummy_assets =
        sml2_core::ending_scene::EndingAssets::new(vec![0; 0x1800], vec![0; 0x400]).unwrap();
    let mut sound_driver = sml2_core::audio::SoundDriver::new();

    // Substates 0..3: blank frame waits
    for sub in 0..4 {
        let fp = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
        assert!(matches!(fp, EndingFrame::SequencePending));
        let res = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
            scene: &mut scene,
            rom: None,
            credits_text: None,
            object_tables: None,
            decrement_fade: false,
        });
        assert_eq!(res.substate, sub + 1);
    }

    // Substate 4: loads ending scene assets
    scene.load(dummy_assets, &mut 4, &mut 0x100);
    let res4 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res4.substate, 5);

    // Fast-forward substates 5..19 verifying state targets and audio dispatch
    // Jump substate 5 -> 6
    seq.fade_timer = 0;
    let res5 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res5.substate, 6);

    // Jump substate 6 -> 7
    seq.fade_timer = 0;
    let res6 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res6.substate, 7);

    // Jump substate 7 -> 8 (raster_y reached 139)
    seq.raster_y = 138;
    seq.frame = 0;
    let res7 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res7.substate, 8);
    assert!(seq.vram_transfer.is_some());

    // Substate 8 -> 9 (transfer finishes)
    seq.vram_transfer.as_mut().unwrap().active = false;
    let res8 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res8.substate, 9);

    // Substate 9 -> 10 (fade reaches 0)
    seq.fade_timer = 0;
    let res9 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res9.substate, 10);

    // Substate 10 -> 11 (raster_y retracts to 13)
    seq.raster_y = 14;
    seq.frame = 0;
    let res10 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res10.substate, 11);

    // Substate 11 -> 12 (ending tile flag set)
    seq.fade_timer = 0;
    let res11 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res11.substate, 12);
    assert_eq!(seq.ending_tile_flag, 0xFF);

    // Substate 12: burst cycle with SFX1=24 on odd frame
    seq.frame = 0;
    let res12 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res12.sfx_request_1, 24);
    sound_driver.sfx_request_1 = res12.sfx_request_1;
    sound_driver.update_sound();
    assert_eq!(
        sound_driver.sfx_request_1, 0,
        "sound driver consumed request"
    );

    // Substate 12 -> 13 (fade reaches 0 -> SFX0=7 burst sound)
    seq.fade_timer = 0;
    let res12_end = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res12_end.substate, 13);
    assert_eq!(res12_end.sfx_request_0, 7);

    // Substate 13 -> 14
    seq.fade_timer = 0;
    let res13 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res13.substate, 14);

    // Substate 14 -> 15 (queues transfer 1)
    let res14 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res14.substate, 15);

    // Substate 15 -> 16 (transfer finished -> SFX0=6 castle transformation)
    seq.vram_transfer.as_mut().unwrap().active = false;
    let res15 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res15.substate, 16);
    assert_eq!(res15.sfx_request_0, 6);

    // Substate 16 -> 17
    seq.fade_timer = 0;
    let res16 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res16.substate, 17);

    // Substate 17 -> 18 (scroll_y reached 24)
    seq.fade_timer = 0;
    let res17 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res17.substate, 18);
    assert_eq!(res17.scroll_y, 24);
    assert!(res17.scanline_split_enabled);

    // Substate 18 -> 19 (credits text finishes)
    seq.credits_text.as_mut().unwrap().done = true;
    let res18 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res18.substate, 19);
    assert!(res18.terminal);
    assert!(res18.scanline_split_enabled);

    // Substate 19 is terminal
    let res19 = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(res19.substate, 19);
    assert!(res19.terminal);
}

#[test]
fn level_timer_load_and_reload_use_header_bytes() {
    let (mut manifest, _, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
    manifest.header.time_limit = 0x12;
    manifest.header.music_track = 0x23;
    let mut sim = PlayerSim::new();
    sim.low_time_flag = 1;
    for _ in 0..2 {
        seed_level_timer(&mut sim, &manifest);
        assert_eq!(
            sim.level_timer,
            sml2_core::level_timer::LevelTimer::new(0x12)
        );
        assert_eq!(sim.level_song, 0x23);
        assert_eq!(sim.low_time_flag, 1);
        sim.level_timer.low = 0x19;
        sim.level_timer.high = 0;
        sim.level_timer.tick_counter = 1;
        sim.low_time_flag = 1;
        sim.level_song = 0xFF;
    }
}

#[test]
fn level_timer_frame_head_freezes_with_pause_and_fades() {
    let mut sim = PlayerSim::new();
    sim.level_timer = sml2_core::level_timer::LevelTimer::new(0x03);
    let initial = sim.level_timer;
    for flags in [
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        (sim.paused, sim.dead_fade, sim.exit_phase) = flags;
        update_level_timer_at_frame_head(&mut sim);
        assert_eq!(sim.level_timer, initial);
    }
    (sim.paused, sim.dead_fade, sim.exit_phase) = (false, false, false);
    update_level_timer_at_frame_head(&mut sim);
    assert_eq!(sim.level_timer.tick_counter, 0x2F);
    assert_eq!((sim.level_timer.high, sim.level_timer.low), (3, 0));
}

#[test]
fn level_timer_expiry_precedes_the_death_snapshot() {
    let mut sim = PlayerSim::new();
    sim.level_timer = sml2_core::level_timer::LevelTimer::new(0);
    sim.level_timer.tick_counter = 1;
    update_level_timer_at_frame_head(&mut sim);
    assert_eq!(sim.dead, 0x80);
    assert_eq!((sim.level_timer.high, sim.level_timer.low), (0, 0));
    assert_eq!(sim.sfx0, 0x0F);
}

/// Load one level's extracted manifest, layout and composed tile block.
fn level_files(manifest_name: &str, bg_name: &str) -> (LevelManifest, LevelLayout, Vec<u8>) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted/levels");
    let manifest: LevelManifest =
        serde_json::from_str(&fs::read_to_string(dir.join(manifest_name)).unwrap()).unwrap();
    let layout =
        LevelLayout::from_bytes(&fs::read(dir.join(&manifest.layout.file)).unwrap()).unwrap();
    let bg = fs::read(dir.join(bg_name)).unwrap();
    (manifest, layout, bg)
}

/// Top row of the level's solid block row in a rendered frame — the row carrying the
/// widest run of the palette's darkest shade below row 60 (the block row; the cloud
/// outlines never cover a whole row).
fn block_row_top(img: &[u8], vw: usize, darkest: u8) -> usize {
    (60..144)
        .map(|y| (y, (0..vw).filter(|&x| img[y * vw + x] == darkest).count()))
        .max_by_key(|&(_, n)| n)
        .expect("a block row in the frame")
        .0
}

/// The view must not be clamped into the level's own bounds. Level 5's start camera sits
/// at the bottom of a 768-px world (cam_y 704), so the view's top row (632) is past
/// `768 - 144`: the engine lets the view overhang and the tilemap window's rows come back
/// around under the status bar. Clamping the view to the level instead shifted the whole
/// background 8 px down against Mario's OAM, and he looked like he floated above the
/// block row. The console — PyBoy, entered through the map route in
/// `captures/probe_level_select.py` — draws that block row's top on row 118.
#[test]
fn a_camera_at_the_level_bottom_does_not_shift_the_background() {
    let (manifest, layout, bg) = level_files("level06.manifest.json", "level06.bgblock.bin");
    let darkest = video::bgp_shades(manifest.header.palettes[0])[3];
    let img = render_view(
        &manifest,
        &layout,
        &[],
        &bg,
        80,
        704,
        160,
        144,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block_row_top(&img, 160, darkest), 118);
}

/// The same frame contract mid-level, where nothing is clamped either way: level 1's
/// start camera (80, 448) draws its block row's top on row 120 on the console.
#[test]
fn a_camera_mid_level_lands_where_the_console_draws_it() {
    let (manifest, layout, bg) = level_files("level01.manifest.json", "level01.bgblock.bin");
    let darkest = video::bgp_shades(manifest.header.palettes[0])[3];
    let img = render_view(
        &manifest,
        &layout,
        &[],
        &bg,
        80,
        448,
        160,
        144,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(block_row_top(&img, 160, darkest), 120);
}

/// `$04DD`/`home.asm:661`: `sMoonPhysics` is the level's own
/// `LevelPropertiesSpace` byte (`$1F91`, indexed by `sCurLevel`), which both
/// frame loops now seed from the manifest. Only three levels are nonzero —
/// indices 18, 19 and 28, values `$08`, `$01`, `$08` — and level 1 is `$00`
/// in all three property bytes, which is exactly why the replay gate was
/// structurally blind to this: every capture ran on `PlayerSim::new()`'s own
/// default. traj19 (level 20) records the byte, so the gate checks the
/// derivation now; this test checks the three known levels' values.
#[test]
fn the_moon_physics_byte_comes_from_the_manifest_for_the_three_known_levels() {
    for (name, id, byte) in [
        ("level19.manifest.json", 18u8, 0x08u8),
        ("level20.manifest.json", 19, 0x01),
        ("level29.manifest.json", 28, 0x08),
    ] {
        let (manifest, _, _) = level_files(name, "level01.bgblock.bin");
        assert_eq!(manifest.level_id, id, "{name}");
        assert_eq!(
            manifest.properties.space_physics, byte,
            "{name}: the ROM's LevelPropertiesSpace entry"
        );
        let mut sim = PlayerSim::new();
        sim.seed_moon_physics(manifest.properties.space_physics);
        assert_eq!(sim.moon, byte, "{name}: seeded raw, not folded to a bool");
    }
    // The default side: level 1's byte is `$00`, so its play and replay are
    // unchanged by the seed — the reason 19 captures never saw the gap.
    let (lvl1, _, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
    assert_eq!(lvl1.properties.space_physics, 0x00);
    let mut sim = PlayerSim::new();
    sim.seed_moon_physics(lvl1.properties.space_physics);
    assert_eq!(sim.moon, 0x00);
}

fn pan_sprites(player: Option<(u16, u16, u8)>) -> SpriteCtx {
    SpriteCtx {
        tables: serde_json::from_str(
            &fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extracted/objects.json"),
            )
            .unwrap(),
        )
        .unwrap(),
        level_id: 0,
        insts: Vec::new(),
        objblock: vec![255; 0x1000],
        obp0: 0xE4,
        obp1: 0xE4,
        player,
        player_shield: None,
        player_behind_bg: false,
        bumpers: Vec::new(),
        fireballs: Vec::new(),
        boss_oam: Vec::new(),
        player_pan: 0,
        scroll_pan: 0,
    }
}

#[test]
fn pan_renderer_moves_bg_by_latched_scroll_phase() {
    let (manifest, layout, bg) = level_files("level01.manifest.json", "level01.bgblock.bin");
    let mut s = pan_sprites(None);
    let baseline = render_view(
        &manifest,
        &layout,
        &[],
        &bg,
        80,
        448,
        160,
        144,
        None,
        Some(&s),
        None,
    )
    .unwrap();
    s.scroll_pan = 3;
    let shifted = render_view(
        &manifest,
        &layout,
        &[],
        &bg,
        80,
        448,
        160,
        144,
        None,
        Some(&s),
        None,
    )
    .unwrap();
    assert_ne!(baseline, shifted);
    assert_eq!(&shifted[3 * 160..144 * 160], &baseline[..141 * 160]);
}

#[test]
fn pan_renderer_moves_only_mario_by_post_decrement_phase() {
    let (manifest, layout, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
    let bg = vec![0; 0x1000];
    let mut s = pan_sprites(Some((34, 468, 32)));
    let baseline = render_view(
        &manifest,
        &layout,
        &[],
        &bg,
        80,
        448,
        160,
        144,
        None,
        Some(&s),
        None,
    )
    .unwrap();
    s.player_pan = 2;
    let shifted = render_view(
        &manifest,
        &layout,
        &[],
        &bg,
        80,
        448,
        160,
        144,
        None,
        Some(&s),
        None,
    )
    .unwrap();
    assert_ne!(baseline, shifted, "the mapping must draw visible pixels");
    assert_eq!(&shifted[..142 * 160], &baseline[2 * 160..144 * 160]);
}

#[test]
fn wario_arena_entries_definitions_6_7_8_through_final_ending_integration() {
    let (_, layout, _) = level_files("level01.manifest.json", "level01.bgblock.bin");
    let tables = pan_sprites(None).tables;
    let mut sim = PlayerSim::new();
    let mut cam = sml2_core::camera::CameraSim::new(80, 72);
    let mut obj = sml2_core::objects::ObjectSim::new(Vec::new(), cam.x);
    obj.level_id = 0x18;

    // =================================================================
    // PHASE 0: Wario Form 1 Entry (Definition 6)
    // =================================================================
    cam.scene_load_pending = true;

    let def_index = boss_def_index(0x18, cam.boss_fight_phase);
    assert_eq!(def_index, 6);
    assert!(boss_def_portable(def_index));

    let (music, level_song) = cam.begin_scene_load(0x18).unwrap();
    assert_eq!((music, level_song), (33, 32));
    sim.music_request = music;
    sim.level_song = level_song;

    obj.spawn_boss(def_index);
    assert_eq!(obj.boss_def_index, 6);
    assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 20); // Wario Form 1
    assert_eq!(obj.slots[1].b[sml2_core::objects::B_DEF], 21); // Child 21
    assert!(cam.boss_mode());
    assert!(!cam.take_blocked_scene_load());
    assert!(obj.unported.is_empty());

    // Form 1 Defeat and Phase 1 Exit:
    // Set slot 0 to state 8 (exit walk) at threshold screen X
    obj.slots[0].b[sml2_core::objects::B_FSTATE] = 8;
    obj.slots[0].b[sml2_core::objects::B_SCRX] = 191;
    obj.fff0 = 1;
    obj.sample_player_inputs(&sim);
    obj.boss_engine(80, 72, &layout, &tables);

    // Object result processed by player and camera
    sim.apply_object_result(&mut obj, false);
    cam.apply_object_result(&mut obj);

    assert_eq!(
        obj.slots[0].b[sml2_core::objects::B_DEF],
        0,
        "Form 1 slot blanked"
    );
    assert_eq!(cam.boss_fight_phase, 1, "Phase advanced to 1");
    assert_eq!(cam.scene_state, 0, "Arena unparked for stream door");
    assert!(!cam.boss_mode());
    assert_eq!(obj.records.len(), 1);
    assert_eq!(obj.records[0].raw, [7, 0xC8, 0, 0, 0xB0, 0x1D]);
    assert!(!cam.take_blocked_scene_load());
    assert!(obj.unported.is_empty());

    // =================================================================
    // PHASE 1: Wario Form 2 Entry (Definition 7)
    // =================================================================
    // Mario walks through the streamed door: transition finishes
    cam.scene_load_pending = true;

    let def_index = boss_def_index(0x18, cam.boss_fight_phase);
    assert_eq!(def_index, 7);
    assert!(
        boss_def_portable(def_index),
        "Definition 7 must be portable"
    );

    let (music, level_song) = cam.begin_scene_load(0x18).unwrap();
    assert_eq!((music, level_song), (34, 32));
    sim.music_request = music;
    sim.level_song = level_song;

    obj.spawn_boss(def_index);
    assert_eq!(obj.boss_def_index, 7);
    assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 23); // Wario Form 2
    assert_eq!(obj.slots[1].b[sml2_core::objects::B_DEF], 24); // Child 24
    assert!(cam.boss_mode());
    assert!(!cam.take_blocked_scene_load());
    assert!(obj.unported.is_empty());

    // Form 2 Defeat and Phase 2 Exit:
    // Set slot 0 to state 8 (exit walk) at threshold screen X
    obj.slots[0].b[sml2_core::objects::B_FSTATE] = 8;
    obj.slots[0].b[sml2_core::objects::B_SCRX] = 190;
    obj.sample_player_inputs(&sim);
    obj.boss_engine(80, 72, &layout, &tables);

    sim.apply_object_result(&mut obj, false);
    cam.apply_object_result(&mut obj);

    assert_eq!(
        obj.slots[0].b[sml2_core::objects::B_DEF],
        0,
        "Form 2 slot blanked"
    );
    assert_eq!(cam.boss_fight_phase, 2, "Phase advanced to 2");
    assert_eq!(cam.scene_state, 0, "Arena unparked for stream door");
    assert!(!cam.boss_mode());
    assert_eq!(obj.records.len(), 1);
    assert_eq!(obj.records[0].raw, [8, 0xC8, 0, 0, 0xB0, 27]);
    assert!(!cam.take_blocked_scene_load());
    assert!(obj.unported.is_empty());

    // =================================================================
    // PHASE 2: Wario Form 3 Entry (Definition 8)
    // =================================================================
    // Mario walks through the streamed door: transition finishes
    cam.scene_load_pending = true;

    let def_index = boss_def_index(0x18, cam.boss_fight_phase);
    assert_eq!(def_index, 8);
    assert!(
        boss_def_portable(def_index),
        "Definition 8 must be portable"
    );

    let (music, level_song) = cam.begin_scene_load(0x18).unwrap();
    assert_eq!((music, level_song), (35, 32));
    sim.music_request = music;
    sim.level_song = level_song;

    obj.spawn_boss(def_index);
    assert_eq!(obj.boss_def_index, 8);
    assert_eq!(obj.slots[0].b[sml2_core::objects::B_DEF], 26); // Wario Form 3
    assert!(cam.boss_mode());
    assert!(!cam.take_blocked_scene_load());
    assert!(obj.unported.is_empty());

    // Form 3 Defeat and Phase 3 Exit:
    // Set slot 0 to state 10 (defeat exit) with expired timer at threshold screen X
    obj.slots[0].b[sml2_core::objects::B_FSTATE] = 10;
    obj.slots[0].b[sml2_core::objects::B_T28] = 0;
    obj.slots[0].b[sml2_core::objects::B_SCRX] = 191;
    obj.fff0 = 1;
    obj.sample_player_inputs(&sim);
    obj.boss_engine(80, 72, &layout, &tables);

    sim.apply_object_result(&mut obj, false);
    cam.apply_object_result(&mut obj);

    assert_eq!(
        obj.slots[0].b[sml2_core::objects::B_DEF],
        0,
        "Form 3 slot blanked"
    );
    assert_eq!(
        cam.boss_fight_phase, 3,
        "Phase advanced to 3 (Ending Entry)"
    );
    assert_eq!(cam.scene_state, 0, "Arena unparked");
    assert!(!cam.boss_mode());
    assert!(!cam.take_blocked_scene_load());
    assert!(obj.unported.is_empty());

    // =================================================================
    // PHASE 3: Ending Entry through Sequence Completion
    // =================================================================
    let mut ending = EndingEntry::default();
    sim.x = 162;
    sim.y = 224;
    sim.fade_timer = 2;
    sim.boss_clear = true;

    // Trigger frame
    let fp0 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
    assert!(matches!(fp0, EndingFrame::Gameplay(_)));
    assert_eq!(ending.state(), EndingEntryState::Fade);

    // Fade frames
    let fp1 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
    assert!(matches!(fp1, EndingFrame::Fade(_)));
    let fp2 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
    assert!(matches!(fp2, EndingFrame::Fade(_)));
    assert_eq!(ending.state(), EndingEntryState::Initialize);

    // Initialize frame
    let fp3 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
    assert!(matches!(fp3, EndingFrame::SequenceInitialized));
    assert_eq!(ending.state(), EndingEntryState::Sequence);

    // Sequence begins
    let fp4 = ending_frame_head(&mut ending, &mut sim, &mut cam, EndingInput::default());
    assert!(matches!(fp4, EndingFrame::SequencePending));

    // Step EndingSequenceController to completion
    let mut seq = sml2_core::ending_sequence::EndingSequenceController::new();
    let mut scene = sml2_core::ending_scene::EndingScene::default();
    let dummy_assets =
        sml2_core::ending_scene::EndingAssets::new(vec![0; 0x1800], vec![0; 0x400]).unwrap();
    scene.load(dummy_assets, &mut 4, &mut 0x100);

    // Fast-forward substates to terminal completion
    seq.substate = 19;
    let final_step = seq.step(&mut sml2_core::ending_sequence::EndingStepContext {
        scene: &mut scene,
        rom: None,
        credits_text: None,
        object_tables: None,
        decrement_fade: false,
    });
    assert_eq!(final_step.substate, 19);
    assert!(final_step.terminal);

    // Verify clean diagnostics throughout
    assert!(!cam.take_blocked_scene_load());
    assert!(obj.unported.is_empty());
}
