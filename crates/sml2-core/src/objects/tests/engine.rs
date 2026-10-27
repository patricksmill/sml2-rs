//! Object engine regression tests.
use super::*;
use crate::level::{SpawnRecord, LAYOUT_SIZE};
use crate::object::AnimProfile;

fn prof(handler: u32) -> AnimProfile {
    AnimProfile {
        handler,
        type_sets: Vec::new(),
        states: Vec::new(),
    }
}

/// Minimal mode-A tables with level 0's item wiring: anim 16 is the pickup
/// handler, 18/20 are the item poses; each anim gets its own sprite id so
/// the touch arm's derive order is observable. Anims 17 (the `$11` hopper)
/// and 22 (the `$86` pose) carry their real handlers/sprites for the
/// below-hit arm.
fn tables() -> ObjectTables {
    let mut profiles = vec![prof(0); 96];
    profiles[15] = prof(0x586E1);
    profiles[16] = prof(0x586F5);
    profiles[17] = prof(0x58712);
    profiles[18] = prof(0x58758);
    profiles[20] = prof(0x58755);
    profiles[22] = prof(0x587BD); // the `$86` pose (anim 22)
    let mut sprites = vec![0u8; 96];
    sprites[16] = 30; // the anim-16 pickup sprite
    sprites[17] = 25; // the `$11` hopper's sprite
    sprites[18] = 26; // the item pose's sprite
    sprites[22] = 31; // the `$86` pose's sprite
    ObjectTables {
        schema: String::new(),
        notes: Vec::new(),
        anim_to_sprite_a: sprites,
        anim_to_sprite_b: Vec::new(),
        hitbox_records_a: vec![[0; 4]],
        hitbox_records_b: Vec::new(),
        hitbox_index_a: vec![0; 96],
        hitbox_index_b: Vec::new(),
        boss_anim_handlers: Vec::new(),
        boss_picker_profile: Vec::new(),
        boss_picker_profile_15: Vec::new(),
        boss_sweep_profile: Vec::new(),
        draw_records: Vec::new(),
        draw_index: Vec::new(),
        sprite_mappings_a: Vec::new(),
        sprite_mappings_b: Vec::new(),
        anim_profiles_a: profiles,
        anim_profiles_b: Vec::new(),
        player_mappings: Vec::new(),
        // 118 empty lists: the boss render pass must find a list for any
        // sprite id a def installs, so the synthetic table keeps the shape
        // of the real one (see `boss_oam_tables` for a populated one).
        boss_oam_lists: vec![vec![0x80]; 118],
    }
}

/// The synthetic tables with sprite 30's boss OAM list replaced by the
/// record under test.
fn boss_oam_tables(list: Vec<u8>) -> ObjectTables {
    let mut t = tables();
    t.boss_oam_lists[30] = list;
    t
}

/// A staged boss slot ready for the render pass: sprite 30, the def-2
/// controller's own `$FFD9`/`$FFDA` pair ($40/$58).
fn boss_staged() -> Staging {
    let mut st = Staging::default();
    st.b[B_SPRITE] = 30;
    st.b[B_SCRY] = 0x40;
    st.b[B_SCRX] = 0x58;
    st
}

fn blank_layout() -> LevelLayout {
    LevelLayout::from_bytes(&[0x60; LAYOUT_SIZE]).unwrap()
}

#[test]
fn handler_0x0ab30_boss_dispatch_moves_child_then_delivers_delay_and_removes() {
    let mut t = tables();
    t.boss_anim_handlers = vec![0x9322; 23];
    t.boss_anim_handlers[22] = 0xAB30;
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.boss_def_count = 1;
    // A parked primary retains the ROM's screen coordinates for the child.
    obj.slots[0].b[B_DEF] = 1;
    obj.slots[0].b[B_SCRX] = 0x91;
    obj.slots[0].b[B_SCRY] = 0x8A;
    obj.slots[1].b[B_DEF] = 22;
    let layout = blank_layout();
    obj.boss_engine(0, 0, &layout, &t); // odd frame, initialization still runs
    assert_eq!(obj.slots[1].b[B_SPRITE], 0x70);
    assert_eq!(obj.slots[1].b[B_SCRX], 0x91);
    assert_eq!(obj.slots[1].b[B_SCRY], 0x10);
    assert_eq!(obj.slots[1].b[B_FSTATE], 1);
    obj.slots[1].b[B_SCRY] = 0x5E;
    obj.boss_engine(0, 0, &layout, &t); // even: still below the $60 band
    assert_eq!(obj.slots[1].b[B_SCRY], 0x5F);
    assert_eq!(obj.boss_delay, 0);
    obj.mario.dead = 0xFF;
    obj.boss_engine(0, 0, &layout, &t); // odd, dead
    obj.boss_engine(0, 0, &layout, &t); // even, dead: no completion write
    assert_eq!(obj.slots[1].b[B_SCRY], 0x5F);
    assert_eq!(obj.boss_delay, 0);
    obj.mario.dead = 0;
    obj.boss_engine(0, 0, &layout, &t); // odd: held
    assert_eq!(obj.slots[1].b[B_SCRY], 0x5F);
    obj.boss_engine(0, 0, &layout, &t); // even: $60+$20 matches primary $80
    assert_eq!(obj.boss_delay, 1);
    assert_eq!(obj.boss_def_count, 0);
    assert_eq!(obj.slots[1].b, [0; 32]);
    assert!(obj.unported.is_empty(), "{:X?}", obj.unported);
}

#[test]
fn handler_0x0ab6e_dispatch_and_level_reload_clear_delay() {
    let mut t = tables();
    t.boss_anim_handlers = vec![0x9322; 26];
    t.boss_anim_handlers[25] = 0xAB6E;
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.boss_def_count = 1;
    obj.slots[0].b[B_DEF] = 1;
    obj.slots[0].b[B_SCRX] = 0x92;
    obj.slots[0].b[B_SCRY] = 0x8A;
    obj.slots[1].b[B_DEF] = 25;
    let layout = blank_layout();
    obj.boss_engine(0, 0, &layout, &t);
    assert_eq!(obj.slots[1].b[B_SPRITE], 0x71);
    assert_eq!(obj.slots[1].b[B_SCRX], 0x92);
    assert_eq!(obj.slots[1].b[B_SCRY], 0x10);
    obj.slots[1].b[B_SCRY] = 0x5F;
    obj.boss_engine(0, 0, &layout, &t);
    assert_eq!(obj.slots[1].b, [0; 32]);
    assert_eq!(obj.boss_delay, 1);
    assert_eq!(obj.boss_def_count, 0);
    assert!(obj.unported.is_empty());
    let frame = obj.fff0;
    obj.reset_level_start();
    assert_eq!(obj.boss_delay, 0);
    assert_eq!(obj.fff0, frame);
}

/// A staged copy carrying the `$2DC6` touch flag (bit 4) and the given
/// type/anim.
fn touched(anim: u8, ty: u8) -> Staging {
    let mut st = Staging::default();
    st.b[B_DEF] = anim;
    st.b[B_TYPE] = ty;
    st.b[B_FLAGS] = 0x10;
    st
}

/// `$8695` order: for `$80`-`$84`/`$89` the flip clears and `$863C` runs
/// while the anim is still the old one — the sprite stays the **old**
/// anim's (the anim-16 handler never re-derives it) — and only then does
/// `$FFD5` become 16. `$8240` wipes `+14..+30`, which is what consumes the
/// touch flag and resets the rise counter; `sObjectDone = 1` stops the slot
/// for this frame.
#[test]
fn touch_switches_to_anim16_keeping_the_old_sprite() {
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = touched(18, 0x80);
    st.b[B_T24] = 0x0A;
    st.b[B_FLASH] = 0x20;
    st.b[B_FSTATE] = 2;
    obj.interaction_dispatch(&mut st, &t);
    assert_eq!(st.b[B_DEF], 16, "the anim switched");
    assert_eq!(st.b[B_SPRITE], 26, "the sprite is the old anim's");
    assert_eq!(st.b[B_FLIP], 0);
    assert_eq!(st.b[B_T24], 0, "$8240 cleared +14..+30");
    assert_eq!(st.b[B_FLASH], 0);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(st.b[B_FLAGS], 0, "the touch flag was consumed");
    assert_eq!(obj.done, 1, "$863C sets sObjectDone");
    assert!(obj.unported.is_empty());
    // $89 takes the same fall-through as $80-$84.
    let mut st = touched(18, 0x89);
    obj.interaction_dispatch(&mut st, &t);
    assert_eq!((st.b[B_DEF], st.b[B_SPRITE]), (16, 26));
}

/// `$86DA` order: type `$85` writes the anim *before* `$863C`, so its
/// sprite is the anim-16 one.
#[test]
fn touch_type85_derives_the_sprite_from_anim16() {
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = touched(15, 0x85);
    obj.interaction_dispatch(&mut st, &t);
    assert_eq!(st.b[B_DEF], 16);
    assert_eq!(st.b[B_SPRITE], 30, "map[16]: the anim was written first");
    assert_eq!(st.b[B_FLIP], 0);
    assert_eq!(obj.done, 1);
}

/// The `$868A` switch's inert arm: `$86`-`$88` and `$8A`+ return without
/// touching the object (their pickup/pose behavior lives in their own
/// handlers). The bit-7-clear type-`$06` carry path is still unported.
#[test]
fn touched_86_to_88_and_8a_plus_are_inert() {
    let t = tables();
    for ty in [0x86u8, 0x87, 0x88, 0x8A, 0x90, 0xC0, 0xFF] {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut st = touched(22, ty);
        st.b[B_T24] = 0x0A;
        obj.interaction_dispatch(&mut st, &t);
        assert_eq!(st.b[B_DEF], 22, "type ${ty:02X} keeps its anim");
        assert_eq!(st.b[B_T24], 0x0A, "and its scratch");
        assert_eq!(obj.done, 0);
        assert!(obj.unported.is_empty());
    }
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = touched(1, 0x06);
    obj.interaction_dispatch(&mut st, &t);
    assert!(obj.unported.is_empty(), "type 06 is ported at $8611");
    assert_eq!(st.b[B_DEF], 2, "kicked without B held: anim += 1");
}

/// `$586F5` per call: `+15 = 1`, the `$5B68B` flash on even `$FFF0`, Y -= 2
/// and `+24` += 2; the first 15 calls never remove.
#[test]
fn pickup16_rises_steps_the_counter_and_flashes_on_even_frames() {
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.fff0 = 0; // even: the flash toggles
    let mut st = Staging::default();
    st.b[B_STATE] = 2;
    set_y(&mut st.b, 100);
    obj.handler_pickup16(&mut st, &t);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(y_of(&st.b), 98);
    assert_eq!(st.b[B_T24], 2);
    assert_eq!(st.b[B_FLASH], 0x10);
    assert_eq!(obj.done, 0);
    obj.fff0 = 1; // odd: no flash
    obj.handler_pickup16(&mut st, &t);
    assert_eq!(st.b[B_FLASH], 0x10);
    assert_eq!(y_of(&st.b), 96);
    assert_eq!(st.b[B_T24], 4);
    assert_eq!(obj.done, 0);
}

/// The 16th call (`+24` reaching `$20`) removes the slot in remove-mode 2:
/// the slot zeroes and the active count drops, but — unlike mode 0/1 — the
/// record marker survives and no coin/money-bag credit runs.
#[test]
fn pickup16_sixteenth_call_despawns_without_credit_or_marker_clear() {
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.active = 1;
    obj.spawned = vec![true]; // a stream-backed record
    let mut st = Staging::default();
    st.b[B_STATE] = 2;
    st.b[B_RECHI] = 0xAB; // $AB06: record 0 of the buffer
    st.b[B_RECLO] = 0x06;
    st.b[B_T24] = 0x1E; // one more call reaches $20
    obj.handler_pickup16(&mut st, &t);
    assert_eq!(obj.done, 2);
    assert_eq!(st.b, [0u8; 32], "the slot zeroes");
    assert_eq!(obj.active, 0);
    assert_eq!(obj.money_bag, 0, "no coin credit in remove-mode 2");
    assert!(obj.spawned[0], "mode 2 keeps the record marker");
    assert_eq!(obj.remove_mode, 0, "$5B2D7 resets the mode");
}

/// The full handshake through the real entry points: frame 1's `$2DC6`
/// stages the mushroom id (type `$81` → `$01`) off the touch and the object
/// pass switches the slot to the anim-16 pickup; the following frames run
/// `$586F5` and the item despawns 31 engine frames after the touch.
#[test]
fn stepped_mushroom_touch_stages_the_id_and_despawns() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.active = 1;
    obj.spawned = vec![true];
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_DEF] = 18;
        s.b[B_TYPE] = 0x81; // bit 7 set, low 7 bits = id 1 (mushroom)
        s.b[B_RECHI] = 0xAB;
        s.b[B_RECLO] = 0x06;
        set_x(&mut s.b, 100);
        set_y(&mut s.b, 100);
        s.b[B_SCRY] = 99; // the staged screen pos the box test reads
        s.b[B_SCRX] = 96;
    }
    let mario = MarioContact {
        x: 0,
        y: 0,
        ..Default::default()
    };
    obj.step(0, 0, &layout, &t, &mario, false);
    assert_eq!(obj.pending_item, 1, "$2DC6 stages type & $7F");
    assert_eq!(obj.slots[0].b[B_FLAGS], 0, "the touch flag was consumed");
    assert_eq!(obj.slots[0].b[B_DEF], 16, "the touch arm switched anim 16");
    assert_eq!(obj.slots[0].b[B_SPRITE], 26);
    // The next frame is the pickup handler's first call.
    obj.step(0, 0, &layout, &t, &mario, false);
    assert_eq!(obj.slots[0].b[B_T24], 2);
    assert_eq!(y_of(&obj.slots[0].b), 98);
    assert_eq!(obj.slots[0].b[B_FSTATE], 1);
    // The parity gate halves the cadence; by 31 engine frames it is gone.
    for _ in 0..40 {
        obj.step(0, 0, &layout, &t, &mario, false);
    }
    assert_eq!(obj.slots[0].b[B_STATE], 0, "the item despawned");
    assert_eq!(obj.active, 0);
    assert_eq!(obj.money_bag, 0, "a pickup is not a coin");
    assert!(obj.spawned[0], "remove-mode 2 keeps the record marker");
    assert!(obj.unported.is_empty(), "no unported handler was reached");
}

/// Pin the extraction wiring: level 0's anim 16 resolves to `$586F5` in the
/// committed `extracted/objects.json`, in both modes, and `run_handler`
/// routes it to a ported body (no `unported` note). Skips when the
/// extraction is not built.
#[test]
fn extracted_anim16_profile_points_at_the_pickup_handler() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json");
    let Ok(txt) = std::fs::read_to_string(path) else {
        eprintln!("skip: {path} not built");
        return;
    };
    let t: ObjectTables = serde_json::from_str(&txt).expect("objects.json parses");
    assert_eq!(t.behavior_for(0, 16), Some(0x586F5));
    assert_eq!(t.behavior_for(0x0B, 16), Some(0x586F5), "mode B too");
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = Staging::default();
    st.b[B_DEF] = 16;
    obj.run_handler(&mut st, &blank_layout(), &t);
    assert!(obj.unported.is_empty(), "anim 16 must have a ported body");
}

/// Anim 22 — the goal bell's own script (`$587BD`) and its ring tick
/// (`$5B545`). Un-touched it re-stages type `$86` every frame (item id 6, the
/// strike); one touch — `+$17` bit 4, which `$2DC6` sets — flips it to frame
/// state 1 with the type cleared, and from then on the tick walks the
/// 4-phase ping-pong with the X mirror on phase 3, one phase per `$FFF0 & 3`.
///
/// The same tick routine serves anim 23 and *is* capture-verified through it
/// (traj13's touch shows the 8-frame steps and the mirror at phase 3), but no
/// capture ever touches anim 22: level 1's bell sits at anchor y 360, the
/// measured jump apex on that floor is y 399, and the touch window is
/// `marioY - objectY` in `[-43, -13)` — 44 px short. This test is that gap's
/// only coverage, and the cadence it pins is the console-measured one.
#[test]
fn the_goal_bells_own_script_arms_then_ping_pongs() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json");
    let Ok(txt) = std::fs::read_to_string(path) else {
        eprintln!("skip: {path} not built");
        return;
    };
    let t: ObjectTables = serde_json::from_str(&txt).expect("objects.json parses");
    assert_eq!(
        t.behavior_for(0, 22),
        Some(0x587BD),
        "anim 22 is the bell's script"
    );

    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = Staging::default();
    st.b[B_DEF] = 22;
    st.b[B_SPRITE] = 0x1F; // anim_to_sprite_a[22], what $8240 stages

    // Un-rung: the type is re-staged every frame, which is what lets the
    // strike happen at all.
    obj.fff0 = 0;
    obj.run_handler(&mut st, &blank_layout(), &t);
    assert!(obj.unported.is_empty(), "the bell's script is ported");
    assert_eq!(st.b[B_TYPE], 0x86, "un-rung type = item id 6");
    assert_eq!(st.b[B_FSTATE], 0, "frame state 0");
    assert_eq!(st.b[B_SPRITE], 0x1F, "and the sprite is untouched");

    // One touch.
    st.b[B_FLAGS] |= 0x10; // +$17 bit 4, set by $2DC6
    obj.run_handler(&mut st, &blank_layout(), &t);
    assert_eq!(st.b[B_FSTATE], 1, "ringing");
    assert_eq!(st.b[B_TYPE], 0, "and no longer collectible");
    assert_eq!(st.b[B_SPRITE], 0x1F, "the touch frame does not tick");

    // The tick: `$FFF0 & 3` gates it, so one phase per 4 engine frames = 8
    // game frames, which is the cadence the console showed.
    st.b[B_FLAGS] &= !0x10;
    for (i, (sprite, mirror)) in [(0x20u8, 0u8), (0x1F, 0), (0x20, 0x20), (0x1F, 0)]
        .into_iter()
        .enumerate()
    {
        obj.fff0 = 4; // == 0 mod 4: the gate opens
        obj.run_handler(&mut st, &blank_layout(), &t);
        let n = i + 1;
        assert_eq!(st.b[B_ANIMFRAME], (n % 4) as u8, "counter after tick {n}");
        assert_eq!(st.b[B_SPRITE], sprite, "sprite after tick {n}");
        assert_eq!(st.b[B_FLIP] & 0x20, mirror, "mirror after tick {n}");
    }

    // A frame whose gate is closed changes nothing at all.
    obj.fff0 = 5;
    let before = (st.b[B_SPRITE], st.b[B_ANIMFRAME], st.b[B_FLIP]);
    obj.run_handler(&mut st, &blank_layout(), &t);
    assert_eq!(
        (st.b[B_SPRITE], st.b[B_ANIMFRAME], st.b[B_FLIP]),
        before,
        "$5B545 returns without ticking when $FFF0 & 3 != 0"
    );
}

/// `$EC40`'s sprite-map mode rule — `ld a,[sCurLevel] / cp $0A / ret c`, so
/// level `$0A` itself already uses the "B" pair. The port had `> 0x0A`, which
/// put level `$0A` in mode A: one level's worth of objects and anim profiles
/// read from the wrong table.
#[test]
fn the_sprite_map_mode_rule_is_inclusive_at_0a() {
    assert_eq!(crate::object::sprite_map_index(0x00), 0);
    assert_eq!(crate::object::sprite_map_index(0x09), 0);
    assert_eq!(
        crate::object::sprite_map_index(0x0A),
        1,
        "$EC52: cp $0A / ret c — $0A is mode B"
    );
    assert_eq!(crate::object::sprite_map_index(0x1F), 1);
}

/// `$58100` is a bare `ret` — the ROM byte is `$C9`, immediately after the
/// 192-byte mode-A table at `$58040` — and nineteen mode-A anims plus eleven
/// mode-B ones dispatch to it. It is a *fixture*, not a gap: the `$8000`
/// prelude still runs, only the per-anim body does nothing. Routing it
/// explicitly is what keeps it out of `unported`, where it would read as
/// port debt and could turn into a spurious gate `note:`.
#[test]
fn the_58100_no_op_is_dispatched_and_not_flagged() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json");
    let Ok(txt) = std::fs::read_to_string(path) else {
        eprintln!("skip: {path} not built");
        return;
    };
    let t: ObjectTables = serde_json::from_str(&txt).expect("objects.json parses");
    // One of the nineteen from each mode's index list.
    assert_eq!(t.behavior_for(0, 5), Some(0x58100), "mode A anim 5");
    assert_eq!(t.behavior_for(0x0B, 40), Some(0x58100), "mode B anim 40");

    for (level, anim) in [(0u8, 5u8), (0x0B, 40)] {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(level);
        let mut st = Staging::default();
        st.b[B_DEF] = anim;
        obj.run_handler(&mut st, &blank_layout(), &t);
        assert!(
            obj.unported.is_empty(),
            "level {level} anim {anim}: the no-op must not be flagged"
        );
    }
}

// -----------------------------------------------------------------
// $586EE / $5B1C8 / $5B241 / $5B28C — level 19's four new classes
// -----------------------------------------------------------------

/// `$586EE` (mode-B anims 27/28/29): type = anim + `$66` and nothing else.
/// The ROM's stream bytes for these records are `$1B`/`$1C`/`$1D` (the
/// extractor's `r5` IS the staged anim byte), so the types come out
/// `$81`/`$82`/`$83` — the spec's `$8D`-`$8F` reading added `$66` to
/// `$27`-style values the stream never contains.
#[test]
fn marker66_sets_the_type_from_the_anim() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    for (anim, ty) in [(27u8, 0x81u8), (28, 0x82), (29, 0x83)] {
        let mut st = Staging::default();
        st.b[B_DEF] = anim;
        st.b[B_T24] = 0x0A;
        obj.handler_marker66(&mut st);
        assert_eq!(st.b[B_TYPE], ty, "anim {anim} -> type ${ty:02X}");
        assert_eq!(st.b[B_T24], 0x0A, "the marker touches nothing else");
        assert_eq!(obj.done, 0);
    }
    assert!(obj.unported.is_empty());
}

/// The four handlers the level-19 stream's cells reach are the class
/// claims: `$586EE`'s anims are the literal record bytes `$1B`-`$1D`, which
/// is why its type arithmetic lands on `$81`-`$83`.
#[test]
fn level19_record_anims_are_the_staged_bytes() {
    // From the level-19 packed stream (`baserom.gb` file offset $E7C8, 91
    // bytes): pack `64 61 06` -> anim 27, `75 DB 0E` -> 27, and the anim-92
    // packs `77 9B 98`/`4D F3 14`... decode to 92.
    let decode = |c: u8, b2: u8, _b3: u8| {
        ((c >> 5) & 7) * 8 + if c & 0x10 != 0 { 0x40 } else { 0 } + ((b2 >> 5) & 7)
    };
    assert_eq!(decode(0x64, 0x61, 0x06), 27, "the anim-27 record");
    assert_eq!(decode(0x77, 0x9B, 0x98), 92, "an anim-92 record");
    assert_eq!(decode(0x72, 0xC5, 0x96), 94, "the first anim-94 record");
    assert_eq!(decode(0x72, 0xFB, 0x9E), 95, "the anim-95 record");
    // anim byte 27 + $66 = $81: the `$3451` powerup arm's id 1.
    assert_eq!(27u8 + 0x66, 0x81);
}

/// `$5B1C8` states 0/1: the arm frame takes no step; the nine state-1
/// calls are three segments of three 1-px steps — up while `+16 == 0`,
/// down while `+16 == 1` — with `+24` reloading from `+25` and `+16`
/// flipping after the first two segments; the third exhausts `+26` into
/// type 0 and state 2.
#[test]
fn grower92_walks_three_segments_then_goes_harmless() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = Staging::default();
    st.b[B_SPRITE] = 122;
    set_y(&mut st.b, 100);
    obj.handler_grower92(&mut st);
    assert_eq!(
        (
            st.b[B_TYPE],
            st.b[B_FSTATE],
            st.b[B_T24],
            st.b[B_T25],
            st.b[B_ANIMFRAME]
        ),
        (0x4C, 1, 3, 3, 3)
    );
    assert_eq!(y_of(&st.b), 100, "state 0 takes no step");
    for expect in [99u16, 98, 97] {
        obj.handler_grower92(&mut st);
        assert_eq!(y_of(&st.b), expect, "segment 1 rises");
    }
    assert_eq!(
        (st.b[B_PHASE], st.b[B_T24], st.b[B_ANIMFRAME]),
        (1, 3, 2),
        "reload + flip after the first segment"
    );
    for expect in [98u16, 99, 100] {
        obj.handler_grower92(&mut st);
        assert_eq!(y_of(&st.b), expect, "segment 2 sinks");
    }
    assert_eq!((st.b[B_PHASE], st.b[B_ANIMFRAME]), (0, 1));
    for expect in [99u16, 98, 97] {
        obj.handler_grower92(&mut st);
        assert_eq!(y_of(&st.b), expect, "segment 3 rises");
    }
    assert_eq!((st.b[B_TYPE], st.b[B_FSTATE], st.b[B_ANIMFRAME]), (0, 2, 0));
}

/// `$5B1C8` states 2/3: the sprite id walks 122 → 124 on every 4th `$FFF0`
/// frame, `+28` = 32 is armed with the state change, 32 calls count it down
/// without touching the id, and the id walks back to 122 (also on 4th
/// frames), which jumps to the state-0 body and re-arms it.
#[test]
fn grower92_sprite_cycle_holds_32_calls_and_rearms() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = Staging::default();
    st.b[B_FSTATE] = 2;
    st.b[B_SPRITE] = 122;
    obj.fff0 = 1;
    obj.handler_grower92(&mut st);
    assert_eq!(st.b[B_SPRITE], 122, "only every 4th $FFF0 frame steps");
    obj.fff0 = 0;
    obj.handler_grower92(&mut st);
    assert_eq!(st.b[B_SPRITE], 123);
    obj.fff0 = 4;
    obj.handler_grower92(&mut st);
    assert_eq!(
        (st.b[B_SPRITE], st.b[B_T28], st.b[B_FSTATE]),
        (124, 0x20, 3)
    );
    for _ in 0..32 {
        obj.handler_grower92(&mut st);
    }
    assert_eq!(
        (st.b[B_SPRITE], st.b[B_T28]),
        (124, 0),
        "the hold is 32 calls"
    );
    obj.fff0 = 5;
    obj.handler_grower92(&mut st);
    assert_eq!(st.b[B_SPRITE], 124, "not a 4th frame");
    obj.fff0 = 8;
    obj.handler_grower92(&mut st);
    assert_eq!(st.b[B_SPRITE], 123);
    obj.fff0 = 12;
    obj.handler_grower92(&mut st);
    assert_eq!(st.b[B_SPRITE], 122);
    assert_eq!(
        (st.b[B_TYPE], st.b[B_FSTATE], st.b[B_T24], st.b[B_ANIMFRAME]),
        (0x4C, 1, 3, 3),
        "the jr $71D3 re-arms state 0"
    );
}

/// `$5B241` (anim 94) + the `$7266` tail: state 0 arms type `$48` and state
/// 1 without stepping; a leg is 18 picker calls summing to 32 px, the last
/// of which resets `+16` at the `$12` sentinel and toggles `+24` — the
/// 0 → 1 edge flips the state, so the object floats up on `$74A1` and back
/// down on `$7488`, and the 1 → 0 edge leaves the state alone. (The capture
/// trace of level 19's anim-94 slot is exactly this: 176 → 144 → 176.)
#[test]
fn floater94_floats_up_then_down_through_the_shared_tail() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = Staging::default();
    set_y(&mut st.b, 200);
    obj.handler_floater94(&mut st);
    assert_eq!((st.b[B_TYPE], st.b[B_FSTATE]), (0x48, 1));
    assert_eq!(y_of(&st.b), 200, "state 0 takes no step");
    obj.handler_floater94(&mut st);
    assert_eq!((y_of(&st.b), st.b[B_PHASE]), (197, 1), "$74A1[0] = 3 up");
    for _ in 0..17 {
        obj.handler_floater94(&mut st);
    }
    assert_eq!(y_of(&st.b), 168, "32 px up over the leg");
    assert_eq!(
        (st.b[B_PHASE], st.b[B_T24], st.b[B_FSTATE]),
        (0, 1, 2),
        "the sentinel resets the profile, +24 flips the direction"
    );
    for _ in 0..18 {
        obj.handler_floater94(&mut st);
    }
    assert_eq!(y_of(&st.b), 200, "32 px back down on $7488");
    assert_eq!(
        (st.b[B_PHASE], st.b[B_T24], st.b[B_FSTATE]),
        (0, 0, 2),
        "the 1 -> 0 edge does not flip"
    );
}

/// `$5B28C` (anim 95): the same machine on X — `$74A1` steps left, `$7488`
/// steps right — with the shared state-0 body and `$7266` tail.
#[test]
fn floater95_shuttles_horizontally_with_the_shared_state0() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = Staging::default();
    set_x(&mut st.b, 300);
    obj.handler_floater95(&mut st);
    assert_eq!((st.b[B_TYPE], st.b[B_FSTATE]), (0x48, 1));
    assert_eq!(x_of(&st.b), 300);
    obj.handler_floater95(&mut st);
    assert_eq!((x_of(&st.b), st.b[B_PHASE]), (297, 1), "left, 3 px");
    st.b[B_T24] = 1;
    st.b[B_PHASE] = 11;
    obj.handler_floater95(&mut st);
    assert_eq!((x_of(&st.b), st.b[B_PHASE]), (294, 12), "left on $7488[11]");
    st.b[B_T24] = 0;
    st.b[B_PHASE] = 0x11; // a leg's last call: 17 -> the $12 sentinel
    obj.handler_floater95(&mut st);
    assert_eq!(
        (x_of(&st.b), st.b[B_PHASE], st.b[B_T24], st.b[B_FSTATE]),
        (294, 0, 1, 2),
        "$74A1[17] = 0: no step, reset + flip to state 2"
    );
    st.b[B_PHASE] = 0x11;
    obj.handler_floater95(&mut st);
    assert_eq!((x_of(&st.b), st.b[B_PHASE], st.b[B_T24]), (297, 0, 0));
    assert_eq!(st.b[B_FSTATE], 2, "the 1 -> 0 edge keeps the state");
}

/// The two floater profiles, byte-exact from `baserom.gb`: `$7488` (whose
/// base is `FALL_SPEED` — the c=`$15` fall consumer reads 22 of its bytes)
/// and `$74A1` (the c=`$12` twin). One leg each is 18 calls and sums to
/// 32 px.
#[test]
fn floater_profiles_are_the_rom_bytes_and_sum_to_32px_each() {
    assert_eq!(
        FALL_SPEED,
        [
            0x00, 0x00, 0x01, 0x01, 0x01, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x03, 0x02, 0x03,
            0x03, 0x03, 0x02, 0x03, 0x03, 0x02, 0x03, 0x03
        ],
        "$7488[0..22]"
    );
    assert_eq!(
        STEP_74A1,
        [
            0x03, 0x02, 0x03, 0x03, 0x03, 0x02, 0x03, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x01,
            0x01, 0x01, 0x00, 0x00, 0x00
        ],
        "$74A1[0..19]"
    );
    let left: u32 = STEP_74A1[..18].iter().map(|&v| v as u32).sum();
    let right: u32 = FALL_SPEED[..18].iter().map(|&v| v as u32).sum();
    assert_eq!((left, right), (32, 32), "one leg each");
}

/// Level 19's mode-B stream names exactly these four classes; all four now
/// have a body, so `run_handler` must dispatch them without leaving an
/// `unported` entry — which is the replay's `note:` line.
#[test]
fn level19_anims_dispatch_to_the_new_bodies_unflagged() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../extracted/objects.json");
    let Ok(txt) = std::fs::read_to_string(path) else {
        eprintln!("skip: {path} not built");
        return;
    };
    let t: ObjectTables = serde_json::from_str(&txt).expect("objects.json parses");
    for (anim, handler) in [
        (27u8, 0x586EEu32),
        (92, 0x5B1C8),
        (94, 0x5B241),
        (95, 0x5B28C),
    ] {
        assert_eq!(
            t.behavior_for(19, anim),
            Some(handler),
            "level 19 anim {anim}"
        );
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(19);
        let mut st = Staging::default();
        st.b[B_STATE] = 2;
        st.b[B_DEF] = anim;
        obj.run_handler(&mut st, &blank_layout(), &t);
        assert!(
            obj.unported.is_empty(),
            "anim {anim} -> ${handler:05X} must not be flagged"
        );
    }
}

// -----------------------------------------------------------------
// $2F72 / $86A0 — the below-hit system
// -----------------------------------------------------------------

/// A stream-backed staging (record pointer `+6/+7` = `$AB06`, record 0).
fn streamed(anim: u8, ty: u8, flags: u8) -> Staging {
    let mut st = Staging::default();
    st.b[B_STATE] = 2;
    st.b[B_DEF] = anim;
    st.b[B_TYPE] = ty;
    st.b[B_FLAGS] = flags;
    st.b[B_RECHI] = 0xAB;
    st.b[B_RECLO] = 0x06;
    st
}

/// One spawn record with a distinctive anim, so the `$86E4` record write
/// is observable.
fn records() -> Vec<SpawnRecord> {
    vec![SpawnRecord {
        x: 352,
        y: 448,
        anim_index: 17,
        mode_flag: 0,
        sprite_id: 25,
        hitbox: [0; 4],
        raw: [0x01, 0x60, 0x00, 0x01, 0xC0, 0x11],
    }]
}

/// `$86E4` → `$8721`: a bit-7 type with non-zero low bits (the `$86` bell,
/// the `$87`/`$88` poses, `$8A`+) re-derives its sprite with the type held
/// across `$8240`'s `+14..+30` clear, then arms the *revive* reaction
/// (`+27 = 2`) that `$5816C`'s landing branch turns back into a live
/// object. No anim swap, no frame state of its own — and no `unported`
/// note any more.
#[test]
fn below_bit7_types_re_derive_and_arm_the_revive_reaction() {
    let t = tables();
    for ty in [0x86u8, 0x87, 0x88, 0x8A, 0x90, 0xC0, 0xFF] {
        let mut obj = ObjectSim::new(records(), 0);
        obj.configure(0);
        let mut st = streamed(22, ty, 0x20);
        st.b[B_FSTATE] = 1;
        st.b[B_FLASH] = 0x10;
        set_x(&mut st.b, 352);
        set_y(&mut st.b, 448);
        obj.dispatch_below(&mut st, &t);
        assert!(obj.clear_stage_flags, "sClearStageFlags = 1");
        assert_eq!(st.b[B_TYPE], ty, "type ${ty:02X} survives the re-derive");
        assert_eq!(st.b[B_HITREACT], 2, "reaction 2 = $5819B's revive");
        assert_eq!(st.b[B_FSTATE], 0, "$8240 cleared +14..+30");
        assert_eq!(st.b[B_FLAGS], 0, "the below-hit flag was consumed");
        assert_eq!(st.b[B_FLASH], 0);
        assert_eq!(st.b[B_DEF], 22, "the anim is untouched");
        assert_eq!(st.b[B_SPRITE], 31, "map[22] re-derived");
        assert_eq!(obj.done, 1, "$863C sets sObjectDone");
        assert!(obj.unported.is_empty(), "the $86E4 cluster is ported");
    }
}

/// `$86E4`'s fall-through (type `$80` alone): the item pose. Anim `$11`
/// (the hopper's pose) with a powerup takes the collect arm — the spawn
/// record's anim byte is rewritten to 18 (`ptr+3` = record+5, so a later
/// re-spawn is the anim-18 pose) and the staged anim/sprite follow — then
/// `$8708` sets type = anim + `$70`, `+15 = 1`, Y += 16 and `+2 = $10`.
#[test]
fn below_pose80_with_a_powerup_collects_the_pose_and_rewrites_the_record() {
    let t = tables();
    let mut obj = ObjectSim::new(records(), 0);
    obj.configure(0);
    obj.mario.powerup = 1;
    let mut st = streamed(0x11, 0x80, 0);
    set_y(&mut st.b, 448);
    obj.dispatch_below(&mut st, &t);
    assert_eq!(obj.records[0].anim_index, 18, "the record's anim byte");
    assert_eq!(st.b[B_DEF], 18, "$FFD5 = 18");
    assert_eq!(st.b[B_SPRITE], 26, "$86E4 stages sprite 26");
    assert_eq!(st.b[B_TYPE], 0x82, "anim 18 + $70");
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(y_of(&st.b), 464, "$3A64: Y += 16");
    assert_eq!(st.b[B_MARKER], 0x10, "$FFD2 = $10");
    assert_eq!(st.b[B_HITREACT], 0, "this arm arms no reaction");
    assert_eq!(obj.done, 0, "no $863C on this arm");
    assert!(obj.unported.is_empty());
}

/// The same bit-7 type without a powerup (and any anim other than `$11`):
/// the collect arm is skipped — record, anim and sprite stay — and only
/// `$8708`'s pose swap runs.
#[test]
fn below_pose80_without_the_powered_11_pose_just_swaps_the_type() {
    let t = tables();
    for (anim, powerup) in [(0x11u8, 0u8), (18, 1), (18, 0)] {
        let mut obj = ObjectSim::new(records(), 0);
        obj.configure(0);
        obj.mario.powerup = powerup;
        let mut st = streamed(anim, 0x80, 0);
        obj.dispatch_below(&mut st, &t);
        assert_eq!(obj.records[0].anim_index, 17, "the record is untouched");
        assert_eq!(st.b[B_DEF], anim);
        assert_eq!(st.b[B_SPRITE], 0, "no sprite staged");
        assert_eq!(st.b[B_TYPE], anim.wrapping_add(0x70));
        assert_eq!(st.b[B_FSTATE], 1);
        assert_eq!(y_of(&st.b), 16, "the stock y=0 + 16");
        assert_eq!(st.b[B_MARKER], 0x10);
        assert_eq!(obj.done, 0);
    }
}

/// The plain (bit-7-clear) arms: `+15 = 0` first, then `$86C6`'s anim bump
/// for the `$10` class — bit3 clear → `$8633` (anim + 1), bit3 set →
/// `$85D7`/`$85DC` (anim − 1, sprite re-derived) — and the `$85F0` defeat
/// for the `$08` class; anything else just re-derives (`$863C`) and takes
/// reaction 1.
#[test]
fn below_plain_types_bump_the_anim_or_defeat() {
    let t = tables();
    // $10 class, bit3 clear: anim + 1, re-derive, reaction 1.
    let mut obj = ObjectSim::new(records(), 0);
    let mut st = streamed(18, 0x10, 0x20);
    st.b[B_FSTATE] = 3;
    obj.dispatch_below(&mut st, &t);
    assert_eq!((st.b[B_DEF], st.b[B_SPRITE]), (19, 0));
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(st.b[B_HITREACT], 1);
    assert_eq!(obj.done, 1);
    // $10 class with bit3: $86D2's anim − 1 (and the sprite re-derive).
    let mut obj = ObjectSim::new(records(), 0);
    let mut st = streamed(20, 0x18, 0x20);
    obj.dispatch_below(&mut st, &t);
    assert_eq!((st.b[B_DEF], st.b[B_SPRITE]), (19, 0));
    assert_eq!(st.b[B_HITREACT], 1);
    assert_eq!(obj.done, 1);
    // $08 class: the defeat — coin credit, type/anim/state cleared.
    let mut obj = ObjectSim::new(records(), 0);
    let mut st = streamed(20, 0x08, 0x20);
    obj.dispatch_below(&mut st, &t);
    assert_eq!(st.b[B_TYPE], 0);
    assert_eq!(st.b[B_DEF], 0);
    assert_eq!(obj.money_bag, 1, "the defeat credits the coin counter");
    assert_eq!(obj.done, 1);
    // Neither bit: a plain re-derive + reaction 1 (sfx 6, audio-only).
    let mut obj = ObjectSim::new(records(), 0);
    let mut st = streamed(22, 0x02, 0x20);
    obj.dispatch_below(&mut st, &t);
    assert_eq!((st.b[B_DEF], st.b[B_SPRITE]), (22, 31));
    assert_eq!(st.b[B_HITREACT], 1);
    assert_eq!(obj.done, 1);
}

/// `$2F72`: the pass rewrites `$1DFE`'s world probe point into screen
/// coords — `(Y & $F0) + 8 − sScrollY − 16`, `((X + 8) & $F0) − sScrollX −
/// 8` — and boxes each slot's staged `+9/+10` against it (|dy| < `$10`,
/// |dx| < `$14`).
///
/// The traj15 geometry as the numbers: the `$01` block's cell is at world
/// (352-367, 448-463), Mario's probe (360, 479), the camera (448, 377) →
/// sScroll (368, 305) → the block's centre in screen bytes (248, 151) =
/// `($F8, $97)`. Slots with a flag already set, or a zero type, are
/// skipped; a type-`$80` hit raises `sObjectBumpSpecial`.
#[test]
fn below_bonk_marks_idle_objects_in_the_tile_window() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let put = |obj: &mut ObjectSim, i: usize, ty: u8, flags: u8, scry: u8, scrx: u8| {
        obj.slots[i].b[B_TYPE] = ty;
        obj.slots[i].b[B_FLAGS] = flags;
        obj.slots[i].b[B_SCRY] = scry;
        obj.slots[i].b[B_SCRX] = scrx;
    };
    put(&mut obj, 0, 0x86, 0, 0x97, 0xF8); // the bell pose on the tile
    put(&mut obj, 1, 0x80, 0, 0x9C, 0x04); // a pose, in the window
    put(&mut obj, 2, 0x4E, 0, 0x97, 0x60); // the walker, 100 px east
    put(&mut obj, 3, 0x81, 0x10, 0x97, 0xF8); // touched: owns its arm
    put(&mut obj, 4, 0, 0, 0x97, 0xF8); // no type
    put(&mut obj, 5, 0x4E, 0, 0xA7, 0xF8); // 16 px off in Y
    let special = obj.below_bonk((0xDF, 0x68), 0x31, 0x70);
    assert!(special, "the $80 pose raises sObjectBumpSpecial");
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x20);
    assert_eq!(obj.slots[1].b[B_FLAGS], 0x20);
    assert_eq!(obj.slots[2].b[B_FLAGS], 0);
    assert_eq!(obj.slots[3].b[B_FLAGS], 0x10, "already flagged");
    assert_eq!(obj.slots[4].b[B_FLAGS], 0, "zero type");
    assert_eq!(obj.slots[5].b[B_FLAGS], 0, "$A7 is one window step out");
    // With no pose at the point the flag stays low.
    obj.slots[1].b[B_TYPE] = 0x86;
    obj.slots[1].b[B_FLAGS] = 0;
    obj.slots[0].b[B_FLAGS] = 0;
    assert!(!obj.below_bonk((0xDF, 0x68), 0x31, 0x70));
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x20);
    assert_eq!(obj.slots[1].b[B_FLAGS], 0x20);
}

/// The whole handshake through the real entry points: `$2F72`'s pass
/// flags the bell pose, the same frame's object engine turns the flag into
/// `$8721` (sprite re-derived from anim 22, type held, `+27 = 2`), and the
/// next calls run `$58101`'s frame-state-0 arm — face Mario, then the
/// `$20` flip toggle — and the knockback (`$411A`: X + 1, Y − the bounce
/// curve's first step 3, `+16` 0 → 1) on the slot-parity cadence.
///
/// Measured on the console (the historical below-hit probe pokes the same
/// state into slot 0 and logs the 32 bytes per frame): sprite `$7F` →
/// `$1F`, type `$86` kept, `+23` `$20` → 0, `+27` → 2, `+15` → 0, then
/// `+15` 1 / `+13` `$20`, then (one parity frame later) (100,468) →
/// (101,465) → (102,462).
#[test]
fn stepped_below_hit_on_the_bell_re_derives_and_faces_mario() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(records(), 0);
    obj.configure(0);
    obj.active = 1;
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_DEF] = 22;
        s.b[B_TYPE] = 0x86;
        s.b[B_SPRITE] = 0x7F; // bogus on purpose: the re-derive shows
        s.b[B_RECHI] = 0xAB;
        s.b[B_RECLO] = 0x06;
        set_x(&mut s.b, 352);
        set_y(&mut s.b, 448);
        s.b[B_SCRY] = 0x97;
        s.b[B_SCRX] = 0xF8;
    }
    // Mario just west of the block, standing below it — the camera holds
    // the traj15 bonk point's frame.
    let mario = MarioContact {
        x: 336,
        y: 448,
        ..Default::default()
    };
    let special = obj.below_bonk((0xDF, 0x68), 0x31, 0x70);
    assert!(!special);
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x20, "hit from below");
    obj.step(448, 377, &layout, &t, &mario, false);
    let s = &obj.slots[0];
    assert_eq!(s.b[B_TYPE], 0x86, "the bell keeps its type");
    assert_eq!(s.b[B_HITREACT], 2);
    assert_eq!(s.b[B_SPRITE], 31, "map[22]");
    assert_eq!(s.b[B_FLAGS], 0, "$818D cleared the flag");
    assert_eq!(s.b[B_FSTATE], 0, "$8240 cleared +14..+30");
    // The reaction's first arm: face Mario (west → state 1, flip 0) and
    // then the toggle → flip $20.
    obj.step(448, 377, &layout, &t, &mario, false);
    let s = &obj.slots[0];
    assert_eq!(s.b[B_FSTATE], 1);
    assert_eq!(s.b[B_FLIP], 0x20);
    assert_eq!(s.b[B_HITREACT], 2, "still owed: the revive clears it");
    // The $411A knockback arm — the console's (100,468) → (101,465),
    // i.e. X + 1 and Y − BOUNCE_SPEED[0] = 3 with +16 0 → 1.
    obj.step(448, 377, &layout, &t, &mario, false);
    let s = &obj.slots[0];
    assert_eq!((s.x(), s.y()), (353, 445));
    assert_eq!(s.b[B_PHASE], 1);
    assert_eq!(s.b[B_FSTATE], 1);
    assert!(obj.unported.is_empty());
}

// -----------------------------------------------------------------
// Level 3 (index 2) — the three new handler classes
// $59BC5 (anim 88) / $594EB (anim 68) / $58DD1 (anim 46)
// -----------------------------------------------------------------

/// A layout of `$60` — no wall (`>= $38`) and no ground (`>= $48`) at any
/// probe — with one cell overridden. The entity probes read the cell
/// `(x >> 4, y >> 4)`.
fn one_tile(x: usize, y: usize, tile: u8) -> LevelLayout {
    let mut cells = vec![0x60u8; LAYOUT_SIZE];
    cells[y * crate::level::LAYOUT_WIDTH + x] = tile;
    LevelLayout::from_bytes(&cells).unwrap()
}

/// `$59BC5` (`$5BD1`): the state-0 one-shot — flash `$10`, type `$4E`,
/// `face_mario` (`$73C6` picks state 1/2 and `$FFDD` 0/`$20` from Mario's
/// side) and `+28` = 1. The `$7585` bob head runs first on even `$FFF0`:
/// `+26` cycles and the sprite takes `[FE, 01, FF, 02]`.
#[test]
fn groundwalker88_state0_sets_flash_type_faces_mario_and_bobs() {
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    st.b[B_SPRITE] = 73; // anim_to_sprite_a[88]
    obj.mario_ref_x = 108; // east of the object: $73C6 -> 2 / $20
    obj.fff0 = 0; // even: the bob head steps
    obj.handler_groundwalker88(&mut st, &layout);
    assert_eq!(st.b[B_FLASH], 0x10);
    assert_eq!(st.b[B_TYPE], 0x4E);
    assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (2, 0x20), "Mario east");
    assert_eq!(st.b[B_T28], 1, "the walk arm is armed");
    assert_eq!((st.b[B_ANIMFRAME], st.b[B_SPRITE]), (1, 74), "bob: +1");

    // Mario west on the same slot's next init call.
    st.b[B_FSTATE] = 0;
    obj.mario_ref_x = 92;
    obj.fff0 = 2; // even again
    obj.handler_groundwalker88(&mut st, &layout);
    assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (1, 0), "Mario west");
    assert_eq!((st.b[B_FLASH], st.b[B_TYPE], st.b[B_T28]), (0x10, 0x4E, 1));
    assert_eq!((st.b[B_ANIMFRAME], st.b[B_SPRITE]), (2, 73), "bob: -1");
    assert!(obj.unported.is_empty());
}

/// `$59BC5` states 1/2 (`$5D0F`/`$5D4B`, the `$5D22` tail): the state-0
/// init leaves `+28` = 1, so the very next call arms `+25` = `$30` and
/// does not move, the **48** calls after that each take one 1-px step
/// (the 48th leaving `+28` = `$10`), the next **16** only count (the 16th
/// re-arming `+25`), and the cycle repeats. The blank layout's `$60`
/// cells are neither walls nor ground, so nothing interrupts the machine.
#[test]
fn groundwalker88_counter_cadence_is_16_idle_calls_then_48_steps() {
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.fff0 = 1; // odd: the bob head is a no-op
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    obj.mario_ref_x = 92; // west: state 1, walks left
    obj.handler_groundwalker88(&mut st, &layout);
    assert_eq!((st.b[B_FSTATE], st.b[B_T28], st.b[B_T25]), (1, 1, 0));

    obj.handler_groundwalker88(&mut st, &layout); // +28 1 -> 0: arms +25
    assert_eq!(
        (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
        (100, 0, 0x30),
        "the count-to-zero call arms $30 and does not move"
    );
    for _ in 0..47 {
        obj.handler_groundwalker88(&mut st, &layout);
    }
    assert_eq!(
        (x_of(&st.b), st.b[B_T25]),
        (53, 1),
        "47 one-pixel steps, one still budgeted"
    );
    obj.handler_groundwalker88(&mut st, &layout); // the 48th step
    assert_eq!(
        (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
        (52, 0x10, 0),
        "the 48th step exhausts +25 and re-arms the pause"
    );
    for _ in 0..15 {
        obj.handler_groundwalker88(&mut st, &layout);
    }
    assert_eq!(
        (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
        (52, 1, 0),
        "16 countdown calls, no motion"
    );
    obj.handler_groundwalker88(&mut st, &layout);
    assert_eq!(
        (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
        (52, 0, 0x30),
        "the 16th count re-arms the step budget"
    );
    obj.handler_groundwalker88(&mut st, &layout);
    assert_eq!(x_of(&st.b), 51, "and the next call steps again");
}

/// `$59BC5` states 1/2: a wall on the moving side flips through `$5D2D`
/// (`+13 ^= $20`, `+15 ^= 3` — 1 <-> 2) and touches neither counter nor
/// X. The probes sit at X∓7, Y−11 / Y−5 (`$3C1C`/`$3BB8` -> the port's
/// `m_left_wall`/`m_right_wall`), so the blocking tile goes in the cell
/// at `(x∓7) >> 4`.
#[test]
fn groundwalker88_side_walls_flip_the_walk_state_in_both_directions() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.fff0 = 1;
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200); // the left probes read (5, 11) and (5, 12)
    st.b[B_FSTATE] = 1;
    st.b[B_T28] = 5;
    st.b[B_T25] = 0x30;
    st.b[B_FLIP] = 0;

    let layout = one_tile(5, 11, 0x00); // a wall left of x = 100
    obj.handler_groundwalker88(&mut st, &layout);
    assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (2, 0x20), "1 -> 2, flipped");
    assert_eq!(
        (x_of(&st.b), st.b[B_T28], st.b[B_T25]),
        (100, 5, 0x30),
        "a wall frame touches neither counter nor X"
    );

    // The right probe reads (107 >> 4, y) = the cell (6, ...).
    let layout = one_tile(6, 11, 0x00);
    obj.handler_groundwalker88(&mut st, &layout);
    assert_eq!((st.b[B_FSTATE], st.b[B_FLIP]), (1, 0), "2 -> 1, unflipped");
    assert_eq!((x_of(&st.b), st.b[B_T28], st.b[B_T25]), (100, 5, 0x30));
    assert!(obj.unported.is_empty());
}

/// `$594EB` (anim 68) and `$582D1` (anim 1) are one body: identical init
/// calls produce byte-identical stagings except `+18` — `$52` vs `$56`,
/// the one immediate `$54F4` differs by.
#[test]
fn fallwalker68_is_the_shared_body_with_type52() {
    let layout = blank_layout();
    let mut a = ObjectSim::new(Vec::new(), 0);
    let mut b = ObjectSim::new(Vec::new(), 0);
    a.fff0 = 1; // odd: the shared bob head is a no-op
    b.fff0 = 1;
    let mut sa = Staging::default();
    let mut sb = Staging::default();
    for st in [&mut sa, &mut sb] {
        set_x(&mut st.b, 100);
        set_y(&mut st.b, 200);
        st.b[B_SPRITE] = 125; // anim_to_sprite_a[68]
    }
    a.handler_fallwalker(&mut sa, &layout);
    b.handler_fallwalker52(&mut sb, &layout);
    assert_eq!((sa.b[B_TYPE], sb.b[B_TYPE]), (0x56, 0x52));
    assert_eq!(sa.b[B_PHASE], 1, "no ground: the fall curve stepped");
    assert_eq!(y_of(&sa.b), 200, "step 0 of the curve is 0 px");
    let (mut ta, mut tb) = (sa.b, sb.b);
    ta[B_TYPE] = 0;
    tb[B_TYPE] = 0;
    assert_eq!(ta, tb, "the whole body but the type byte");
}

/// The `run_handler` arms: an in-memory level-2 table routes anims
/// 46/68/88 to `$58DD1`/`$594EB`/`$59BC5`, each body runs (anim 68's type
/// is `$52`, not the `$582D1` arm's `$56`) and none is flagged unported —
/// a flagged handler is the gate's `note:` line.
#[test]
fn level3_anims_dispatch_to_the_new_bodies_unflagged() {
    let mut t = tables();
    t.anim_profiles_a[46] = prof(0x58DD1);
    t.anim_profiles_a[68] = prof(0x594EB);
    t.anim_profiles_a[88] = prof(0x59BC5);
    let layout = blank_layout();
    for (anim, handler, ty) in [
        (46u8, 0x58DD1u32, 0x20u8),
        (68, 0x594EB, 0x52),
        (88, 0x59BC5, 0x4E),
    ] {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.configure(2); // level index 2 -> mode A
        let mut st = Staging::default();
        st.b[B_DEF] = anim;
        obj.run_handler(&mut st, &layout, &t);
        assert_eq!(st.b[B_TYPE], ty, "anim {anim} -> ${handler:05X}");
        assert!(obj.unported.is_empty(), "anim {anim} must not be flagged");
    }
}

/// `$58DD1` state 0 (`$4DD1`): the one-shot — type `$20` (the on-object
/// arm: Mario stands on it) and state 2. Nothing else is touched.
#[test]
fn carrier46_state0_sets_type20_and_enters_the_wait() {
    let layout = blank_layout();
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = Staging::default();
    set_x(&mut st.b, 1712); // level 3's record: (1712, 176)
    set_y(&mut st.b, 176);
    st.b[B_SPRITE] = 35; // anim_to_sprite_a[46]
    obj.handler_carrier46(&mut st, &layout, &t);
    assert_eq!((st.b[B_TYPE], st.b[B_FSTATE]), (0x20, 2));
    assert_eq!((y_of(&st.b), st.b[B_T28]), (176, 0), "no move, no timer");
    assert_eq!(obj.done, 0);
    assert!(obj.unported.is_empty());
}

/// `$58DD1` state 2 (`$4E04`): `+23` bit 3 — the `$2DF7` on-object flag
/// the port's `respond` stages — is the only gate; any other flag bit
/// leaves it waiting. The handover call arms state 1 with `+28` = 28 and
/// itself neither moves nor carries.
#[test]
fn carrier46_state2_waits_for_the_stood_on_flag() {
    let layout = blank_layout();
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.mario.y = 500;
    let mut st = Staging::default();
    set_y(&mut st.b, 176);
    st.b[B_FSTATE] = 2;
    st.b[B_FLAGS] = 0x04; // some other interaction flag
    obj.handler_carrier46(&mut st, &layout, &t);
    assert_eq!(
        (st.b[B_FSTATE], st.b[B_T28], y_of(&st.b)),
        (2, 0, 176),
        "bit 3 clear: still waiting"
    );
    st.b[B_FLAGS] = 0x08; // stood on this frame
    obj.handler_carrier46(&mut st, &layout, &t);
    assert_eq!((st.b[B_FSTATE], st.b[B_T28]), (1, 0x1C));
    assert_eq!(
        (y_of(&st.b), obj.mario_y_adjust),
        (176, None),
        "the handover call does not move or carry"
    );
    assert_eq!(obj.done, 0);
}

/// `$58DD1` state 1 (`$4DE5`/`$4DEE`): `+28` = 28 counts down — those
/// calls do nothing and the call that takes it to 0 falls *through* to
/// the move — then every call steps the `$7488` curve (`$7451`), moves
/// the object down (`$3D3D`), carries Mario by the same `b` (`$3CDF`,
/// only while `+23` bit 3 is held) and probes the ground (`$3B40`).
#[test]
fn carrier46_counts_down_then_falls_and_carries_mario() {
    let layout = blank_layout(); // $60: no ground anywhere
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.mario.y = 500;
    let mut st = Staging::default();
    set_y(&mut st.b, 400);
    st.b[B_FSTATE] = 1;
    st.b[B_T28] = 3;
    st.b[B_PHASE] = 2; // $7488[2] = 1 px
    st.b[B_FLAGS] = 0x08;
    for _ in 0..2 {
        obj.handler_carrier46(&mut st, &layout, &t);
    }
    assert_eq!(
        (y_of(&st.b), st.b[B_T28], st.b[B_PHASE]),
        (400, 1, 2),
        "the countdown calls do not move"
    );
    obj.handler_carrier46(&mut st, &layout, &t); // +28 1 -> 0: the move
    assert_eq!(
        (y_of(&st.b), st.b[B_T28], st.b[B_PHASE]),
        (401, 0, 3),
        "the count-to-zero call falls through"
    );
    assert_eq!(obj.mario_y_adjust, Some(501), "carried by b = 1");
    // +28 already 0: the next call moves immediately, and the carry adds
    // onto the Y staged this frame (the ROM reads the live $FFC0).
    obj.handler_carrier46(&mut st, &layout, &t);
    assert_eq!((y_of(&st.b), st.b[B_PHASE]), (402, 4));
    assert_eq!(obj.mario_y_adjust, Some(502));
    // A fresh frame (the $2D7D pass resets the Option) with Mario off:
    // the object keeps falling, the carry stays closed.
    obj.mario_y_adjust = None;
    st.b[B_FLAGS] = 0;
    obj.handler_carrier46(&mut st, &layout, &t);
    assert_eq!((y_of(&st.b), obj.mario_y_adjust), (403, None));
    assert_eq!(obj.done, 0, "no ground, no removal");
}

/// `$58DD1` state 1's terminal step: with ground under the object the
/// `$3B40` probe keeps bit 3 and the `$72D7` removal runs — the staging
/// zeroes, the active count drops, `sObjectDone = 2` and (remove-mode 0)
/// a stream-backed slot clears its record marker.
#[test]
fn carrier46_grounded_step_end_removes_itself_and_clears_the_marker() {
    let t = tables();
    let mut obj = ObjectSim::new(records(), 0);
    obj.active = 1;
    obj.spawned[0] = true;
    let layout = one_tile(6, 25, 0x00); // ground under (96, 400)
    let mut st = Staging::default();
    set_x(&mut st.b, 96);
    set_y(&mut st.b, 400); // Y & $0C == 0: the probe is taken
    st.b[B_STATE] = 2;
    st.b[B_DEF] = 46;
    st.b[B_FSTATE] = 1;
    st.b[B_T28] = 0; // the countdown is already spent
    st.b[B_FLAGS] = 0x08;
    st.b[B_RECHI] = 0xAB; // record 0 of the stream buffer
    st.b[B_RECLO] = 0x06;
    obj.handler_carrier46(&mut st, &layout, &t);
    assert_eq!(obj.done, 2, "$72D7 sets sObjectDone = 2");
    assert_eq!(st.b, [0u8; 32], "the removal zeroes the staging");
    assert_eq!(obj.active, 0);
    assert!(!obj.spawned[0], "remove-mode 0 clears the record marker");
    assert_eq!(obj.remove_mode, 0, "$5B2D7 resets the mode");
}

/// `$3CDF`/`$3CD7`: the carry adds `b` to Mario's **16-bit** Y — the high
/// byte takes the `adc` — gated on `+23` bit 3; a Y already staged this
/// frame is the base, and with the bit clear the byte is untouched.
#[test]
fn carry_adds_b_to_marios_16bit_y_only_when_stood_on() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.mario.y = 0x20FF;
    let mut st = Staging::default();
    st.b[B_FLAGS] = 0x08;
    obj.carry_mario(&mut st, 3);
    assert_eq!(obj.mario_y_adjust, Some(0x2102), "carry into the high byte");
    obj.carry_mario(&mut st, 2);
    assert_eq!(obj.mario_y_adjust, Some(0x2104), "stacks on the staged Y");
    st.b[B_FLAGS] = 0x00;
    obj.mario_y_adjust = None;
    obj.carry_mario(&mut st, 9);
    assert_eq!(obj.mario_y_adjust, None, "bit 3 clear: nothing");
}

// -----------------------------------------------------------------
// $2FED / $3007 — the fireball-vs-object pass
// -----------------------------------------------------------------

/// Every slot boxed around (0x50, 0x50) with a 0x10 hitbox: `$3007`'s
/// expansion is `top - 6`, `bottom + 6` (and the same on X), so the window
/// is `(sy, sx)` in `0x4A..0x66`.
fn fire_slot(obj: &mut ObjectSim, i: usize, ty: u8, flags: u8) {
    let s = &mut obj.slots[i];
    s.b[B_TYPE] = ty;
    s.b[B_FLAGS] = flags;
    s.b[B_SCRY] = 0x50;
    s.b[B_SCRX] = 0x50;
    s.b[B_HB_TOP] = 0;
    s.b[B_HB_BOTTOM] = 0x10;
    s.b[B_HB_LEFT] = 0;
    s.b[B_HB_RIGHT] = 0x10;
}

/// `$2FED`/`$3007`: a bit-2 type is *defeated* (bit 1 of `+17`, the flag
/// `$85AA`'s dispatch turns into `defeat()`), everything else just takes
/// the sfx arm — and both leave `sObjectHitResult` non-zero, which is what
/// kills the ball. The window test is byte-wise: the far edge misses.
#[test]
fn fire_hit_defeats_bit2_types_and_dies_on_the_rest() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    fire_slot(&mut obj, 0, 0x24, 0); // bit 2 set: fireball-defeatable
    assert_eq!(obj.fire_hit(0x4A, 0x4A, (0, 0)), FireHit::Defeat);
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x02);
    obj.slots[0].b[B_FLAGS] = 0;
    assert_eq!(
        obj.fire_hit(0x4A, 0x66, (0, 0)),
        FireHit::None,
        "one past the right edge"
    );
    assert_eq!(
        obj.fire_hit(0x66, 0x4A, (0, 0)),
        FireHit::None,
        "one past the bottom edge"
    );
    assert_eq!(
        obj.fire_hit(0x49, 0x4A, (0, 0)),
        FireHit::None,
        "one short of the top"
    );
    assert_eq!(obj.slots[0].b[B_FLAGS], 0, "a miss writes nothing");

    fire_slot(&mut obj, 0, 0x20, 0); // bit 2 clear
    assert_eq!(obj.fire_hit(0x4A, 0x4A, (0, 0)), FireHit::Die);
    assert_eq!(
        obj.slots[0].b[B_FLAGS], 0,
        "no defeat for a bit-2-clear type"
    );
}

/// The scan's own gates: the flags byte must be exactly 0 — bit 3
/// included, `$2FED` has no `$2D7D`-style `and $F7` — and a bit-7 type is
/// an item pose, transparent to a ball.
#[test]
fn fire_hit_skips_flagged_slots_items_and_empty_slots() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    fire_slot(&mut obj, 0, 0x24, 0x08); // stood on: not hittable
    fire_slot(&mut obj, 1, 0x82, 0x00); // the item pose
    fire_slot(&mut obj, 2, 0x00, 0x00); // empty
    fire_slot(&mut obj, 3, 0x80, 0x00); // the base item type
    assert_eq!(obj.fire_hit(0x4A, 0x4A, (0, 0)), FireHit::None);
    for (i, want) in [(0usize, 0x08u8), (1, 0), (2, 0), (3, 0)] {
        assert_eq!(obj.slots[i].b[B_FLAGS], want, "slot {i}");
    }
}

/// The scan keeps walking after a hit, exactly like `$2FED`'s loop, so the
/// *last* hit is the verdict `sObjectHitResult` holds — and an earlier
/// defeat is not undone by it.
#[test]
fn fire_hit_keeps_the_last_verdict_and_still_defeats() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    fire_slot(&mut obj, 0, 0x24, 0); // defeat
    fire_slot(&mut obj, 1, 0x20, 0); // die
    assert_eq!(obj.fire_hit(0x4A, 0x4A, (0, 0)), FireHit::Die, "the last hit wins");
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x02, "the earlier defeat stands");
}

// ---- Bank 2 $8DFD / $8E6C boss spawn staging unit tests (U1) -----------

#[test]
fn boss_spawn_clears_pages_and_populates_metadata() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.boss_slots_raw.fill(0xAA);
    obj.slots[0].b.fill(0xFF);
    obj.spawn_boss(0);

    assert_eq!(obj.boss_hits_remaining, 6);
    assert_eq!(obj.boss_sprite, 1);
    assert_eq!(obj.boss_timer_reload, 80);
    assert_eq!(obj.boss_slots_raw[0..5], [0, 0, 0, 0, 0]);
    let (_, _, _, obj0, _) = crate::scene::lookup_boss_spawn_def(0);
    assert_eq!(&obj.boss_slots_raw[5..23], &obj0);
    assert_eq!(&obj.slots[0].b[5..23], &obj0);
}

#[test]
fn boss_spawn_count_1_stages_one_sub_without_spill() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.spawn_boss(0); // def 0: count 1

    let (_, _, _, _, subs) = crate::scene::lookup_boss_spawn_def(0);
    assert_eq!(&obj.boss_slots_raw[0x25..0x25 + 18], &subs[0]);
    assert_eq!(obj.boss_slots_raw[0x25 + 18], 0);
    assert_eq!(obj.slots[2].b, [0; 32]);
}

/// `$8E6C`'s staging for a count-2 def. The stride is `((e + 16) & $F0) + 5`
/// on the cursor *after* its own 18-byte copy, so the images land at
/// `$AD05`, `$AD25`, `$AD45` — +$20 each, slots 0/1/2. (Console bytes for
/// level `$0D`'s def 2: `ad25: .. 0B .. 60 88 1E`, `ad45: .. 0A .. 60 28
/// 1E`, i.e. the second sub-image really does occupy slot 2. Reading the
/// arithmetic without the post-copy `E` gave +$10 and left slot 2 inert,
/// which cost the arena one of its houses.)
#[test]
fn boss_spawn_def_index_4_fills_slots_0_1_and_2_at_a_20_stride() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.spawn_boss(4); // def 4: count 2

    let (count, _, _, obj0, subs) = crate::scene::lookup_boss_spawn_def(4);
    assert_eq!(count, 2);
    assert_eq!(&obj.boss_slots_raw[5..5 + 18], &obj0[..]);
    assert_eq!(&obj.boss_slots_raw[0x25..0x25 + 18], &subs[0][..]);
    assert_eq!(
        &obj.boss_slots_raw[0x45..0x45 + 18],
        &subs[1][..],
        "the $8E6C step is +$20, not +$10"
    );
    assert_eq!(&obj.slots[0].b[5..23], &obj0[..]);
    assert_eq!(&obj.slots[1].b[5..23], &subs[0][..]);
    assert_eq!(&obj.slots[2].b[5..23], &subs[1][..]);
    assert_eq!(obj.slots[2].b[5], 18, "the second sub-image is live");
    assert_eq!(obj.slots[3].b, [0; 32]);
}

#[test]
fn boss_spawn_index_8_stages_no_subs_but_keeps_obj0() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.spawn_boss(8); // count 0

    let (count, _, _, obj0, _) = crate::scene::lookup_boss_spawn_def(8);
    assert_eq!(count, 0);
    assert_eq!(&obj.slots[0].b[5..23], &obj0);
    assert_eq!(obj.slots[1].b, [0; 32]);
    assert_eq!(obj.slots[2].b, [0; 32]);
}

// ---- $8611 / $5839B / $58444 — the carry, the kick and the KO ---------

/// `$8611` with B held: the one-shot `sActionLatch`, anim += 2 (the stunned
/// shell's 2 -> the carried 4) and `$863C`, whose `$8240` clears `+14..+30`
/// — the type byte with them, which is what leaves the carried object
/// outside the KO scan. A second contact cannot re-trigger it.
#[test]
fn the_8611_pickup_takes_the_shell_to_the_carried_anim() {
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = streamed(2, 0x06, 0x10);
    st.b[B_FSTATE] = 1;
    let contact = MarioContact {
        b_held: true,
        ..Default::default()
    };
    obj.post_touch(&mut st, &[0u8; 48], &t, &contact);
    assert_eq!(st.b[B_DEF], 4, "anim 2 + 2 = the carried state");
    assert_eq!(obj.action_latch, 1, "the latch arms");
    assert_eq!(st.b[B_TYPE], 0, "$8240 cleared +14..+30, type included");
    assert_eq!(st.b[B_FSTATE], 0, "and the state byte");
    assert_eq!(obj.done, 1, "$863C sets sObjectDone");
    assert!(obj.unported.is_empty(), "$8611 is ported now");

    // The latch holds: a second B-held contact does nothing.
    let mut st2 = streamed(4, 0x00, 0x10);
    obj.post_touch(&mut st2, &[0u8; 48], &t, &contact);
    assert_eq!(st2.b[B_DEF], 4, "no second bump");
}

/// `$8611` with B *not* held: `$8633` bumps the anim to 3 (the kicked
/// walker) and the `$864A` tail nudges the shell 16 px to Mario's facing
/// side — open air there means it just walks off, no type change.
#[test]
fn the_8611_kick_places_the_shell_on_marios_facing_side() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.active = 1;
    obj.spawned = vec![true];
    let mario = MarioContact {
        x: 100,
        y: 100,
        dir_right: true,
        ..Default::default()
    };
    obj.mario = mario;
    obj.mario_ref_x = 108;
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_DEF] = 2;
        s.b[B_TYPE] = 0x06;
        s.b[B_FLAGS] = 0x10;
        set_x(&mut s.b, 100);
        set_y(&mut s.b, 100);
    }
    // Through the engine, so the $864A tail runs with the layout.
    obj.engine(0, 0, &layout, &t);
    let s = &obj.slots[0];
    assert_eq!(s.b[B_DEF], 3, "$8633: anim += 1");
    assert_eq!(s.b[B_TYPE], 0x06, "no wall: the type stays");
    assert_eq!(s.x(), 116, "nudged 16 px to Mario's right");
}

/// The same tail with a wall on Mario's facing side: the shell is put back
/// and becomes a *live* kicked shell — type `$5F`, walking into the wall
/// (state 1), which the anim-3 handler turns around next frame. The probe
/// runs from the *nudged* position, i.e. (x + 16 + 7, y - 8).
#[test]
fn a_wall_on_marios_facing_side_makes_a_live_kicked_shell() {
    let mut layout = blank_layout();
    layout.cells[5 * 256 + 7] = 0x20; // the nudged (x+7, y-8) probe's cell
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.active = 1;
    obj.spawned = vec![true];
    obj.mario = MarioContact {
        x: 100,
        y: 100,
        dir_right: true,
        ..Default::default()
    };
    obj.mario_ref_x = 108;
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_DEF] = 2;
        s.b[B_TYPE] = 0x06;
        s.b[B_FLAGS] = 0x10;
        set_x(&mut s.b, 100);
        set_y(&mut s.b, 100);
    }
    obj.engine(0, 0, &layout, &t);
    let s = &obj.slots[0];
    assert_eq!(s.b[B_DEF], 3);
    assert_eq!(s.b[B_TYPE], 0x5F, "a live kicked shell");
    assert_eq!(s.b[B_FSTATE], 1, "walking into the wall");
    assert_eq!(s.x(), 100, "put back where it was");
}

/// `$5839B` (anim 3): the kicked shell walks 3 px a frame *away* from Mario
/// (`$5B3C6` puts Mario-to-the-right at state 2 = the left walk), turns at a
/// wall, steps off a ledge into the `$5B451` fall and lands back into the
/// walk with the fall index cleared and Y snapped to the tile.
#[test]
fn the_kicked_shell_walks_turns_falls_and_lands() {
    let mut layout = blank_layout();
    layout.cells[6 * 256..=6 * 256 + 20].fill(0x20); // ground under it
    let mut st = streamed(3, 0x00, 0);
    st.b[B_FSTATE] = 0;
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 96); // (y & $0C) == 0: the ground probe is live
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario = MarioContact {
        x: 300,
        ..Default::default()
    };
    obj.mario_ref_x = 308; // Mario to the right
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.b[B_TYPE], 0x5F, "state 0 arms the shell type");
    assert_eq!(st.b[B_FSTATE], 2, "$5B3C6: Mario right -> the left walk");

    // State 2 walks left 3 px while the ground probe holds.
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.x(), 97, "3 px left, away from Mario");
    assert_eq!(st.b[B_FSTATE], 2);

    // A left wall turns it (state 2 -> 1).
    layout.cells[5 * 256 + 5] = 0x20; // (x-7=90, y-8=88) -> row 5 col 5
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.b[B_FSTATE], 1, "turned at the wall");
    assert_eq!(st.x(), 97, "and it did not step into it");
    assert_eq!(obj.sfx0, 2, "$583F7: the bump sound");

    // State 1 walks right (away from the wall) before the ledge.
    layout.cells[5 * 256 + 5] = 0x60;
    let x_wall = st.x();
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.x(), x_wall + 3, "state 1 walks right");

    // Walking right with no ground under it -> the fall state (+2 = 3).
    layout.cells[6 * 256..=6 * 256 + 20].fill(0x60);
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.b[B_FSTATE], 3, "no ground: state += 2");

    // Falling: Y += the $5B451 curve + 1, and the index steps.
    let y0 = st.y();
    obj.handler_kicked(&mut st, &layout);
    assert!(st.y() > y0, "it falls");
    assert_eq!(st.b[B_PHASE], 1, "the fall index stepped");

    // Land it: ground back, state -= 2 into the walk.
    layout.cells[6 * 256..=6 * 256 + 20].fill(0x20);
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.b[B_FSTATE], 1, "landed back into the right walk");
    assert_eq!(st.b[B_PHASE], 0, "the fall index clears");
    assert_eq!(st.b[B_YLO] & 0x0F, 0, "and Y snaps to the tile");
}

fn layout_for_ko() -> LevelLayout {
    blank_layout()
}

/// `$5B697`: the kicked shell defeats an enemy it overlaps — the target's
/// `+17` bit 1 (the flag the interaction dispatch turns into `defeat()`)
/// plus `sObjectHitFlag`. Type `$5F` (itself, another shell), bit 7 item
/// poses and bit 5 types are all skipped, and the box is the target's own
/// hitbox widened by 3/10 on Y and 5 on X.
#[test]
fn the_kicked_shell_knocks_out_another_enemy() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = streamed(3, 0x5F, 0);
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 100);
    st.b[B_SCRY] = 96;
    st.b[B_SCRX] = 96;
    for (i, ty) in [(0usize, 0x4Eu8), (1, 0x5F), (2, 0x82), (3, 0x0A)] {
        let s = &mut obj.slots[i];
        s.b[B_STATE] = 2;
        s.b[B_TYPE] = ty;
        s.b[B_SCRY] = 90;
        s.b[B_SCRX] = 96;
        s.b[B_HB_TOP] = 0;
        s.b[B_HB_BOTTOM] = 0x10;
        s.b[B_HB_LEFT] = 0;
        s.b[B_HB_RIGHT] = 0x10;
    }
    // Through the handler, so the `bit 6` gate is covered too.
    obj.handler_kicked(&mut st, &layout_for_ko());
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x02, "the walker is defeated");
    assert_eq!(obj.slots[1].b[B_FLAGS], 0, "another shell is not");
    assert_eq!(obj.slots[2].b[B_FLAGS], 0, "an item pose is not");
    assert_eq!(obj.slots[3].b[B_FLAGS], 0x02, "a bit-5/7-clear enemy is");
    assert_eq!(obj.object_hit_flag, 1, "and the hit flag rises");

    // With bit 6 clear (a `$06` shell that never went through state 0) the
    // KO scan does not even run.
    for i in 0..4 {
        obj.slots[i].b[B_FLAGS] = 0;
    }
    st.b[B_TYPE] = 0x06;
    obj.handler_kicked(&mut st, &layout_for_ko());
    assert_eq!(obj.slots[0].b[B_FLAGS], 0, "bit 6 clear: no KO");
    st.b[B_TYPE] = 0x5F;

    // Far enough out on X (the box is ~26 px wide): nothing.
    obj.object_hit_flag = 0;
    obj.slots[0].b[B_FLAGS] = 0;
    obj.slots[3].b[B_FLAGS] = 0;
    for i in 0..4 {
        obj.slots[i].b[B_SCRX] = 96u8.wrapping_add(0x30);
    }
    obj.ko_overlap(&st);
    assert_eq!(obj.slots[0].b[B_FLAGS], 0, "out of the window");
    assert_eq!(obj.object_hit_flag, 0);
}

/// `$581B1` (anim 0): the death animation a defeat runs into — the enemy
/// hops 20 px up and flips upside down, rises on the `$74AB` curve at 2 b a
/// frame, then falls on the `$5B451` curve and is removed once its staged
/// screen Y passes `$A0`. The engine now routes anim 0 here, so a defeat
/// raises no `unported` note either.
#[test]
fn a_defeated_enemy_flips_up_and_falls_off_the_screen() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_defeated = 2;
    let mut st = streamed(0, 0x00, 0);
    st.b[B_FSTATE] = 0;
    st.b[B_SPRITE] = 3;
    st.b[B_TYPE] = 0x00;
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 100);
    obj.handler_death(&mut st, &layout, &t);
    assert_eq!(st.y(), 80, "the 20 px hop (Y -= 20)");
    assert_eq!(st.b[B_FLIP], 0x40, "flipped upside down");
    assert_eq!(st.b[B_FSTATE], 1, "and rising");
    assert_eq!(st.b[B_TYPE], 0);

    // $581CE: 2x the $74AB curve, one byte a frame.
    let y0 = st.y();
    obj.handler_death(&mut st, &layout, &t);
    assert_eq!(st.y(), y0 - 4, "$74AB[0] = 2, doubled by the sla");
    for _ in 0..7 {
        obj.handler_death(&mut st, &layout, &t);
    }
    assert_eq!(st.b[B_FSTATE], 2, "the curve's sentinel ends the rise");
    assert_eq!(st.b[B_PHASE], 0, "and resets the fall index");

    // $581E2: it falls until the draw's staged screen Y passes $A0.
    let mut frames = 0;
    while st.b[B_STATE] != 0 && frames < 80 {
        st.b[B_SCRY] = st.b[B_SCRY].wrapping_add(6); // the draw step
        obj.handler_death(&mut st, &layout, &t);
        frames += 1;
    }
    assert_eq!(st.b[B_STATE], 0, "removed once off the bottom");
    assert_eq!(obj.remove_mode, 0, "$5B2D7 resets the mode");
    assert_eq!(obj.boss_defeated, 1, "and counts sBossDefeated down once");
}

/// `$8DA9`: the boss driver walks `sObjectDefCount + 1` slots from `$AD00`
/// at a $20 stride, and each slot runs `$8DBE`'s body only when its staged
/// anim byte (`$FFD5`) is non-zero. `spawn_boss(4)` (count 2) therefore
/// touches slots 0-2 and leaves slot 3 alone; `$FFF0` is bumped
/// unconditionally, unlike `$8000`'s phase-toggle increment.
#[test]
fn the_boss_driver_walks_count_plus_one_slots_at_a_20_stride() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.spawn_boss(4);
    assert_eq!(obj.boss_def_count, 2);
    assert_eq!(obj.boss_def_index, 4);
    // def 4's images: anim 16 at $AD05 (slot 0's byte 5), 17 at $AD25 (slot
    // 1), 18 at $AD45 (slot 2) — the +$20 `$8E6C` stride, so all three are
    // live. (Measured on the console for level `$0D`'s def 2; slot 2 being
    // zero was the port's reading error, not the ROM's behavior.)
    assert_eq!(obj.slots[0].b[B_DEF], 16);
    assert_eq!(obj.slots[1].b[B_DEF], 17);
    assert_eq!(obj.slots[2].b[B_DEF], 18, "the second sub-image is live");
    assert_eq!(obj.slots[3].b, [0u8; 32], "slot 3 stays untouched");
    // Plant markers: an active slot's flags, the last slot the walk must
    // reach (count + 1 = 3), and one beyond it.
    obj.slots[1].b[B_FLAGS] = 0xAA;
    obj.slots[2].b[B_DEF] = 0x2A;
    obj.slots[2].b[B_FLAGS] = 0xAA;
    obj.slots[3].b[B_DEF] = 0x2A;
    obj.slots[3].b[B_FLAGS] = 0xAA;
    let fff0 = obj.fff0;
    obj.boss_engine(0, 0, &layout, &t);
    assert_eq!(obj.fff0, fff0.wrapping_add(1), "$FFF0 bumps every frame");
    assert_eq!(obj.slots[1].b[B_FLAGS], 0, "$FFE7 = 0 clears the flags");
    assert_eq!(
        obj.slots[2].b[B_FLAGS], 0,
        "count + 1 slots are walked, so slot 2 is staged"
    );
    assert_eq!(obj.slots[3].b[B_FLAGS], 0xAA, "slot 3 is never staged");
    assert_eq!(obj.slots[3].b[B_DEF], 0x2A, "and keeps its anim byte");
    // `$3A14` is ported now, so the pass runs: the synthetic table's lists
    // are empty, so it draws nothing and — importantly — reports nothing.
    assert!(obj.boss_oam.is_empty(), "no list, no OAM");
    assert!(!obj.unported.contains(&0x3A14), "$3A14 is no longer a gap");
    assert!(!obj.unported.contains(&0x66092));
}

/// `$92DF`: `ld a,[sMarioDead] / cp $FF / ret z` — while Mario is dead the
/// handler is skipped for **every** slot, so the records hold their last
/// values. Console-verified on the level 13 tape: all three slot records
/// stop changing on the frame Mario dies (f462) and stay put through the
/// death animation, while the frozen houses keep rendering. The driver's
/// remaining body still runs (render, `$FFE7 = 0`, `$9048`, `$81BD`), so the
/// slot is *written back* unchanged rather than blanked.
#[test]
fn a_dead_mario_freezes_every_boss_slot() {
    let mut t = tables();
    t.boss_anim_handlers = vec![0x9322, 0x93A7, 0x9606, 0x9323];
    t.boss_picker_profile = vec![0x60; 24];
    t.boss_picker_profile_15 = vec![1; 18];
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 1;
    // Slot 0 is mid-way through the anim-3 exit: one frame away from the
    // `SCRY >= $D0` hand-off, so without the guard it moves this frame.
    obj.slots[0].b[B_DEF] = 3;
    obj.slots[0].b[B_SCRY] = 0xC0;
    obj.slots[0].b[B_FSTATE] = 2;
    // Slot 1 is a live part the walk would advance too.
    obj.slots[1].b[B_DEF] = 10;
    obj.slots[1].b[B_SCRY] = 0x40;
    obj.slots[1].b[B_FSTATE] = 3;
    let before0 = obj.slots[0].b;
    let before1 = obj.slots[1].b;

    obj.mario.dead = 0xFF;
    obj.boss_engine(0, 0, &layout, &t);
    assert_eq!(obj.slots[0].b, before0, "a dead Mario freezes slot 0");
    assert_eq!(obj.slots[1].b, before1, "and slot 1");
    assert!(
        !obj.unported.contains(&0x92DF),
        "the guard itself is ported"
    );
    assert_eq!(obj.boss_def_count, 1, "nothing left the room either");

    // Alive again: the very same frame's work now happens, which is what
    // makes the guard load-bearing rather than a no-op assertion. (Only slot
    // 0 is asserted here: slot 1's part needs its def's HP and hitbox to
    // reach the walk, and that live path is covered by the part tests.)
    obj.mario.dead = 0;
    obj.boss_engine(0, 0, &layout, &t);
    assert_ne!(obj.slots[0].b, before0, "with Mario alive slot 0 moves");
}

/// `$9EE3` (boss anim 13): the child the def-2 controller spawns when the
/// parts are gone. It is a gate on `sObjectRemoveMode == 2` — inert with the
/// value the controller leaves (0), converting to anim 4 at `SCRY = $90` /
/// `SCRX = $58` when it is 2. The console numbers come from poking
/// `sObjectRemoveMode = 2` on the arena tape: `anim=0D fst=00 y=40 x=58
/// spr=0E` becomes `anim=04 fst=00 y=90 x=58 spr=09` on the next frame.
#[test]
fn the_controller_child_gates_on_remove_mode() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = streamed(0, 0x00, 0);
    st.b[B_DEF] = 0x0D;
    st.b[B_SPRITE] = 0x0E;
    st.b[B_SCRY] = 0x40;
    st.b[B_SCRX] = 0x58;
    st.b[B_FSTATE] = 0x03;

    // What the controller leaves behind: nothing happens, not even the moves.
    obj.remove_mode = 0;
    obj.dispatch_handler(0x9EE3, &mut st, &layout, &t);
    assert_eq!(st.b[B_DEF], 0x0D, "still the gate anim");
    assert_eq!(st.b[B_SCRY], 0x40, "and still where the spawn put it");
    assert!(obj.unported.is_empty(), "anim $0D is ported now");

    // The teardown value: SCRY/SCRX move and the slot becomes anim 4.
    obj.remove_mode = 2;
    obj.dispatch_handler(0x9EE3, &mut st, &layout, &t);
    assert_eq!(st.b[B_SCRY], 0x90);
    assert_eq!(st.b[B_SCRX], 0x58);
    assert_eq!(st.b[B_DEF], 0x04, "handed to the anim-4 chain");
    assert_eq!(st.b[B_SPRITE], 0x09);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(st.b[B_PHASE], 0);
    assert_eq!(st.b[B_TYPE], 0);
}

/// `$66000`: the render pass walks the record into OAM with the slot's own
/// `$FFD9`/`$FFDA` as the base position — no world-coordinate math, because
/// in the boss path those bytes ARE the object's screen position.
#[test]
fn the_boss_render_emits_oam_from_the_record() {
    let t = boss_oam_tables(vec![
        0x08, 0x08, 0x11, 0x01, // plain cell: y $40+8, x $58+8, its attr
        0x00, 0xF8, 0x22, 0x00, // dx -8 (the offsets are signed)
        0x80,
    ]);
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_render(&boss_staged(), &t);
    assert_eq!(
        obj.boss_oam,
        vec![
            BossOamCell {
                y: 0x48,
                x: 0x60,
                tile: 0x11,
                attr: 0x01
            },
            BossOamCell {
                y: 0x40,
                x: 0x50,
                tile: 0x22,
                attr: 0x00
            },
        ]
    );
    assert_eq!(obj.boss_oam.len(), 2, "the $80 terminator ends the walk");
    assert_eq!(obj.sprite_flip_flags, 0, "$6087 recomputed the flags byte");
}

/// The `$6087` flags byte does double duty: bit 5 mirrors the X offset
/// (`~dx - 7`), bit 6 the Y offset (`~dy - 7`), and the whole byte is XORed
/// into every emitted attr — which is how a boss flips without a second
/// sprite sheet.
#[test]
fn the_boss_render_mirrors_offsets_and_xors_the_flags_byte() {
    let t = boss_oam_tables(vec![0x08, 0x08, 0x11, 0x01, 0x80]);
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = boss_staged();
    st.b[B_FLIP] = 0x60; // bit 5 (X) + bit 6 (Y)
    st.b[B_FLASH] = 0x01; // so the byte is an XOR of three slot bytes
    obj.boss_render(&st, &t);
    assert_eq!(obj.sprite_flip_flags, 0x61);
    // ~8 - 7 = $F0 on both axes; the attrs carry the flags as well.
    assert_eq!(
        obj.boss_oam,
        vec![BossOamCell {
            y: 0x40u8.wrapping_add(0xF0),
            x: 0x58u8.wrapping_add(0xF0),
            tile: 0x11,
            attr: 0x01 ^ 0x61,
        }]
    );
}

/// `$88 <adj>` is the big-blob origin marker: one operand, and it shifts the
/// base Y for the cells that follow **only** while flip bit 6 is set (the
/// flipped blobs' offsets are measured from the flipped origin).
#[test]
fn the_boss_render_shifts_the_origin_only_when_y_flipped() {
    let t = boss_oam_tables(vec![
        0x88, 0xF0, // origin: -16 to the base Y when Y-flipped
        0x00, 0x00, 0x44, 0x00, // one cell at the origin
        0x80,
    ]);
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_render(&boss_staged(), &t);
    assert_eq!(
        obj.boss_oam[0].y, 0x40,
        "no Y flip: the operand is consumed and ignored"
    );

    obj.boss_oam.clear();
    let mut st = boss_staged();
    st.b[B_FLIP] = 0x40;
    obj.boss_render(&st, &t);
    // base Y $40 - $10 = $30, and the cell's own dy 0 flips to ~0-7 = $F8.
    assert_eq!(obj.boss_oam[0].y, 0x30u8.wrapping_add(0xF8));
    assert_eq!(obj.boss_oam[0].x, 0x58, "the marker shifts Y only");
}

/// The console's own OAM bounds: a Y at or past `$A0` drops the whole cell
/// (nothing is written), and an X at or past `$A8` writes a `0` over the Y
/// byte — the port emits that entry hidden, in the same slot order.
#[test]
fn the_boss_render_applies_the_oam_bounds() {
    let t = boss_oam_tables(vec![
        0x60, 0x00, 0x11, 0x00, // y = $40 + $60 = $A0 -> dropped entirely
        0x00, 0x50, 0x22, 0x00, // x = $58 + $50 = $A8 -> hidden (y 0)
        0x00, 0x00, 0x33, 0x00, // kept
        0x80,
    ]);
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_render(&boss_staged(), &t);
    assert_eq!(
        obj.boss_oam,
        vec![
            BossOamCell {
                y: 0,
                x: 0xA8,
                tile: 0x22,
                attr: 0x00
            },
            BossOamCell {
                y: 0x40,
                x: 0x58,
                tile: 0x33,
                attr: 0x00
            },
        ],
        "the tall cell is gone, the wide one is hidden, the third survives"
    );
}

/// `$66000`'s two early outs and its one report: `sObjectDone` (a slot that
/// took a hit this frame is not drawn), the `$80` terminator, and a sprite
/// id past the 118-entry table — where the ROM would read the following
/// record bytes as a pointer, so the port reports instead of serving them.
#[test]
fn the_boss_render_stops_on_done_the_terminator_and_a_missing_list() {
    let t = boss_oam_tables(vec![
        0x00, 0x00, 0x11, 0x00, 0x80, // one cell, then END
        0x00, 0x00, 0x99, 0x00, // never read
    ]);
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.done = 1;
    obj.boss_render(&boss_staged(), &t);
    assert!(
        obj.boss_oam.is_empty(),
        "$66000 returns while sObjectDone is set"
    );

    obj.done = 0;
    obj.boss_render(&boss_staged(), &t);
    assert_eq!(obj.boss_oam.len(), 1, "the $80 terminator stops the walk");
    assert_eq!(obj.boss_oam[0].tile, 0x11);

    obj.boss_oam.clear();
    let mut st = boss_staged();
    st.b[B_SPRITE] = 200; // past the table
    obj.boss_render(&st, &t);
    assert!(obj.boss_oam.is_empty());
    assert!(
        obj.unported.contains(&0x66092),
        "a sprite id the table cannot serve is reported"
    );
}

/// Every slot image the nine boss defs install carries a sprite id inside
/// the `$6092` table (118 entries), so no retail arena can report `$66092`
/// instead of drawing. The `sprite` field of a def record is
/// `sBossSprite`, not a slot byte — the slot's own id is image byte 6
/// (`B_SPRITE`, since the images land at `$AD05` = slot byte 5).
#[test]
fn every_boss_def_slot_sprite_is_inside_the_boss_oam_table() {
    for def in 0..=8u8 {
        let (_, _, _, obj0, subs) = crate::scene::lookup_boss_spawn_def(def);
        for (i, img) in std::iter::once(&obj0).chain(subs.iter()).enumerate() {
            let sprite = img[6];
            assert!(
                (sprite as usize) < 118,
                "def {def} image {i}: sprite {sprite} is past the $6092 table"
            );
        }
    }
}

/// The driver's own gate: a slot whose anim byte is 0 is skipped entirely
/// (no handler, no `$9048` extent, no copy-back), which is how the def
/// images that straddle slot pages come out inert.
#[test]
fn the_boss_driver_skips_slots_with_a_zero_anim_byte() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.spawn_boss(4);
    obj.slots[1].b[B_DEF] = 0; // as the +$10 stride can leave it
    obj.slots[1].b[B_FLAGS] = 0xAA;
    obj.slots[1].b[B_SPRITE] = 7;
    obj.boss_engine(0, 0, &layout, &t);
    assert_eq!(
        obj.slots[1].b[B_FLAGS], 0xAA,
        "not even the flag clear runs"
    );
    assert_eq!(obj.slots[1].b[B_SPRITE], 7, "and the sprite is untouched");
}

/// `$9048` uses the scene-state table `$505B` (`draw_extent_for`), not the
/// level's `$4737`/`$4935` (`hitbox_for`) — the two disagree, and the boss
/// slots must get the former.
#[test]
fn the_boss_driver_fetches_the_505b_extent_record() {
    // The synthetic `tables()` in this module has no `$505B` records, so
    // this one needs the committed extraction.
    let t: ObjectTables = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.spawn_boss(4);
    // A sprite id present in both tables with different records.
    let sprite = (0u8..=255)
        .find(|s| {
            t.draw_extent_for(*s).is_some()
                && t.hitbox_for(0, *s).is_some()
                && t.draw_extent_for(*s) != t.hitbox_for(0, *s)
        })
        .expect("a sprite the two tables disagree on");
    let want: [u8; 4] = t.draw_extent_for(sprite).unwrap().map(|b| b as u8);
    obj.slots[1].b[B_SPRITE] = sprite;
    obj.slots[1].b[B_HB_TOP] = 0xEE;
    obj.boss_engine(0, 0, &layout, &t);
    assert_eq!(
        [
            obj.slots[1].b[B_HB_TOP] as u8,
            obj.slots[1].b[B_HB_BOTTOM],
            obj.slots[1].b[B_HB_LEFT],
            obj.slots[1].b[B_HB_RIGHT],
        ],
        want,
        "the $505B record, not the level's"
    );
}

/// `$9285`: the hit pass books hp/sfx/latch on the *primary* slot only,
/// treats 0 or a borrow as the kill, and arms removal plus the clear fade.
#[test]
fn the_boss_damage_pass_books_hp_sfx_and_the_kill() {
    let layout = blank_layout();
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 4;
    obj.boss_hits_remaining = 6;
    let mut st = streamed(0, 0x00, 0);
    // No flags: nothing at all.
    obj.boss_slot_ptr_lo = 0;
    obj.boss_damage_pass(&mut st);
    assert_eq!(obj.boss_hits_remaining, 6);
    assert_eq!(obj.done, 0);
    // A stomp on a *secondary* slot: done, but no hp/sfx.
    st.b[B_FLAGS] = 0x01;
    obj.boss_slot_ptr_lo = 0x20;
    obj.boss_damage_pass(&mut st);
    assert_eq!(obj.done, 1);
    assert_eq!(obj.boss_hits_remaining, 6, "only slot 0 books hp");
    assert_eq!(obj.sfx0, 0);
    // The primary slot: sfx 19, −2, latch.
    obj.boss_slot_ptr_lo = 0;
    obj.boss_damage_pass(&mut st);
    assert_eq!(obj.boss_hit_latch, 1);
    assert_eq!(obj.sfx0, 0x13);
    assert_eq!(obj.boss_hits_remaining, 4);
    // A projectile hit: sfx 9, −1.
    st.b[B_FLAGS] = 0x02;
    obj.boss_damage_pass(&mut st);
    assert_eq!(obj.sfx1, 9);
    assert_eq!(obj.boss_hits_remaining, 3);
    obj.boss_damage_pass(&mut st);
    assert_eq!(obj.boss_hits_remaining, 2);
    assert_eq!(obj.boss_clear_flag, 0, "not dead yet");
    // Two more: 0 -> the kill path.
    obj.boss_damage_pass(&mut st);
    obj.boss_damage_pass(&mut st);
    assert_eq!(obj.boss_hits_remaining, 0);
    assert_eq!(obj.remove_mode, 1, "$92C4");
    assert_eq!(obj.boss_clear_flag, 1);
    assert_eq!(obj.fade_timer_set, Some(0x0280), "2/$80");
    let _ = (&layout, &t);
}

/// `$B157`/`$B15B` are the same blank with one difference: the count. The
/// primary (last) slot leaves through `$B15B` and must *not* decrement, or
/// the controller would wait for a live count that never matches.
#[test]
fn the_boss_finish_helpers_split_counted_from_blank() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 2;
    let mut st = streamed(0, 0x00, 0);
    st.b[B_DEF] = 7;
    st.b[B_SCRY] = 0x40;
    obj.boss_finish_counted(&mut st); // $B157
    assert_eq!(obj.boss_def_count, 1);
    assert_eq!(st.b, [0u8; 32]);
    assert_eq!(obj.done, 1);

    let mut st = streamed(0, 0x00, 0);
    st.b[B_DEF] = 7;
    obj.done = 0;
    obj.boss_finish_blank(&mut st); // $B15B
    assert_eq!(
        obj.boss_def_count, 1,
        "the blank half leaves the count alone"
    );
    assert_eq!(st.b, [0u8; 32]);
    assert_eq!(obj.done, 1);
}

/// `$9323` (boss anim 3), the body the `$9443` morph hands a defeated part
/// to. A non-primary slot rides the two pickers up and back down, then leaves
/// through `$B157`: the live count drops and the staging is blanked. Without
/// this body the slot froze the moment its `B_DEF` became 3 — reported as
/// `$9323` — and the room kept a house that should have fallen away.
#[test]
fn a_defeated_part_rides_anim3_out_and_drops_the_count() {
    let mut t = tables();
    // anim 3 -> $9323, so the driver's dispatch reaches the new body.
    t.boss_anim_handlers = vec![0x9322, 0x93A7, 0x9606, 0x9323];
    t.boss_picker_profile = vec![2; 24]; // $71A3, cap 21
    t.boss_picker_profile_15 = vec![3; 18]; // $7214, cap 15
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 2;
    obj.slots[1].b[B_DEF] = 3;
    obj.slots[1].b[B_SCRY] = 0x60;
    obj.slots[1].b[B_SCRX] = 0x40;
    obj.slots[1].b[B_FLIP] = 0x20;
    obj.slots[1].b[B_TYPE] = 0x45;

    obj.boss_engine(0, 0, &layout, &t);
    // Entry: `$B270` with b = 32, the Y flip, type 0, state 1, sfx 23 (this
    // is slot 1, so `sObjectSlotPtrLo` is $20, not the primary 0).
    assert_eq!(obj.slots[1].b[B_SCRY], 0x40, "up 32");
    assert_eq!(obj.slots[1].b[B_FLIP], 0x60, "Y flip toggled");
    assert_eq!(obj.slots[1].b[B_TYPE], 0);
    assert_eq!(obj.slots[1].b[B_FSTATE], 1);
    assert_eq!(obj.sfx0, 23);
    assert_eq!(obj.boss_def_count, 2, "still in the room");

    let mut frames = 1;
    while obj.slots[1].b[B_DEF] != 0 && frames < 400 {
        obj.boss_engine(0, 0, &layout, &t);
        frames += 1;
    }
    assert!(frames < 400, "the part left instead of hanging around");
    assert_eq!(obj.boss_def_count, 1, "$B157 drops the live count");
    assert_eq!(obj.slots[1].b, [0u8; 32], "and blanks the slot");
    assert_eq!(obj.remove_mode, 2);
    assert_eq!(obj.music_request, None, "no fanfare for a non-primary slot");
    assert!(!obj.unported.contains(&0x9323), "anim 3 is ported now");
}

/// The primary slot's arm of `$9326`'s state 2 (`$9399`): it is the last one
/// out, so it does *not* touch the count — instead it plays the 208-frame
/// clear fanfare (`sMusicRequest = 12`) and only then blanks itself through
/// `$B15B`.
#[test]
fn the_primary_boss_slot_plays_the_fanfare_before_it_blanks() {
    let mut t = tables();
    t.boss_anim_handlers = vec![0x9322, 0x93A7, 0x9606, 0x9323];
    t.boss_picker_profile = vec![0x60; 24]; // one big step down
    t.boss_picker_profile_15 = vec![1; 18];
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 0;
    obj.remove_mode = 1;
    obj.slots[0].b[B_DEF] = 3;
    obj.slots[0].b[B_SCRY] = 0xC0;
    obj.boss_engine(0, 0, &layout, &t);
    assert_eq!(obj.slots[0].b[B_FSTATE], 1, "entry first, as always");
    assert_eq!(obj.sfx0, 13, "sfx 13 for the primary slot");

    let mut frames = 1;
    let mut fanfare_seen = false;
    let mut prev_state = obj.slots[0].b[B_FSTATE];
    while obj.slots[0].b[B_DEF] != 0 && frames < 400 {
        obj.boss_engine(0, 0, &layout, &t);
        frames += 1;
        let state = obj.slots[0].b[B_FSTATE];
        if prev_state == 2 && state == 3 {
            // The transition frame itself: the timer is set and the track
            // requested there, and the countdown starts on the frame after.
            fanfare_seen = true;
            assert_eq!(obj.slots[0].b[B_T28], 208, "the fanfare is 208 frames");
            assert_eq!(obj.music_request, Some(12), "and it asks for track 12");
        }
        if state == 3 {
            assert_eq!(obj.remove_mode, 1, "reward waits until the fanfare ends");
        }
        prev_state = state;
    }
    assert!(fanfare_seen, "the fanfare ran");
    assert!(frames < 400, "the primary slot left too");
    assert_eq!(obj.boss_def_count, 0, "$B15B leaves the count alone");
    assert_eq!(obj.slots[0].b, [0u8; 32]);
    assert_eq!(obj.remove_mode, 2, "timer expiry releases the reward");
}

/// `$B264`-`$B276`: eight-bit screen movers (the world-coord
/// `mv_x_add`/`mv_y_add` are different routines), including the wrap.
#[test]
fn the_boss_screen_movers_are_eight_bit() {
    let obj = ObjectSim::new(Vec::new(), 0);
    let mut st = streamed(0, 0x00, 0);
    st.b[B_SCRX] = 0x10;
    st.b[B_SCRY] = 0x20;
    obj.scr_x_add(&mut st, 3);
    obj.scr_x_sub(&mut st, 0x14);
    assert_eq!(st.b[B_SCRX], 0xFF, "0x13 - 0x14 wraps",);
    assert_eq!(st.b[B_SCRX], 0xFF);
    obj.scr_y_sub(&mut st, 0x21);
    assert_eq!(st.b[B_SCRY], 0xFF);
    obj.scr_y_add(&mut st, 1);
    assert_eq!(st.b[B_SCRY], 0x00);
}

/// `$B2E8`-`$B32D`: the blink toggles XOR the sprite on a `$FFF0` gate, and
/// the eight entry points differ only in `(mask, xor)`.
#[test]
fn the_boss_blinks_gate_on_fff0() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    let mut st = streamed(0, 0x00, 0);
    st.b[B_SPRITE] = 0x40;
    obj.fff0 = 1;
    obj.sprite_blink(&mut st, 0x01, 0x01);
    assert_eq!(st.b[B_SPRITE], 0x40, "odd frame: the mask fails");
    obj.fff0 = 0;
    obj.sprite_blink(&mut st, 0x01, 0x01);
    assert_eq!(st.b[B_SPRITE], 0x41);
    obj.fff0 = 8; // 8 & 7 == 0: the $B321 gate
    obj.sprite_blink(&mut st, 0x07, 0x1F);
    assert_eq!(st.b[B_SPRITE], 0x5E, "$B321's xor");
}

/// `$B224`: appending a child def record — the boss's extra parts. The
/// record is slot `count + 1`'s byte 5 onward, inheriting the parent's
/// `$FFD6-$FFDA`, and the live count grows so the driver walks it.
#[test]
fn the_boss_child_spawner_appends_a_def_record() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 1;
    let mut st = streamed(0, 0x00, 0);
    st.b[B_RECHI] = 0x0A;
    st.b[B_STATE] = 2;
    st.b[B_SCRY] = 0x30;
    st.b[B_SCRX] = 0x90;
    obj.spawn_anim = 4;
    obj.spawn_sprite = 9;
    obj.spawn_attr = 0x20;
    obj.boss_spawn_child(&st);
    assert_eq!(obj.boss_def_count, 2);
    let child = &obj.slots[2].b;
    assert_eq!(child[B_DEF], 4, "the record's first byte is the anim");
    assert_eq!(child[B_RECHI], 0x0A, "the parent's $FFD6-$FFDA");
    assert_eq!(child[B_SCRY], 0x30);
    assert_eq!(child[B_SCRX], 0x90);
    assert_eq!(child[B_SPRITE], 9);
    assert_eq!(child[B_FLIP], 0x20, "the attr skips the defeat byte");
    assert_eq!(
        (obj.spawn_anim, obj.spawn_sprite, obj.spawn_attr),
        (0, 0, 0)
    );
    // Past the last slot: no write.
    obj.boss_def_count = 0x0F;
    obj.spawn_anim = 4;
    obj.boss_spawn_child(&st);
    assert_eq!(obj.boss_def_count, 0x0F, "$B228's $10 abort");
}

/// The shared boss hit prologue (`$93AA`-`$93D4`): a hit banks sprite,
/// state and type, parks the slot in state 1 (its hit-stun) and — only for a
/// stomp — clears the sprite and loads `sTimerReload`.
#[test]
fn the_shared_hit_prologue_banks_and_parks() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_timer_reload = 0x50;
    let mut st = streamed(0, 0x00, 0);
    st.b[B_SPRITE] = 0x35;
    st.b[B_FSTATE] = 3;
    st.b[B_TYPE] = 0x45;
    // A projectile hit: no latch, so no sprite/timer clobber.
    obj.boss_hit_save(&mut st);
    assert_eq!(obj.spawn_sprite, 0x35, "banked");
    assert_eq!(obj.saved_frame_state, 3);
    assert_eq!(obj.saved_type, 0x45);
    assert_eq!(st.b[B_SPRITE], 0x35, "kept: not a stomp");
    assert_eq!(st.b[B_TYPE], 0);
    assert_eq!(st.b[B_FSTATE], 1, "parks in the hit-stun");
    // A stomp: sprite cleared and the reload timer loaded.
    let mut st = streamed(0, 0x00, 0);
    st.b[B_SPRITE] = 0x35;
    obj.boss_hit_latch = 1;
    obj.boss_hit_save(&mut st);
    assert_eq!(st.b[B_SPRITE], 0);
    assert_eq!(st.b[B_T28], 0x50);
}

/// `$9443`: the morph tail — flash, count down, then clear the state, hand
/// the slot to anim 3 and stop the music on the primary slot only.
#[test]
fn the_boss_morph_tail_hands_the_slot_to_anim3() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = streamed(0, 0x00, 0);
    st.b[B_T28] = 2;
    obj.boss_slot_ptr_lo = 0;
    obj.boss_morph_to_anim3(&mut st);
    assert_eq!(st.b[B_T28], 1, "still counting");
    assert_eq!(st.b[B_DEF], 0, "not yet handed over");
    assert_eq!(obj.music_request, None);
    obj.boss_morph_to_anim3(&mut st);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(st.b[B_DEF], 3, "handed to anim 3");
    assert_eq!(st.b[B_FLASH], 0);
    assert_eq!(obj.music_request, Some(0xFF), "the music stops");
    // A non-primary slot leaves the music alone.
    obj.music_request = None;
    obj.boss_slot_ptr_lo = 0x20;
    st.b[B_T28] = 0;
    obj.boss_morph_to_anim3(&mut st);
    assert_eq!(obj.music_request, None);
}

/// `$93A7` (boss anim 1): the intro wakes the fight once the fade is idle,
/// and begins its source movement profile before a projectile parks it in stun.
#[test]
fn the_def0_boss_wakes_up_then_moves_and_banks_a_hit() {
    let real: ObjectTables = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = streamed(0, 0x00, 0);
    // A fade in progress: the intro holds.
    obj.fade_timer = 0x0280;
    obj.fade_idle();
    obj.dispatch_handler(0x93A7, &mut st, &layout, &real);
    assert_eq!(st.b[B_TYPE], 0, "still waiting on the fade");
    // Idle: the fight arms.
    obj.fade_timer = 0;
    obj.dispatch_handler(0x93A7, &mut st, &layout, &real);
    assert_eq!(st.b[B_TYPE], 0x45);
    assert_eq!(st.b[B_SPRITE], 3);
    assert_eq!(st.b[B_FSTATE], 3, "$93E5: into the first sweep state");
    // State 3 reads the profile before incrementing its phase.
    let old_y = st.b[B_SCRY];
    obj.dispatch_handler(0x93A7, &mut st, &layout, &real);
    assert_eq!(st.b[B_PHASE], 1);
    assert_eq!(
        st.b[B_SCRY],
        old_y.wrapping_add(real.boss_picker_profile[0])
    );
    assert!(obj.unported.is_empty());
    // A hit banks the slot and parks it in the hit-stun.
    obj.done = 0;
    st.b[B_FLAGS] = 0x02;
    obj.boss_def_index = 0;
    obj.boss_hits_remaining = 6;
    obj.dispatch_handler(0x93A7, &mut st, &layout, &real);
    assert_eq!(st.b[B_FSTATE], 1, "the hit-stun");
    assert_eq!(obj.saved_frame_state, 3);
}

/// The def-2 parts (boss anims 10/11) carry HP in `B_STATE` — seeded by the
/// def image — and 0 or a borrow sends the part to `$5435`, the defeat fall.
#[test]
fn a_def2_part_takes_hp_and_falls_at_zero() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 2;
    obj.boss_sprite = 0x2A;
    obj.boss_timer_reload = 0x50;
    let mut st = streamed(0, 0x00, 0);
    st.b[B_DEF] = 0x0A;
    st.b[B_STATE] = 2; // the def image's 6, shortened for the test
    st.b[B_SPRITE] = 0x2C;
    st.b[B_FLAGS] = 0x02; // a projectile
    obj.dispatch_handler(0x9C36, &mut st, &layout, &t);
    assert_eq!(st.b[B_STATE], 1, "one point");
    assert_eq!(obj.sfx1, 9);
    assert_eq!(obj.spawn_sprite, 0x2C, "banked");
    assert_eq!(st.b[B_FSTATE], 1, "into the hit-stun");
    // The next point kills it.
    st.b[B_FLAGS] = 0x02;
    obj.dispatch_handler(0x9C36, &mut st, &layout, &t);
    assert_eq!(st.b[B_STATE], 0);
    assert_eq!(st.b[B_FSTATE], 2, "$5435: the defeat fall");
    assert_eq!(st.b[B_T28], 0x20);
    assert_eq!(st.b[B_SPRITE], 0x2A, "the sprite comes from the def");
    // And a *borrow* kills too: 1 HP against a stomp's 2 wraps to $FF, which
    // the ROM's `jp c` catches just like a clean 0.
    let mut st = streamed(0, 0x00, 0);
    st.b[B_DEF] = 0x0A;
    st.b[B_STATE] = 1;
    st.b[B_FLAGS] = 0x01;
    obj.boss_hits_remaining = 6;
    obj.boss_slot_ptr_lo = 0;
    obj.boss_hit_latch = 0;
    obj.dispatch_handler(0x9C36, &mut st, &layout, &t);
    assert_eq!(st.b[B_STATE], 0xFF, "the wrap");
    assert_eq!(st.b[B_FSTATE], 2, "and the borrow is fatal");
}

/// A stomp takes two points, latches, and loads the reload timer.
#[test]
fn a_def2_stomp_takes_two_points_and_latches() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 2;
    obj.boss_timer_reload = 0x50;
    let mut st = streamed(0, 0x00, 0);
    st.b[B_DEF] = 0x0A;
    st.b[B_STATE] = 6;
    st.b[B_FLAGS] = 0x01; // a stomp
    obj.boss_hits_remaining = 6;
    obj.boss_slot_ptr_lo = 0;
    obj.dispatch_handler(0x9C36, &mut st, &layout, &t);
    assert_eq!(st.b[B_STATE], 4, "two points");
    assert_eq!(obj.sfx0, 0x13);
    assert_eq!(st.b[B_SPRITE], 0x1F, "the hit pose");
    assert_eq!(st.b[B_T28], 0x50, "the reload timer");
    assert_eq!(obj.boss_hits_remaining, 4, "$9285 booked it too");
}

/// The controller (anim 12) banks the slot but never touches `$FFD8`, and
/// its intro waits for the parts to clear before spawning its own child.
#[test]
fn the_def2_controller_has_no_hp_and_spawns_once() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_index = 2;
    obj.boss_def_count = 1; // one part still alive
    let mut st = streamed(0, 0x00, 0);
    st.b[B_DEF] = 0x0C;
    st.b[B_STATE] = 0;
    st.b[B_FLAGS] = 0x02;
    obj.boss_slot_ptr_lo = 0;
    obj.boss_hits_remaining = 6;
    obj.dispatch_handler(0x9E07, &mut st, &layout, &t);
    assert_eq!(st.b[B_STATE], 0, "no HP on the controller");
    assert_eq!(st.b[B_FSTATE], 1, "$9725: type 0 / state 1");
    assert_eq!(obj.spawn_sprite, 0x00, "it banks its own sprite");
    // The intro holds while a part is alive.
    st.b[B_FSTATE] = 0;
    st.b[B_FLAGS] = 0;
    obj.dispatch_handler(0x9E07, &mut st, &layout, &t);
    assert_eq!(st.b[B_T28], 0, "no counting yet");
    // With the parts gone it counts to $30, then spawns.
    obj.boss_def_count = 0;
    for _ in 0..0x31 {
        st.b[B_FSTATE] = 0;
        obj.done = 0; // the driver clears it per slot per frame
        obj.dispatch_handler(0x9E07, &mut st, &layout, &t);
        if obj.boss_def_count != 0 {
            break; // it spawned: the state it left is the assertion
        }
    }
    assert_eq!(obj.slots[1].b[B_DEF], 0x0D, "the child's anim");
    assert_eq!(obj.slots[1].b[B_SPRITE], 0x0E, "and its sprite");
    assert_eq!(st.b[B_FSTATE], 3, "then the walk arms");
    assert_eq!(st.b[B_TYPE], 0x45);
    // And the walk's speed is the fight's HP: 4 - hits/2. The hit above
    // spent one, so 5 hits -> 4 - 2 = 2 px. State 4 moves left ($B264).
    st.b[B_FSTATE] = 4;
    st.b[B_SCRX] = 0x40;
    obj.fff0 = 1; // the odd-frame gate
    obj.dispatch_handler(0x9E07, &mut st, &layout, &t);
    assert_eq!(st.b[B_SCRX], 0x40, "odd frames hold");
    obj.fff0 = 2;
    obj.dispatch_handler(0x9E07, &mut st, &layout, &t);
    assert_eq!(st.b[B_SCRX], 0x3E, "4 - 5/2 = 2 px left");
}

/// `$9CDF`: the bob picks its profile by `B_T24` — the cap-15 ascent, then
/// the cap-21 descent, which ends the flight at `B_SCRY = $90`.
#[test]
fn the_def2_bob_alternates_its_profiles() {
    let real: ObjectTables = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = streamed(0, 0x00, 0);
    st.b[B_SCRY] = 0x70;
    st.b[B_T24] = 0;
    let before = st.b[B_SCRY];
    obj.boss_bob(&mut st, &real);
    assert!(st.b[B_SCRY] < before, "the ascent climbs");
    // Walk the cap-15 profile out: it flips B_T24 and resets the phase.
    for _ in 0..0x10 {
        obj.boss_bob(&mut st, &real);
    }
    assert_eq!(st.b[B_T24], 1, "then the descent takes over");
    // And the descent ends at $90 with state 4.
    st.b[B_SCRY] = 0x8E;
    for _ in 0..0x18 {
        obj.boss_bob(&mut st, &real);
        if st.b[B_FSTATE] == 4 {
            break;
        }
    }
    assert_eq!(st.b[B_SCRY], 0x90);
    assert_eq!(st.b[B_FSTATE], 4);
    assert_eq!(st.b[B_FLIP], 0x20);
}

/// `$9606` (boss anim 2): the def-2 spawner arms only while the slot's
/// remove-mode is 2, spawns one anim-4/sprite-9 child, parks at sprite 7 and
/// never runs again.
#[test]
fn the_def2_boss_spawner_arms_only_while_removing() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 0;
    let mut st = streamed(0, 0x00, 0);
    st.b[B_DEF] = 2;
    st.b[B_TYPE] = 0x48;
    // Not removing: nothing happens.
    obj.handler_boss_def2(&mut st);
    assert_eq!(st.b[B_TYPE], 0x48);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(obj.boss_def_count, 0);
    // Removing: it spawns and parks.
    obj.remove_mode = 2;
    obj.handler_boss_def2(&mut st);
    assert_eq!(st.b[B_TYPE], 0);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.b[B_SPRITE], 7);
    assert_eq!(obj.boss_def_count, 1);
    assert_eq!(obj.slots[1].b[B_DEF], 4, "the child");
    assert_eq!(obj.slots[1].b[B_SPRITE], 9);
    // A second call returns at the state test.
    st.b[B_DEF] = 2;
    obj.handler_boss_def2(&mut st);
    assert_eq!(obj.boss_def_count, 1, "once only");
    let _ = (&layout, &t);
}

/// anim 0 (`$9322`) is a bare `ret` — the same inert-anim idiom as the level
/// table's `$58100`. The dispatch must reach it without reporting a gap.
#[test]
fn the_inert_boss_anim_is_dispatched_not_reported() {
    let real: ObjectTables = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(real.boss_anim_handler(0), Some(0x9322));
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    let mut st = Staging {
        b: [0; 32],
        ..Default::default()
    };
    obj.dispatch_handler(0x9322, &mut st, &layout, &real);
    assert!(obj.unported.is_empty(), "a fixture, not a gap");
    assert_eq!(st.b, [0u8; 32], "and it changes nothing");
}

/// The boss path never stages `B_SCRY`/`B_SCRX` from world coords: the def
/// image carries them (`$AD05`'s bytes 4/5 are the screen pair) and the
/// handlers move them. A driver frame must therefore leave a slot's screen
/// bytes exactly as it found them — deriving them from `$C090` would clobber
/// every boss that had moved.
#[test]
fn the_boss_driver_leaves_the_screen_bytes_alone() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.spawn_boss(4);
    // The def's own screen pair, as `$8E6C` staged it at $AD05: the
    // 18-byte image starts at slot byte 5, so its bytes 4/5 land in
    // `B_SCRY`/`B_SCRX`. Def 4's image carries $30/$90 (def 0's, for
    // contrast, carries $7C/$58).
    let (scy, scx) = (obj.slots[0].b[B_SCRY], obj.slots[0].b[B_SCRX]);
    assert_eq!((scy, scx), (0x30, 0x90), "$AD09/$AD0A from the def image");
    // Move the world position far away: if the driver derived the screen
    // pair from it, the assertion below would fail.
    obj.slots[0].b[B_XHI] = 0x05;
    obj.slots[0].b[B_XLO] = 0x00;
    obj.slots[0].b[B_YHI] = 0x03;
    obj.slots[0].b[B_YLO] = 0x00;
    obj.boss_engine(2000, 1500, &layout, &t);
    assert_eq!(
        (obj.slots[0].b[B_SCRY], obj.slots[0].b[B_SCRX]),
        (scy, scx),
        "the handler/def screen bytes survive"
    );
}

/// `$92DF` dispatches through the *boss* table (`$92E8`) — anim 16's boss
/// handler is `$A0B9` (bank 2), while the level's profile table gives
/// `$586F5` (bank 22) for the same anim. The boss path must initialize
/// the native boss, never silently run the level handler.
#[test]
fn the_boss_driver_dispatches_through_the_92e8_table() {
    let real: ObjectTables = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        ))
        .unwrap(),
    )
    .unwrap();
    // Bank-2 base: entry 16 ($60B9) is file $A0B9, NOT $5A0B9 — the first
    // extraction added the bank-22 constant ($54000) to a bank-2 table.
    assert_eq!(
        real.boss_anim_handler(16),
        Some(0xA0B9),
        "boss table entry 16"
    );
    // The table's length is not declared; the extractor takes the leading
    // run of in-window values (29), so anim 29 is past it and must report
    // rather than serve the next routine's data.
    assert!(real.boss_anim_handler(28).is_some());
    assert_eq!(real.boss_anim_handler(29), None, "past the table");
    assert_eq!(real.behavior_for(0, 16), Some(0x586F5), "level table");

    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 0;
    obj.slots[0].b[B_DEF] = 16;
    obj.boss_engine(0, 0, &layout, &real);
    assert_eq!(obj.slots[0].b[B_FSTATE], 3, "boss initialization state");
    assert_eq!(obj.slots[0].b[B_TYPE], 69);
    assert_eq!(obj.slots[0].b[B_T28], 1);
    assert_eq!(obj.door_phase, 4);
    assert!(obj.unported.is_empty(), "boss handler is implemented");
    assert!(
        !obj.unported.contains(&0x586F5),
        "and the level's handler for the same anim is not run"
    );
}

/// `$0652`: while `sSceneState & $F0 != 0` the frame's object pass is the
/// boss driver *instead of* `$8000` — the level engine's own tail (`$817D`
/// latches the camera into `prev_cam_x`, then the spawn scan) must not run.
#[test]
fn a_parked_frame_runs_the_boss_driver_and_not_the_level_engine() {
    let t = tables();
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.boss_def_count = 0;
    obj.slots[0].b[B_DEF] = 1;
    let mario = MarioContact::default();

    obj.step(0x40, 0x40, &layout, &t, &mario, true);
    assert_eq!(obj.prev_cam_x, 0, "$8000's tail did not run");
    assert_eq!(obj.fff0, 1, "the boss driver's $FFF0 bump did");

    obj.step(0x40, 0x40, &layout, &t, &mario, false);
    assert_eq!(obj.prev_cam_x, 0x40, "unparked, the level engine runs");
    assert_eq!(obj.fff0, 1, "and does not touch $FFF0 on an even phase");
}

/// `$C014`: the bank-3 draw returns before drawing a slot whose
/// `sObjectDone` is set, which is why a defeated enemy disappears instead
/// of freezing where it stood — `$85FC` (the defeat) sets that byte.
#[test]
fn a_defeated_slot_is_not_drawn() {
    let layout = blank_layout();
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.active = 1;
    obj.spawned = vec![true];
    obj.mario = MarioContact::default();
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_DEF] = 9;
        s.b[B_TYPE] = 0x4E;
        s.b[B_FSTATE] = 1;
        s.b[B_SPRITE] = 3;
        set_x(&mut s.b, 100);
        set_y(&mut s.b, 96);
    }
    obj.engine(0, 0, &layout, &t);
    assert!(!obj.slot_done[0], "a live object draws");
    assert_eq!(obj.slots[0].b[B_TYPE], 0x4E);

    // The fireball's flag (bit 1) -> the dispatch's defeat -> `$85FC` sets
    // `sObjectDone`, and `$C014` stops the draw on that very frame.
    obj.slots[0].b[B_FLAGS] = 0x02;
    obj.engine(0, 0, &layout, &t);
    assert_eq!(obj.slots[0].b[B_TYPE], 0, "defeated");
    assert_eq!(obj.slots[0].b[B_DEF], 0);
    assert!(obj.slot_done[0], "and it is not drawn");
}

/// `$C014`'s type gate: a slot whose staged type (`$FFE2`) is exactly `$80`
/// is not drawn. That is the item still inside its block — `$58758` parks
/// it there (type `$80`, frame state 2) and `$8611` ignores Mario's touch on
/// it — so it stays invisible until the hit from below (`$86E4` -> `$8708`)
/// turns the type into anim + `$70` and the rise starts.
#[test]
fn an_item_inside_its_block_is_not_drawn_until_released() {
    let layout = blank_layout();
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.active = 1;
    obj.spawned = vec![true];
    obj.mario = MarioContact::default();
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_DEF] = 18; // `$58758`, the item pose
        s.b[B_SPRITE] = 26;
        set_x(&mut s.b, 100);
        set_y(&mut s.b, 96);
    }
    // Frame 1: `$58758`'s init parks the item (type `$80`) and sets
    // `sObjectDone`, so it is skipped on this frame either way.
    obj.engine(0, 0, &layout, &t);
    assert_eq!(obj.slots[0].b[B_TYPE], 0x80);
    assert_eq!(obj.slots[0].b[B_FSTATE], 2);
    // Later frames: `sObjectDone` is clear, but the type is still `$80`.
    for _ in 0..4 {
        obj.engine(0, 0, &layout, &t);
        assert_eq!(obj.slots[0].b[B_TYPE], 0x80);
        assert!(obj.slot_done[0], "an item inside its block is not drawn");
    }
    // The block is hit from below: the item is released and drawn.
    obj.slots[0].b[B_FLAGS] = 0x20;
    obj.engine(0, 0, &layout, &t);
    assert_eq!(obj.slots[0].b[B_TYPE], 0x82, "anim 18 + $70");
    assert!(!obj.slot_done[0], "the released item draws");
}

/// The same chain through the engine, with the *committed* extraction's
/// anim-0 handler ($581B1, the death animation): the frames after a
/// fireball defeat flip the enemy, hop it up and draw it again as it falls,
/// instead of freezing the sprite and flagging anim 0 as an unported gap.
#[test]
fn a_fireball_defeat_plays_the_death_animation_through_the_engine() {
    let real: ObjectTables = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        real.profile_for(0, 0).map(|p| p.handler),
        Some(0x581B1),
        "the extraction's anim 0 IS the death handler"
    );
    let mut t = tables();
    t.anim_profiles_a[0] = prof(0x581B1);

    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.active = 1;
    obj.spawned = vec![true];
    obj.mario = MarioContact::default();
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_DEF] = 9;
        s.b[B_TYPE] = 0x4E;
        s.b[B_FSTATE] = 1;
        s.b[B_SPRITE] = 3;
        s.b[B_FLAGS] = 0x02; // the fireball's defeat flag
        set_x(&mut s.b, 100);
        set_y(&mut s.b, 96);
    }
    obj.engine(0, 0, &layout, &t);
    assert_eq!(obj.slots[0].b[B_TYPE], 0, "defeated");
    assert!(obj.slot_done[0], "and not drawn on the defeat frame");

    let y0 = y_of(&obj.slots[0].b);
    for _ in 0..3 {
        obj.engine(0, 0, &layout, &t);
    }
    assert!(obj.unported.is_empty(), "anim 0 is dispatched, not a gap");
    assert!(y_of(&obj.slots[0].b) < y0, "it hops up (Y -= 20)");
    assert_eq!(obj.slots[0].b[B_FLIP] & 0x40, 0x40, "flipped upside down");
    assert!(!obj.slot_done[0], "and it is drawn again while it falls");
}

/// The whole koopa chain the play test exercised: a stunned shell (anim 2,
/// type `$06`) is picked up with B held (`$8611` -> anim 4, carried),
/// follows Mario, is released (anim 4 -> 3) and then *walks* as the kicked
/// walker instead of hanging where it was thrown.
#[test]
fn the_shell_chain_pickup_carry_release_walks() {
    let mut layout = blank_layout();
    layout.cells[6 * 256..=6 * 256 + 24].fill(0x20); // ground at row 6
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    // Mario's Y is 91 so that the carried offset (+5 for big Mario) leaves
    // the shell at Y = 96 — row 6, the ground row — while its wall probes
    // (y-8 = 88) read row 5, the air above it.
    obj.mario = MarioContact {
        x: 100,
        y: 91,
        powerup: 1,
        b_held: true,
        dir_right: true,
        ..Default::default()
    };
    obj.mario_ref_x = 108;
    // The stunned shell beside Mario.
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_DEF] = 2;
        s.b[B_TYPE] = 0x06;
        s.b[B_FSTATE] = 1;
        set_x(&mut s.b, 120);
        set_y(&mut s.b, 96);
    }
    // 1. B-held contact: the pickup.
    let mut st = Staging {
        b: obj.slots[0].b,
        ..Default::default()
    };
    st.b[B_FLAGS] = 0x10;
    let contact = obj.mario;
    obj.post_touch(&mut st, &[0u8; 48], &t, &contact);
    assert_eq!(st.b[B_DEF], 4, "the shell is carried");
    assert_eq!(obj.action_latch, 1);

    // 2. Carried: the first frame only initialises the carried state
    //    ($58452: type 0, state 1), from then on it follows Mario
    //    (+8 X, +5 Y for big Mario).
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!(st.b[B_FSTATE], 1, "$58452: type 0, state 1");
    assert_eq!(st.b[B_TYPE], 0);
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!((st.x(), st.y()), (108, 96), "pinned to Mario");
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!((st.x(), st.y()), (108, 96), "and it stays there");

    // 3. B released: anim 3 and the placement nudge.
    obj.mario.b_held = false;
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!(st.b[B_DEF], 3, "the kicked walker");
    assert_eq!(st.x(), 124, "nudged 16 px out of Mario");

    // 4. `$3AB4` calls `$8240`: the new anim starts in state 0. Its first
    //    update arms contact $5F before the later walking updates move it.
    let x0 = st.x();
    assert_eq!(st.b[B_FSTATE], 0, "release resets the handler state");
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.b[B_TYPE], 0x5F, "the released shell is live");
    assert_eq!(st.x(), x0, "initialization precedes movement");
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.x(), x0 + 3, "but it walks 3 px");
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.x(), x0 + 6, "and keeps walking");

    // 5. Off the ledge: it falls, then lands back into the walk.
    layout.cells[6 * 256..=6 * 256 + 24].fill(0x60);
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.b[B_FSTATE], 3, "no ground: state += 2 (falling right)");
    let y0 = st.y();
    obj.handler_kicked(&mut st, &layout);
    assert!(st.y() > y0, "and it falls rather than hanging");
    layout.cells[6 * 256..=6 * 256 + 24].fill(0x20);
    obj.handler_kicked(&mut st, &layout);
    assert_eq!(st.b[B_FSTATE], 1, "landed, walking again");
    assert_eq!(st.b[B_PHASE], 0, "with the fall index cleared");
}

/// `$58444` (anim 4): while B is held the carried shell is pinned to Mario
/// (`$5B4C1`: his X + 8 and Y + 5 big / 14 small / 20 small+ducking), and on
/// release it drops to anim 3 (via `$3AB4`) and runs the placement tail.
#[test]
fn the_carried_shell_follows_mario_and_releases_into_a_kick() {
    let layout = blank_layout();
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.action_latch = 1;
    obj.mario = MarioContact {
        x: 200,
        y: 300,
        powerup: 1,
        b_held: true,
        dir_right: true,
        ..Default::default()
    };
    let mut st = streamed(4, 0x00, 0);
    st.b[B_FSTATE] = 1;
    set_x(&mut st.b, 0);
    set_y(&mut st.b, 0);
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!(st.x(), 208, "Mario's X + 8");
    assert_eq!(st.y(), 305, "and Y + 5 for big Mario");

    // Small and ducking takes the 20 px offset.
    obj.mario.powerup = 0;
    obj.mario.ducking = true;
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!(st.y(), 320, "small ducking: Y + 20");

    // Release: anim 4 -> 3 plus the placement tail (no wall -> the shell
    // stays anim 3, 16 px further out than the carried position).
    obj.mario.ducking = false;
    obj.mario.powerup = 1;
    obj.mario.b_held = false;
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!(st.b[B_DEF], 3, "the release drops the anim");
    assert_eq!(obj.action_latch, 0, "the latch clears");
    assert_eq!(st.x(), 224, "and the placement nudges +16 (no wall)");
    assert_eq!(st.b[B_TYPE], 0, "open air: no live-shell type");
}

/// The carried shell's exit arms: an object it knocked out
/// (`sObjectHitFlag`) and a ceiling inside the `$3B68` probe both defeat the
/// shell itself (`$584C3` -> `$3ADC` = `$85FC`), and a pipe ride removes it.
#[test]
fn the_carried_shell_is_defeated_by_a_hit_or_a_ceiling() {
    let layout = blank_layout();
    let t = tables();
    // An enemy under the carried shell's staged screen pair.
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario = MarioContact {
        x: 200,
        y: 300,
        powerup: 1,
        b_held: true,
        ..Default::default()
    };
    let mut st = streamed(4, 0x00, 0);
    st.b[B_FSTATE] = 1;
    st.b[B_SCRY] = 96;
    st.b[B_SCRX] = 96;
    {
        let s = &mut obj.slots[0];
        s.b[B_STATE] = 2;
        s.b[B_TYPE] = 0x4E;
        s.b[B_SCRY] = 90;
        s.b[B_SCRX] = 96;
        s.b[B_HB_BOTTOM] = 0x10;
        s.b[B_HB_RIGHT] = 0x10;
    }
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!(st.b[B_DEF], 0, "$584C3 defeated the shell");
    assert_eq!(st.b[B_TYPE], 0);
    assert_eq!(obj.action_latch, 0, "and cleared the latch");

    // A ceiling above it does the same.
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario = MarioContact {
        x: 200,
        y: 300,
        powerup: 1,
        b_held: true,
        ..Default::default()
    };
    let mut layout2 = blank_layout();
    layout2.cells[18 * 256 + 13] = 0x20; // the (x, y-8) probe: the followed (208, 297)
    let mut st = streamed(4, 0x00, 0);
    st.b[B_FSTATE] = 1;
    obj.handler_carried(&mut st, &layout2, &t);
    assert_eq!(st.b[B_DEF], 0, "the $3B68 ceiling defeats it");
    assert_eq!(obj.action_latch, 0);

    // A pipe ride removes it instead.
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.action_latch = 1;
    obj.mario = MarioContact {
        travelling: true,
        ..Default::default()
    };
    let mut st = streamed(4, 0x00, 0);
    st.b[B_STATE] = 2;
    obj.handler_carried(&mut st, &layout, &t);
    assert_eq!(st.b[B_STATE], 0, "$5B2D7 removal");
    assert_eq!(obj.action_latch, 0);
}

#[test]
fn handler_walker_native_migration_preserves_raw_slot_bytes() {
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario_ref_x = 150;
    obj.fff0 = 2; // even frame

    // 1. Initial run: faces Mario (mario_ref_x > 100) -> state 2, flip 0x20, flash 16, type 0x4E
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    st.b[B_FSTATE] = 0;
    obj.handler_walker(&mut st, &layout);
    assert_eq!(st.b[B_FSTATE], 2);
    assert_eq!(st.b[B_FLIP], 0x20);
    assert_eq!(st.b[B_FLASH], 16);
    assert_eq!(st.b[B_TYPE], 0x4E);
    assert_eq!(obj.facing, 1);

    // 2. Initial run with Mario to the left -> state 1, flip 0
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario_ref_x = 50;
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    st.b[B_FSTATE] = 0;
    obj.handler_walker(&mut st, &layout);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.b[B_FLIP], 0);
    assert_eq!(st.b[B_FLASH], 16);
    assert_eq!(st.b[B_TYPE], 0x4E);
    assert_eq!(obj.facing, 0xFF);

    // 3. Wall hit turns Walker
    let mut layout_wall = blank_layout();
    let col = (93 / 16) as usize;
    let row = (189 / 16) as usize;
    layout_wall.cells[row * 256 + col] = 0x20;
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.fff0 = 2;
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    st.b[B_FSTATE] = 1; // WalkLeft
    obj.handler_walker(&mut st, &layout_wall);
    assert_eq!(st.b[B_FSTATE], 2, "turned right on wall");
    assert_eq!(st.b[B_FLIP], 0x20);
    assert_eq!(x_of(&st.b), 100, "did not move X on turn");
}

#[test]
fn handler_fallwalker_native_migration_preserves_raw_slot_bytes() {
    let layout = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.fff0 = 1; // odd frame: bob is no-op

    // 1. In-air fall: increments phase, sets type 0x56 for fallwalker
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    st.b[B_FSTATE] = 0;
    st.b[B_PHASE] = 0;
    obj.handler_fallwalker(&mut st, &layout);
    assert_eq!(st.b[B_TYPE], 0x56);
    assert_eq!(st.b[B_PHASE], 1);
    assert_eq!(y_of(&st.b), 200);

    // 2. In-air fall: increments phase, sets type 0x52 for fallwalker52
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    st.b[B_FSTATE] = 0;
    st.b[B_PHASE] = 0;
    obj.handler_fallwalker52(&mut st, &layout);
    assert_eq!(st.b[B_TYPE], 0x52);
    assert_eq!(st.b[B_PHASE], 1);
    assert_eq!(y_of(&st.b), 200);

    // 3. Grounded init: resets phase to 0, faces Mario (mario_ref_x > obj_x -> state 2, flip 0x20, facing 1)
    let mut ground_layout = blank_layout();
    let col = (100 / 16) as usize;
    let row = (192 / 16) as usize;
    ground_layout.cells[row * 256 + col] = 0x20;
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.fff0 = 1;
    obj.mario_ref_x = 150;
    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 192);
    st.b[B_FSTATE] = 0;
    st.b[B_PHASE] = 5;
    obj.handler_fallwalker(&mut st, &ground_layout);
    assert_eq!(st.b[B_FSTATE], 2);
    assert_eq!(st.b[B_FLIP], 0x20);
    assert_eq!(st.b[B_PHASE], 0);
    assert_eq!(st.b[B_TYPE], 0x56);
    assert_eq!(obj.facing, 1);
}

#[test]
fn handler_squashed_native_migration_preserves_raw_slot_bytes() {
    let t = tables();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    // State 0 init: sets flash 16, state 1, phase 32, type 0
    let mut st = Staging::default();
    st.b[B_FSTATE] = 0;
    st.b[B_TYPE] = 0x4E;
    obj.handler_squashed(&mut st, &t);
    assert_eq!(st.b[B_FLASH], 16);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.b[B_PHASE], 32);
    assert_eq!(st.b[B_TYPE], 0);

    // State 1 countdown: decrements phase
    obj.handler_squashed(&mut st, &t);
    assert_eq!(st.b[B_PHASE], 31);
    assert_eq!(st.b[B_STATE], 0); // not removed yet

    // At phase 1: counts down to 0 and despawns
    st.b[B_PHASE] = 1;
    st.b[B_STATE] = 2; // Active slot
    obj.handler_squashed(&mut st, &t);
    assert_eq!(st.b[B_PHASE], 0);
    assert_eq!(st.b[B_STATE], 0, "removed via remove_mode 1");
    assert_eq!(obj.money_bag, 1, "credited coin on remove_mode 1");
    assert_eq!(obj.remove_mode, 0, "reset after removal");
}

#[test]
fn handler_bobber_downward_0x584cb_dispatch_and_state_transitions() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario_ref_x = 100;

    let mut st = Staging::default();
    set_x(&mut st.b, 100); // Mario is right above (dx = 0 < 24)
    set_y(&mut st.b, 200);

    // Dispatch 0x584CB: Waiting state, Mario too close -> stays waiting, defeat = 0x80
    obj.dispatch_handler(0x584CB, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(st.b[B_DEFEAT], 0x80);

    // Move Mario far away (dx = 30 >= 24)
    obj.mario_ref_x = 130;
    obj.dispatch_handler(0x584CB, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.b[B_T24], 32);
    assert_eq!(st.b[B_T25], 32);
    assert_eq!(st.b[B_TYPE], 0x44);
    assert_eq!(st.y(), 200);

    // Step 1 of downward emergence (moves y + 1)
    obj.dispatch_handler(0x584CB, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.y(), 201);
    assert_eq!(st.b[B_T24], 31);

    // Finish 31 remaining emergence steps
    for _ in 0..31 {
        obj.dispatch_handler(0x584CB, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_FSTATE], 2, "transitions to Retracting");
    assert_eq!(st.y(), 232);
    assert_eq!(st.b[B_T24], 32);
    assert_eq!(st.b[B_T28], 24);

    // Step 1 of retraction (moves y - 1 back toward ceiling)
    obj.dispatch_handler(0x584CB, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2);
    assert_eq!(st.y(), 231);
    assert_eq!(st.b[B_T24], 31);

    // Finish 31 remaining retraction steps
    for _ in 0..31 {
        obj.dispatch_handler(0x584CB, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_FSTATE], 3, "transitions to Resting");
    assert_eq!(st.y(), 200, "returned to exact starting position");
    assert_eq!(st.b[B_T28], 24);

    // Rest 24 frames
    for _ in 0..24 {
        obj.dispatch_handler(0x584CB, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_FSTATE], 0, "back to Waiting state");
}

#[test]
fn handler_bobber_upward_0x584e0_dispatch_and_state_transitions() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario_ref_x = 130;

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);

    // Emerging trigger
    obj.dispatch_handler(0x584E0, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.y(), 200);

    // Step 1 of upward emergence (moves y - 1)
    obj.dispatch_handler(0x584E0, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.y(), 199);
    assert_eq!(st.b[B_T24], 31);
}

#[test]
fn handler_bobber_pipe_travel_removes_object() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario.travelling = true;

    let mut st = Staging::default();
    st.b[B_STATE] = 2; // Active slot
    obj.dispatch_handler(0x584CB, &mut st, &l, &t);
    assert_eq!(obj.done, 2, "despawn removal marked done=2");
}

#[test]
fn handler_fire_piranha_0x58541_full_lifecycle_and_projectile_spawn() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario_ref_x = 100;
    obj.mario_ref_y = 150; // Mario is above

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);

    // Waiting state: Mario dx = 0 < 24 -> stays waiting
    obj.dispatch_handler(0x58541, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 0);
    assert_eq!(st.b[B_DEFEAT], 0x80);

    // Mario moves far away (dx = 30 >= 24)
    obj.mario_ref_x = 130;
    obj.dispatch_handler(0x58541, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1);
    assert_eq!(st.b[B_T24], 32);
    assert_eq!(st.b[B_T25], 32);
    assert_eq!(st.b[B_TYPE], 0x44);

    // Step 1 through 31 of emergence (y -= 1, aiming up sprite = 18)
    for _ in 1..=31 {
        obj.dispatch_handler(0x58541, &mut st, &l, &t);
        assert_eq!(st.b[B_FSTATE], 1);
        assert_eq!(st.b[B_SPRITE], 18);
    }
    assert_eq!(st.y(), 169);

    // Step 32: reaches top, opens mouth (sprite 19), spawns projectile (anim 14), transitions to state 2 (timer 48)
    let standby_before = obj.standby;
    obj.dispatch_handler(0x58541, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2);
    assert_eq!(st.b[B_SPRITE], 19);
    assert_eq!(st.b[B_T28], 48);
    assert_eq!(
        obj.standby,
        standby_before + 1,
        "spawned standby projectile slot"
    );
    // Find the spawned slot
    let spawned_child = obj.slots.iter().find(|s| s.b[B_DEF] == 14);
    assert!(spawned_child.is_some(), "child slot with anim 14 exists");
    let child = spawned_child.unwrap();
    assert_eq!(child.b[B_STATE], 1, "child is in Standby");
    assert_eq!(child.b[B_SPRITE], 22, "child sprite is 22");

    // State 2: wait 48 frames
    for _ in 0..48 {
        obj.dispatch_handler(0x58541, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_FSTATE], 3, "transitions to Retracting");
    assert_eq!(st.b[B_SPRITE], 18, "closes mouth to sprite 18");
    assert_eq!(st.b[B_T24], 32);

    // State 3: retract 32 steps
    for _ in 0..32 {
        obj.dispatch_handler(0x58541, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_FSTATE], 4, "transitions to Resting");
    assert_eq!(st.y(), 200, "back at starting Y");
    assert_eq!(st.b[B_T28], 4, "rest timer is 4");

    // State 4: resting ticks down on frames where fff0 % 8 == 0
    for tick in 1..=4 {
        obj.fff0 = tick * 8;
        obj.dispatch_handler(0x58541, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_FSTATE], 0, "returned to Waiting state");
}

#[test]
fn handler_fire_piranha_pipe_travel_despawn() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.mario.travelling = true;

    let mut st = Staging::default();
    st.b[B_STATE] = 2; // Active slot
    obj.dispatch_handler(0x58541, &mut st, &l, &t);
    assert_eq!(obj.done, 2, "despawn removal marked done=2");
}

#[test]
fn handler_boo_0x59545_dispatch_and_stalking_behavior() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.fff0 = 4; // frame divisible by 4

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 100);

    // Case 1: Mario is at x = 150 looking left (facing Boo).
    // Boo is shy!
    obj.mario_ref_x = 150;
    obj.mario.dir_right = false;
    obj.dispatch_handler(0x59545, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 0, "state is Shy (0)");
    assert_eq!(st.b[B_SPRITE], 14, "shy sprite is 14");

    // Case 2: Mario turns around (dir_right = true).
    // Boo uncovers face (sprite 13) and pursues right!
    obj.mario.dir_right = true;
    obj.dispatch_handler(0x59545, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1, "state is ChasingRight (1)");
    assert_eq!(st.b[B_SPRITE], 13);
    assert_eq!(st.b[B_FLIP] & 0x20, 0x20, "horizontally flipped right");
    assert_eq!(st.b[B_TYPE], 0x48);

    // Case 3: Step right. Next frame divisible by 4.
    obj.fff0 = 8;
    obj.mario_ref_y = 100; // Mario at same Y -> motion_leg remains 8 -> dy = 0
    obj.dispatch_handler(0x59545, &mut st, &l, &t);
    assert_eq!(st.x(), 101, "moved 1 px right");
    assert_eq!(st.y(), 100);

    // Case 4: Mario turns back to look at Boo (dir_right = false).
    // Boo immediately freezes in shy pose.
    obj.fff0 = 12;
    obj.mario.dir_right = false;
    obj.dispatch_handler(0x59545, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 0, "frozen in Shy state");
    assert_eq!(st.b[B_SPRITE], 14);
}

#[test]
fn handler_piranha_fireball_0x585ed_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    st.b[B_DEF] = 14;
    st.b[B_SPRITE] = 22;
    // Downward right: horizontal flip = true, vertical flip = true
    st.b[B_FLIP] = 0x60;

    // Dispatch frame 0: Init frame
    obj.fff0 = 1; // odd frame (no sprite toggle)
    obj.dispatch_handler(0x585ED, &mut st, &l, &t);
    assert_eq!(st.b[B_TYPE], 0x40, "lethal obstacle type 0x40");
    assert_eq!(st.y(), 188, "offset y by -12");
    assert_eq!(st.x(), 100);
    assert_eq!(st.b[B_FLIP] & 0x40, 0, "vertical flip cleared");
    assert_eq!(st.b[B_T24], 1, "motion_leg set to 1 for downward direction");
    assert_eq!(st.b[B_FSTATE], 2, "state is MovingRight (2)");

    // Dispatch frame 1: Moving right (+2 X, +1 Y)
    obj.fff0 = 2; // even frame: toggles sprite 22 -> 23
    obj.dispatch_handler(0x585ED, &mut st, &l, &t);
    assert_eq!(st.x(), 102, "advanced 2 px right");
    assert_eq!(st.y(), 189, "advanced 1 px down");
    assert_eq!(st.b[B_SPRITE], 23, "toggled sprite to 23");

    // Dispatch frame 2: Next step (+2 X, +1 Y)
    obj.fff0 = 3; // odd frame: keeps sprite 23
    obj.dispatch_handler(0x585ED, &mut st, &l, &t);
    assert_eq!(st.x(), 104, "advanced to 104");
    assert_eq!(st.y(), 190, "advanced to 190");
    assert_eq!(st.b[B_SPRITE], 23);
}

#[test]
fn handler_paced_walker_0x59ce7_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 50);
    st.b[B_SPRITE] = 162;
    obj.mario_ref_x = 200; // Mario to the right
    obj.fff0 = 1; // not multiple of 4

    // Dispatch frame 0: State 0 init
    obj.dispatch_handler(0x59CE7, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2, "state is WalkRight (2)");
    assert_eq!(st.b[B_FLIP] & 0x20, 0x20, "horizontal flip set");
    assert_eq!(st.b[B_FLASH], 0x10, "flash byte set to 0x10");
    assert_eq!(st.b[B_TYPE], 76, "contact kind 76 (0x4C)");
    assert_eq!(st.b[B_T28], 1, "pause timer initialized to 1");
    assert_eq!(st.x(), 100);

    // Dispatch frame 1: timer 1 -> 0, arms budget to 48, no movement
    obj.fff0 = 2;
    obj.dispatch_handler(0x59CE7, &mut st, &l, &t);
    assert_eq!(st.b[B_T28], 0);
    assert_eq!(st.b[B_T25], 48);
    assert_eq!(st.x(), 100);

    // Dispatch frame 2: step 1 to the right
    obj.fff0 = 3;
    obj.dispatch_handler(0x59CE7, &mut st, &l, &t);
    assert_eq!(st.b[B_T25], 47);
    assert_eq!(st.x(), 101);

    // Dispatch frame 3: tick multiple of 4 (fff0 = 4) toggles sprite
    obj.fff0 = 4;
    obj.dispatch_handler(0x59CE7, &mut st, &l, &t);
    assert_eq!(st.b[B_T25], 46);
    assert_eq!(st.x(), 102);
    assert_eq!(st.b[B_SPRITE], 163, "low bit of sprite toggled");
}

#[test]
fn handler_vertical_pacer_0x59e06_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 200);
    st.b[B_SPRITE] = 74;
    obj.fff0 = 1; // odd frame

    // Dispatch frame 0: State 0 init
    obj.dispatch_handler(0x59E06, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1, "state is MoveUp (1)");
    assert_eq!(st.b[B_FLASH], 0x10, "flash byte set to 0x10");
    assert_eq!(st.b[B_TYPE], 76, "contact kind 76 (0x4C)");
    assert_eq!(st.b[B_T24], 48, "motion_leg initialized to 48");
    assert_eq!(st.y(), 200);

    // Dispatch frame 1: odd frame (fff0 = 1) -> gate returns, no movement
    obj.dispatch_handler(0x59E06, &mut st, &l, &t);
    assert_eq!(st.b[B_T24], 48);
    assert_eq!(st.y(), 200);

    // Dispatch frame 2: even frame (fff0 = 2) -> steps Y up by 1
    obj.fff0 = 2;
    obj.dispatch_handler(0x59E06, &mut st, &l, &t);
    assert_eq!(st.b[B_T24], 47);
    assert_eq!(st.y(), 199);

    // Dispatch frame 3: even frame (fff0 = 4) -> steps Y up by 1 and toggles sprite
    obj.fff0 = 4;
    obj.dispatch_handler(0x59E06, &mut st, &l, &t);
    assert_eq!(st.b[B_T24], 46);
    assert_eq!(st.y(), 198);
    assert_eq!(st.b[B_SPRITE], 75, "low bit of sprite toggled");
}

#[test]
fn handler_perched_flyer_0x5a1f6_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 80);
    st.b[B_SPRITE] = 94;
    obj.mario_ref_x = 200; // far away (> 24)

    // Dispatch frame 0: State 0 init
    obj.dispatch_handler(0x5A1F6, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 3, "state is Perched (3)");
    assert_eq!(st.b[B_TYPE], 77, "contact kind 77 (0x4D)");

    // Dispatch frame 1: Mario still far away -> sets bit 0 of sprite, stays perched
    obj.dispatch_handler(0x5A1F6, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 3);
    assert_eq!(st.b[B_SPRITE], 95);
    assert_eq!(st.x(), 100);
    assert_eq!(st.y(), 80);
    assert_eq!(obj.sfx1, 0);

    // Dispatch frame 2: Mario approaches to 110 (10 px to the right, < 24) -> startled!
    obj.mario_ref_x = 110;
    obj.dispatch_handler(0x5A1F6, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2, "state becomes FleeLeft (2)");
    assert_eq!(st.b[B_FLIP] & 0x20, 0x20, "flip horizontal set");
    assert_eq!(obj.sfx1, 23, "emits SFX 23");
    assert_eq!(st.x(), 100);

    // Dispatch frame 3: flies away (X -= 3, Y -= 1)
    obj.fff0 = 2; // even frame toggles sprite
    obj.dispatch_handler(0x5A1F6, &mut st, &l, &t);
    assert_eq!(st.x(), 97);
    assert_eq!(st.y(), 79);
    assert_eq!(st.b[B_SPRITE], 94, "sprite toggled");
}

#[test]
fn handler_cannon_ant_0x5a773_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 80);
    st.b[B_SPRITE] = 146;
    obj.mario_ref_x = 50; // Mario to the left
    obj.mario_ref_y = 40; // Mario above

    // Dispatch frame 0: State 0 init
    obj.dispatch_handler(0x5A773, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1, "state is WalkLeft (1)");
    assert_eq!(st.b[B_TYPE], 77, "contact kind 77 (0x4D)");
    assert_eq!(st.b[B_T25], 24, "initial walk budget 24");
    assert_eq!(st.b[B_T24], 48, "full walk budget 48");

    // Fast-forward 23 steps left
    for _ in 1..=23 {
        obj.dispatch_handler(0x5A773, &mut st, &l, &t);
    }
    assert_eq!(st.x(), 77);
    assert_eq!(st.b[B_T25], 1);

    // 24th step reaches 0: transitions to Aim (3), aims up (sprite 148)
    obj.dispatch_handler(0x5A773, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 3, "state is Aim (3)");
    assert_eq!(st.b[B_SPRITE], 148, "aiming up sprite 148");
    assert_eq!(st.b[B_T28], 16, "aim timer 16");

    // 16 frames of countdown
    for _ in 1..=16 {
        obj.dispatch_handler(0x5A773, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_T28], 0);

    // Next frame: fires projectile (anim 78, sprite 145, sfx 17)
    obj.dispatch_handler(0x5A773, &mut st, &l, &t);
    assert_eq!(obj.sfx1, 17, "fires with sfx 17");
    assert_eq!(obj.standby, 1, "spawned dynamic projectile in standby");
    assert_eq!(st.b[B_FSTATE], 4, "state is Recovery (4)");
    assert_eq!(st.b[B_T28], 32, "recovery timer 32");
}

#[test]
fn handler_charging_enemy_0x5a9e8_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 80);
    st.b[B_SPRITE] = 175;
    obj.mario_ref_x = 110; // Mario close (< 40)

    // Dispatch frame 0: State 0 init
    obj.dispatch_handler(0x5A9E8, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 5, "state is Wait (5)");
    assert_eq!(st.y(), 82, "moves Y by +2");
    assert_eq!(st.b[B_DEFEAT], 0x80, "defeat mode armored 0x80");
    assert_eq!(st.b[B_TYPE], 74, "contact kind 74 (0x4A)");
    assert_eq!(st.b[B_T25], 24, "creep budget 24");
    assert_eq!(st.b[B_T28], 24, "pause timer 24");

    // Dispatch frame 1: Mario still close (< 40) -> stays in state 5
    obj.dispatch_handler(0x5A9E8, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 5);

    // Dispatch frame 2: Mario moves far to the right (160, distance 60 >= 40) -> triggers CreepRight (2)
    obj.mario_ref_x = 160;
    obj.dispatch_handler(0x5A9E8, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2, "state is CreepRight (2)");
    assert_eq!(st.b[B_FLIP] & 0x20, 0x20, "faces right");

    // Fast-forward 24 frames of pause
    for _ in 1..=24 {
        obj.dispatch_handler(0x5A9E8, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_T28], 0);

    // Fast-forward 23 creep steps right
    for _ in 1..=23 {
        obj.dispatch_handler(0x5A9E8, &mut st, &l, &t);
    }
    assert_eq!(st.x(), 123);
    assert_eq!(st.b[B_T25], 1);

    // 24th creep step finishes: triggers charge (state 4), clears defeat mode, emits sfx 16
    obj.dispatch_handler(0x5A9E8, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 4, "state is ChargeRight (4)");
    assert_eq!(st.b[B_DEFEAT], 0, "defeat mode cleared to 0");
    assert_eq!(obj.sfx1, 16, "emitted sfx 16");

    // Charge step 1: moves X by +3 (124 + 3 = 127)
    obj.dispatch_handler(0x5A9E8, &mut st, &l, &t);
    assert_eq!(st.x(), 127);
}

#[test]
fn handler_wave_flyer_0x59f5e_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 80);
    st.b[B_SPRITE] = 140;
    obj.mario_ref_x = 150; // Mario to the right
    obj.fff0 = 0;

    // Dispatch frame 0: State 0 init (Mario to right -> FlyRight)
    obj.dispatch_handler(0x59F5E, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2, "state is FlyRight (2)");
    assert_eq!(st.b[B_FLIP] & 0x20, 0x20, "faces right (flip_h)");
    assert_eq!(st.b[B_FLASH], 0x10, "flash is 0x10");
    assert_eq!(st.b[B_TYPE], 72, "contact kind 72 (0x48)");
    assert_eq!(st.b[B_T25], 48, "budget is 48");
    assert_eq!(st.b[B_T24], 0, "motion leg is Climbing (0)");
    assert_eq!(obj.facing, 1, "facing recorded as 1 (right)");
    assert_eq!(st.b[B_SPRITE], 141, "sprite bit 0 toggles on fff0=0");

    // Frames 1..3: fff0 not multiple of 4 -> movement skipped, sprite not toggled
    for f in 1..=3 {
        obj.fff0 = f;
        obj.dispatch_handler(0x59F5E, &mut st, &l, &t);
        assert_eq!(st.x(), 100);
        assert_eq!(st.y(), 80);
        assert_eq!(st.b[B_SPRITE], 141);
    }

    // Frame 4 (step 1): moves X by +2, Y by -1, budget decrements to 46, sprite toggles back to 140
    obj.fff0 = 4;
    obj.dispatch_handler(0x59F5E, &mut st, &l, &t);
    assert_eq!(st.x(), 102);
    assert_eq!(st.y(), 79);
    assert_eq!(st.b[B_T25], 46);
    assert_eq!(st.b[B_SPRITE], 140);

    // Fast-forward 22 more active steps (steps 2..=23, frames 8..=92)
    for step in 2..=23 {
        obj.fff0 = step * 4;
        obj.dispatch_handler(0x59F5E, &mut st, &l, &t);
    }
    assert_eq!(st.x(), 146);
    assert_eq!(st.y(), 57); // 80 - 23
    assert_eq!(st.b[B_T25], 2);

    // Step 24 (frame 96): Apex reached!
    // Moves X by +2 (to 148), budget reloaded to 48, motion_leg becomes Diving (1), Y does not move.
    obj.fff0 = 96;
    obj.dispatch_handler(0x59F5E, &mut st, &l, &t);
    assert_eq!(st.x(), 148);
    assert_eq!(st.y(), 57);
    assert_eq!(st.b[B_T25], 48);
    assert_eq!(st.b[B_T24], 1, "motion leg transitions to Diving (1)");

    // Step 25 (diving step 1, frame 100): moves X by +2, Y by +1, budget decrements to 46
    obj.fff0 = 100;
    obj.dispatch_handler(0x59F5E, &mut st, &l, &t);
    assert_eq!(st.x(), 150);
    assert_eq!(st.y(), 58);
    assert_eq!(st.b[B_T25], 46);

    // Fast-forward 22 more diving steps (steps 26..=47, frames 104..=188)
    for step in 26..=47 {
        obj.fff0 = step * 4;
        obj.dispatch_handler(0x59F5E, &mut st, &l, &t);
    }
    assert_eq!(st.x(), 194);
    assert_eq!(st.y(), 80);
    assert_eq!(st.b[B_T25], 2);

    // Step 48 (frame 192): Trough reached & turnaround!
    // Moves X by +2 (to 196), budget reloaded to 48, motion_leg flips to Climbing (0),
    // state flips to FlyLeft (1), flip_h flips to 0, Y does not move.
    obj.fff0 = 192;
    obj.dispatch_handler(0x59F5E, &mut st, &l, &t);
    assert_eq!(st.x(), 196);
    assert_eq!(st.y(), 80);
    assert_eq!(st.b[B_T25], 48);
    assert_eq!(st.b[B_T24], 0, "motion leg transitions to Climbing (0)");
    assert_eq!(st.b[B_FSTATE], 1, "state toggles to FlyLeft (1)");
    assert_eq!(st.b[B_FLIP] & 0x20, 0, "faces left (no flip_h)");
}

#[test]
fn handler_horizontal_platform_0x58c6a_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 80);
    obj.mario_ref_x = 150; // Mario to the right
    obj.mario.x = 100;
    obj.mario.y = 64;

    // Frame 0: Init (Mario right -> InitialRight / 2)
    obj.dispatch_handler(0x58C6A, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2, "state is InitialRight (2)");
    assert_eq!(st.b[B_TYPE], 32, "contact kind 32 (0x20 solid on-object)");
    assert_eq!(st.b[B_T24], 36, "motion leg is 36 (half span)");
    assert_eq!(st.b[B_PHASE], 0, "motion phase is 0");
    assert_eq!(st.b[B_FLIP] & 0x20, 0x20, "faces right (flip_h)");
    assert_eq!(obj.facing, 1);

    // Mario stands on platform (bit 3 of B_FLAGS / interactions)
    st.b[B_FLAGS] = 0x08;

    // 35 steps right
    for step in 1..=35 {
        obj.dispatch_handler(0x58C6A, &mut st, &l, &t);
        assert_eq!(st.x(), 100 + step);
        assert_eq!(obj.mario_x_adjust, Some(100 + step), "carries Mario right");
    }
    assert_eq!(st.x(), 135);
    assert_eq!(st.b[B_PHASE], 35);

    // 36th step: reaches boundary (136), turns to FullLeft (3), motion_phase resets to 0
    obj.dispatch_handler(0x58C6A, &mut st, &l, &t);
    assert_eq!(st.x(), 136);
    assert_eq!(st.b[B_FSTATE], 3, "turns to FullLeft (3)");
    assert_eq!(st.b[B_PHASE], 0, "motion phase resets to 0");
    assert_eq!(obj.mario_x_adjust, Some(136));

    // Move 72 steps left in FullLeft
    for _ in 1..=72 {
        obj.dispatch_handler(0x58C6A, &mut st, &l, &t);
    }
    // 136 - 72 = 64
    assert_eq!(st.x(), 64);
    assert_eq!(st.b[B_FSTATE], 4, "turns to FullRight (4)");
    assert_eq!(st.b[B_PHASE], 0, "motion phase resets to 0");
    assert_eq!(obj.mario_x_adjust, Some(64));
}

#[test]
fn handler_winged_walker_0x58f4c_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 80);
    st.b[B_SPRITE] = 86;
    obj.mario_ref_x = 150; // Mario to the right
    obj.fff0 = 0;

    // Frame 0: Init (Mario right -> WalkRight / 2)
    obj.dispatch_handler(0x58F4C, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2, "state is WalkRight (2)");
    assert_eq!(st.b[B_TYPE], 77, "contact kind 77 (0x4D)");
    assert_eq!(st.b[B_FLASH], 16, "flash is 16");
    assert_eq!(st.b[B_FLIP] & 0x20, 0x20, "faces right (flip_h)");
    assert_eq!(obj.facing, 1);

    // Frame 0 (even): steps right by 1 px (100 -> 101)
    obj.dispatch_handler(0x58F4C, &mut st, &l, &t);
    assert_eq!(st.x(), 101);
    assert_eq!(st.b[B_FSTATE], 2);

    // Frame 1 (odd): blank_layout has no ground under (101, 80) -> triggers takeoff to FlyUp (5)!
    obj.fff0 = 1;
    obj.dispatch_handler(0x58F4C, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 5, "transitions to FlyUp (5)");
    assert_eq!(st.b[B_SPRITE], 88, "sprite is 88 (fly)");
    assert_eq!(st.b[B_T24], 0, "altitude is 0");

    // Next frame (frame 2): rises UP by 1 (Y: 80 -> 79), altitude becomes 1
    obj.fff0 = 2;
    obj.dispatch_handler(0x58F4C, &mut st, &l, &t);
    assert_eq!(st.y(), 79);
    assert_eq!(st.b[B_T24], 1);
}

#[test]
fn handler_edge_walker_0x59baf_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);

    let mut st = Staging::default();
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 80);
    st.b[B_SPRITE] = 90;
    obj.mario_ref_x = 150; // Mario to the right
    obj.fff0 = 0;

    // Frame 0: Init (Mario right -> WalkRight / 2)
    obj.dispatch_handler(0x59BAF, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2, "state is WalkRight (2)");
    assert_eq!(st.b[B_TYPE], 77, "contact kind 77 (0x4D)");
    assert_eq!(st.b[B_FLIP] & 0x20, 0x20, "faces right (flip_h)");
    assert_eq!(obj.facing, 1);
    assert_eq!(
        st.b[B_SPRITE], 91,
        "four-step bob delta (+1) applied to sprite 90 -> 91"
    );

    // Frame 0 (even): steps right 1 px (100 -> 101)
    obj.dispatch_handler(0x59BAF, &mut st, &l, &t);
    assert_eq!(st.x(), 101);
    assert_eq!(st.b[B_FSTATE], 2);

    // Frame 1 (odd): blank_layout has no ground under (101, 80) -> reverses at ledge to WalkLeft (1)!
    obj.fff0 = 1;
    obj.dispatch_handler(0x59BAF, &mut st, &l, &t);
    assert_eq!(
        st.b[B_FSTATE], 1,
        "ledge reverse toggles state to WalkLeft (1)"
    );
    assert_eq!(st.b[B_FLIP] & 0x20, 0, "faces left (no flip_h)");

    // Frame 2 (even): steps left 1 px (101 -> 100)
    obj.fff0 = 2;
    obj.dispatch_handler(0x59BAF, &mut st, &l, &t);
    assert_eq!(st.x(), 100);
    assert_eq!(st.b[B_FSTATE], 1);
}

#[test]
fn handler_left_marcher_0x591de_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(6); // level 6 (Tree Zone 2)

    let mut st = Staging::default();
    set_x(&mut st.b, 200);
    set_y(&mut st.b, 100);
    st.b[B_SPRITE] = 104;
    obj.fff0 = 1; // odd frame: no sprite advance

    // Frame 1: Init frame
    obj.dispatch_handler(0x591DE, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1, "state is Marching (1)");
    assert_eq!(
        st.b[B_TYPE], 77,
        "contact kind 77 (0x4D standard stompable)"
    );
    assert_eq!(st.x(), 200, "init frame does not move");
    assert_eq!(st.b[B_SPRITE], 104, "sprite untouched on odd frame");

    // Frame 2: Even frame -> moves 1 px left (200 -> 199), sprite cycle advances (104 -> 105)
    obj.fff0 = 2;
    obj.dispatch_handler(0x591DE, &mut st, &l, &t);
    assert_eq!(st.x(), 199, "moved 1 px left");
    assert_eq!(st.b[B_SPRITE], 105, "three-step cycle step 1: +1");

    // Frame 3: Odd frame -> moves 1 px left (199 -> 198), sprite unchanged
    obj.fff0 = 3;
    obj.dispatch_handler(0x591DE, &mut st, &l, &t);
    assert_eq!(st.x(), 198, "moved 1 px left");
    assert_eq!(st.b[B_SPRITE], 105, "odd frame: sprite unchanged");

    // Frame 4: Even frame -> moves 1 px left (198 -> 197), sprite cycle advances (105 -> 106)
    obj.fff0 = 4;
    obj.dispatch_handler(0x591DE, &mut st, &l, &t);
    assert_eq!(st.x(), 197, "moved 1 px left");
    assert_eq!(st.b[B_SPRITE], 106, "three-step cycle step 2: +1");

    // Frame 6: Even frame -> moves 1 px left (197 -> 196), sprite cycle wraps (106 -> 104)
    obj.fff0 = 6;
    obj.dispatch_handler(0x591DE, &mut st, &l, &t);
    assert_eq!(st.x(), 196, "moved 1 px left");
    assert_eq!(st.b[B_SPRITE], 104, "three-step cycle step 3: -2");
}

#[test]
fn handler_falling_dropper_0x595b2_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(7); // level 7 (Tree Zone 3)

    let mut st = Staging::default();
    set_x(&mut st.b, 200);
    set_y(&mut st.b, 80);
    st.b[B_SPRITE] = 37;

    // Mario is far away: x = 260 -> dx = 60 >= 56 -> stays Waiting
    obj.mario_ref_x = 260;
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 0, "state is Waiting (0)");
    assert_eq!(st.b[B_TYPE], 72, "contact kind 72 (0x48)");
    assert_eq!(obj.sfx1, 0, "no SFX while waiting");

    // Mario enters trigger zone: x = 250 -> dx = 50 < 56 -> triggers drop!
    obj.mario_ref_x = 250;
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1, "state is Wobbling (1)");
    assert_eq!(st.b[B_T28], 16, "countdown timer is 16");
    assert_eq!(obj.sfx1, 21, "emitted SFX 21 on trigger");

    // Frame 1 of wobble: timer becomes 15, travel_direction 0 -> 1 (+3 X)
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.b[B_T28], 15);
    assert_eq!(st.x(), 203, "wobbled +3 px right");

    // Frame 2 of wobble: timer becomes 14, travel_direction 1 -> 0 (-3 X)
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.b[B_T28], 14);
    assert_eq!(st.x(), 200, "wobbled -3 px left");

    // Advance 13 more frames of wobble (frames 3..=15)
    for _ in 3..=15 {
        obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    }
    assert_eq!(st.b[B_T28], 1);
    assert_eq!(st.x(), 203);
    assert_eq!(st.b[B_FSTATE], 1);

    // Frame 16: timer reaches 0 -> transitions to Falling (2)
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 2, "state is Falling (2)");
    assert_eq!(st.x(), 203, "no X shift on transition frame");

    // Falling frame 1 (phase 0): FALL_SPEED[0] = 0 -> dy = 0, phase becomes 1
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.y(), 80);
    assert_eq!(st.b[B_PHASE], 1);

    // Falling frame 2 (phase 1): FALL_SPEED[1] = 0 -> dy = 0, phase becomes 2
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.y(), 80);
    assert_eq!(st.b[B_PHASE], 2);

    // Falling frame 3 (phase 2): FALL_SPEED[2] = 1 -> dy = 2 (80 -> 82), phase becomes 3
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.y(), 82, "fell 2 px");
    assert_eq!(st.b[B_PHASE], 3);

    // Falling frame 4 (phase 3): FALL_SPEED[3] = 1 -> dy = 2 (82 -> 84), phase becomes 4
    obj.dispatch_handler(0x595B2, &mut st, &l, &t);
    assert_eq!(st.y(), 84, "fell 2 px");
    assert_eq!(st.b[B_PHASE], 4);
}

#[test]
fn handler_dashing_walker_0x59d5a_dispatch_and_movement() {
    let t = tables();
    let l = blank_layout();
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(15); // level 15 (Space Zone 1)

    let mut st = Staging::default();
    set_x(&mut st.b, 200);
    set_y(&mut st.b, 80);
    st.b[B_SPRITE] = 14;

    // Mario is to the left: x = 100 -> faces left: PatrolLeft (1), flip 0
    obj.mario_ref_x = 100;
    obj.dispatch_handler(0x59D5A, &mut st, &l, &t);
    assert_eq!(st.b[B_FSTATE], 1, "state is PatrolLeft (1)");
    assert_eq!(st.b[B_FLIP] & 0x20, 0, "facing left");
    assert_eq!(st.b[B_TYPE], 72, "contact kind 72 (0x48)");
    assert_eq!(st.b[B_FLASH], 0x10, "flash 0x10");
    assert_eq!(obj.facing, 0xFF, "facing left");

    // Mario far away: x = 100 (dx = 100 >= 36) -> steps 1 px left
    obj.dispatch_handler(0x59D5A, &mut st, &l, &t);
    assert_eq!(st.x(), 199);
    assert_eq!(st.b[B_FSTATE], 1);

    // Mario moves close: x = 180 (dx = 19 < 36) -> steps 1 px left (198) and triggers ChargeLeft (3)
    obj.mario_ref_x = 180;
    obj.dispatch_handler(0x59D5A, &mut st, &l, &t);
    assert_eq!(st.x(), 198);
    assert_eq!(st.b[B_FSTATE], 3, "state transitioned to ChargeLeft (3)");
    assert_eq!(st.b[B_PHASE], 0);

    // Charge step 1 (phase 0): FALL_SPEED[0] = 0 -> moves 0 px, phase becomes 1
    obj.dispatch_handler(0x59D5A, &mut st, &l, &t);
    assert_eq!(st.x(), 198);
    assert_eq!(st.b[B_PHASE], 1);

    // Charge step 2 (phase 1): FALL_SPEED[1] = 0 -> moves 0 px, phase becomes 2
    obj.dispatch_handler(0x59D5A, &mut st, &l, &t);
    assert_eq!(st.x(), 198);
    assert_eq!(st.b[B_PHASE], 2);

    // Charge step 3 (phase 2): FALL_SPEED[2] = 1 -> moves 1 px left (198 -> 197), phase becomes 3
    obj.dispatch_handler(0x59D5A, &mut st, &l, &t);
    assert_eq!(st.x(), 197);
    assert_eq!(st.b[B_PHASE], 3);
}

/// `$2EAA` & `$85E2`: defeating enemies with star increments the streak counter,
/// sets the bounce timer to 32, plays sfx 16 for kills 1..4, and plays sfx 9
/// and awards a life for kill 5 and above.
#[test]
fn star_defeat_streak_increments_plays_sfx_and_awards_lives_at_5() {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.mario.star_high = 4; // Mario has active star

    // Enemy slot 0
    obj.slots[0].b[B_STATE] = 2;
    obj.slots[0].b[B_TYPE] = 0x40; // bit 6 set, not immune

    let t = tables();

    // Kill 1
    obj.harm_or_star(0);
    assert_eq!(obj.stomp_chain, 1);
    assert_eq!(obj.stomp_bounce_timer, 32);
    let mut st = Staging::default();
    st.b = obj.slots[0].b;
    obj.interaction_dispatch(&mut st, &t);
    assert_eq!(obj.sfx0, 16, "kill 1 emits sfx 16");
    assert_eq!(obj.award_lives, 0, "no extra life yet");

    // Kills 2, 3, 4
    for count in 2..=4 {
        obj.slots[0].b[B_TYPE] = 0x40;
        obj.harm_or_star(0);
        assert_eq!(obj.stomp_chain, count);
        let mut st = Staging::default();
        st.b = obj.slots[0].b;
        obj.interaction_dispatch(&mut st, &t);
        assert_eq!(obj.sfx0, 16, "kill {count} emits sfx 16");
        assert_eq!(obj.award_lives, 0);
    }

    // Kill 5: awards 1UP (sfx0 = 9) and increments lives
    obj.slots[0].b[B_TYPE] = 0x40;
    obj.harm_or_star(0);
    assert_eq!(obj.stomp_chain, 5);
    assert_eq!(obj.sfx0, 9, "kill 5 sets sfx 9");
    assert_eq!(obj.award_lives, 1, "kill 5 awards 1 life");
    let mut st = Staging::default();
    st.b = obj.slots[0].b;
    obj.interaction_dispatch(&mut st, &t);
    assert_eq!(obj.sfx0, 9, "interaction dispatch keeps sfx 9");

    // Timer countdown: 32 ticks down to 0
    let mut count = 0;
    while let Some((timer, chain)) = obj.tick_stomp_bounce_timer() {
        assert_eq!(chain, 5);
        assert_eq!(timer, 31 - count);
        count += 1;
    }
    assert_eq!(count, 32);
    assert_eq!(obj.stomp_bounce_timer, 0);
    assert!(obj.tick_stomp_bounce_timer().is_none());
}

fn margin_record(x: u16) -> SpawnRecord {
    SpawnRecord {
        x,
        y: 0x0090,
        anim_index: 16,
        mode_flag: 0,
        sprite_id: 0,
        hitbox: [0; 4],
        raw: [(x >> 8) as u8, x as u8, 0, 0, 0x90, 16],
    }
}

#[test]
fn a_view_margin_spawns_records_in_the_widened_band_only() {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    // Native scan edge is cam + $70; a 384-px view widens it by 112.
    let records = vec![
        margin_record(cam - 0x70 - 0x40), // left band
        margin_record(cam + 0x40),        // inside the native window: never filled
        margin_record(cam + 0x70 + 0x40), // right band
        margin_record(cam + 0x70 + 0x78), // beyond the widened edge
    ];
    let mut obj = ObjectSim::new(records, cam);
    obj.configure(0);
    obj.view_margin = (112, 36);
    obj.step(cam, 0x0100, &layout, &t, &MarioContact::default(), false);
    assert_eq!(obj.spawned, vec![true, false, true, false]);
    assert_eq!(obj.standby, 2);
}

#[test]
fn a_culled_margin_record_waits_for_the_camera_to_uncover_it_again() {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    let mut obj = ObjectSim::new(vec![margin_record(cam + 0x70 + 0x40)], cam);
    obj.configure(0);
    obj.view_margin = (112, 36);
    let step = |obj: &mut ObjectSim, cam| {
        obj.step(cam, 0x0100, &layout, &t, &MarioContact::default(), false);
    };
    let holds_record = |obj: &ObjectSim| obj.slots.iter().any(|s| s.b[B_STATE] != 0);

    step(&mut obj, cam);
    assert_eq!(obj.spawned, vec![true]);
    let slot = obj.slots.iter().position(|s| s.b[B_STATE] != 0).unwrap();

    // Past the widened alive window: the odd-phase cull removes it.
    set_x(&mut obj.slots[slot].b, cam + 0x300);
    for _ in 0..2 {
        step(&mut obj, cam);
    }
    assert_eq!(obj.slots[slot].b[B_STATE], 0, "culled");

    for _ in 0..8 {
        step(&mut obj, cam);
        assert_eq!(obj.spawned, vec![false], "no respawn under a still camera");
        assert!(!holds_record(&obj));
    }

    step(&mut obj, cam + 0x200);
    assert_eq!(obj.spawned, vec![false]);
    step(&mut obj, cam);
    assert_eq!(obj.spawned, vec![true], "re-entering the window refills it");
    assert!(holds_record(&obj));
}

#[test]
fn a_zero_margin_leaves_a_still_camera_without_spawns() {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    let mut obj = ObjectSim::new(vec![margin_record(cam + 0x70 + 0x40)], cam);
    obj.configure(0);
    obj.step(cam, 0x0100, &layout, &t, &MarioContact::default(), false);
    assert_eq!(obj.spawned, vec![false]);
}

#[test]
fn a_view_margin_keeps_active_slots_alive_past_the_native_edge() {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    let mut obj = ObjectSim::new(Vec::new(), cam);
    obj.configure(0);
    obj.view_margin = (112, 36);
    obj.active = 1;
    let s = &mut obj.slots[0];
    s.b[B_STATE] = 2;
    s.b[B_RECHI] = 0xAD; // not stream-backed
    set_x(&mut s.b, cam + 0xA0 + 0x40); // past native alive, inside widened
    set_y(&mut s.b, 0x0100);
    for _ in 0..4 {
        obj.step(cam, 0x0100, &layout, &t, &MarioContact::default(), false);
    }
    assert_ne!(
        obj.slots[0].b[B_STATE], 0,
        "the widened alive window keeps it"
    );
}

/// Every horizontal offset a 384-px view keeps alive, with an active harmful
/// slot on Mario's row: which offsets hurt him.
fn hurt_offsets(margin: (u16, u16)) -> Vec<i32> {
    let t = tables();
    let layout = blank_layout();
    let cam = 0x0200;
    let mut base = ObjectSim::new(Vec::new(), cam);
    base.configure(0);
    base.view_margin = margin;
    base.active = 1;
    let mario = MarioContact {
        x: cam,
        y: 0x0100,
        ..Default::default()
    };
    let reach = 0xA0 + i32::from(margin.0);
    (-reach..=reach)
        .filter(|&dx| {
            let mut o = base.clone();
            let s = &mut o.slots[0];
            s.b[B_STATE] = 2;
            s.b[B_RECHI] = 0xAD;
            s.b[B_TYPE] = 0x40;
            s.b[B_HB_TOP] = 0;
            s.b[B_HB_BOTTOM] = 16;
            s.b[B_HB_LEFT] = 0;
            s.b[B_HB_RIGHT] = 16;
            set_x(&mut s.b, (i32::from(cam) + dx) as u16);
            set_y(&mut s.b, 0x0100 + 20);
            for _ in 0..2 {
                o.step(cam, 0x0100, &layout, &t, &mario, false);
            }
            o.mario_dead != 0
        })
        .collect()
}

#[test]
fn an_enlarged_view_never_hurts_mario_through_the_screen_byte_wrap() {
    assert_eq!(hurt_offsets((112, 36)), (5..=13).collect::<Vec<_>>());
}

#[test]
fn a_native_view_keeps_the_rom_contact() {
    assert_eq!(hurt_offsets((0, 0)), (5..=13).collect::<Vec<_>>());
}

/// A fire-hit slot boxed on the ball's screen point, with world X `dx` from
/// a camera at (`$0200`, `$0100`).
fn wide_fire_sim(margin: (u16, u16), dx: i32) -> ObjectSim {
    let mut obj = ObjectSim::new(Vec::new(), 0x0200);
    obj.view_margin = margin;
    obj.camera_position = crate::actor::Position {
        x: 0x0200,
        y: 0x0100,
    };
    fire_slot(&mut obj, 0, 0x24, 0);
    set_x(&mut obj.slots[0].b, (0x0200 + dx) as u16);
    set_y(&mut obj.slots[0].b, 0x0100);
    obj
}

#[test]
fn an_enlarged_view_fire_hit_skips_slots_out_of_the_balls_reach() {
    let ball = (0x0210, 0x0100);
    let mut obj = wide_fire_sim((112, 36), 0x100);
    assert_eq!(obj.fire_hit(0x4A, 0x4A, ball), FireHit::None);
    assert_eq!(obj.slots[0].b[B_FLAGS], 0, "no defeat through the wrap");
    let mut obj = wide_fire_sim((112, 36), 0x10);
    assert_eq!(obj.fire_hit(0x4A, 0x4A, ball), FireHit::Defeat);
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x02);
    let mut obj = wide_fire_sim((0, 0), 0x100);
    assert_eq!(
        obj.fire_hit(0x4A, 0x4A, ball),
        FireHit::Defeat,
        "native: ROM bytes"
    );
}

/// A ball out past the native window still hits the slot beside it: the
/// reach follows the ball, not the camera.
#[test]
fn an_enlarged_view_fire_hit_reaches_slots_past_the_native_window() {
    let mut obj = wide_fire_sim((112, 36), 0x100);
    assert_eq!(obj.fire_hit(0x4A, 0x4A, (0x0300, 0x0100)), FireHit::Defeat);
}

/// A `$80` pose on the bonk tile of `below_bonk_marks_idle_objects_in_the_tile_window`
/// with world X `dx` from a camera at (`$0200`, `$0100`).
fn wide_bonk_sim(margin: (u16, u16), dx: i32) -> ObjectSim {
    let mut obj = ObjectSim::new(Vec::new(), 0x0200);
    obj.view_margin = margin;
    obj.camera_position = crate::actor::Position {
        x: 0x0200,
        y: 0x0100,
    };
    let s = &mut obj.slots[0];
    s.b[B_TYPE] = 0x80;
    s.b[B_SCRY] = 0x97;
    s.b[B_SCRX] = 0xF8;
    set_x(&mut s.b, (0x0200 + dx) as u16);
    set_y(&mut s.b, 0x0100);
    obj
}

#[test]
fn an_enlarged_view_below_bonk_skips_slots_out_of_the_blocks_reach() {
    let block = (0x00DF, 0x0268);
    let mut obj = wide_bonk_sim((112, 36), -0x100);
    assert!(!obj.below_bonk(block, 0x31, 0x70));
    assert_eq!(obj.slots[0].b[B_FLAGS], 0, "no bonk through the wrap");
    let mut obj = wide_bonk_sim((112, 36), 0x10);
    assert!(obj.below_bonk(block, 0x31, 0x70));
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x20);
    let mut obj = wide_bonk_sim((0, 0), -0x100);
    assert!(obj.below_bonk(block, 0x31, 0x70), "native: ROM bytes");
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x20);
}

/// A shell at world (100, 100) and a walker with matching screen bytes at
/// world X `100 + dx`.
fn wide_ko_sim(margin: (u16, u16), dx: i32) -> (ObjectSim, Staging) {
    let mut obj = ObjectSim::new(Vec::new(), 0);
    obj.configure(0);
    obj.view_margin = margin;
    let mut st = streamed(3, 0x5F, 0);
    set_x(&mut st.b, 100);
    set_y(&mut st.b, 100);
    st.b[B_SCRY] = 96;
    st.b[B_SCRX] = 96;
    let s = &mut obj.slots[0];
    s.b[B_STATE] = 2;
    s.b[B_TYPE] = 0x4E;
    s.b[B_SCRY] = 90;
    s.b[B_SCRX] = 96;
    s.b[B_HB_TOP] = 0;
    s.b[B_HB_BOTTOM] = 0x10;
    s.b[B_HB_LEFT] = 0;
    s.b[B_HB_RIGHT] = 0x10;
    set_x(&mut s.b, (100 + dx) as u16);
    set_y(&mut s.b, 94);
    (obj, st)
}

#[test]
fn an_enlarged_view_shell_never_knocks_out_through_the_wrap() {
    let (mut obj, st) = wide_ko_sim((112, 36), 0x100);
    obj.ko_overlap(&st);
    assert_eq!(obj.slots[0].b[B_FLAGS], 0, "256 px away: no KO");
    assert_eq!(obj.object_hit_flag, 0);
    let (mut obj, st) = wide_ko_sim((112, 36), 0);
    obj.ko_overlap(&st);
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x02, "the overlap still hits");
    let (mut obj, st) = wide_ko_sim((0, 0), 0x100);
    obj.ko_overlap(&st);
    assert_eq!(obj.slots[0].b[B_FLAGS], 0x02, "native: ROM bytes");
}

/// In boss mode (`sSceneState & $F0 != 0`) slot screen bytes are the boss
/// path's own OAM positions and world X/Y are stale, so no slot is excluded.
#[test]
fn an_enlarged_view_never_excludes_boss_contacts() {
    let mut obj = ObjectSim::new(Vec::new(), 0x0200);
    obj.view_margin = (112, 36);
    obj.camera_position = crate::actor::Position {
        x: 0x0200,
        y: 0x0100,
    };
    set_x(&mut obj.slots[0].b, 0x0400);
    set_y(&mut obj.slots[0].b, 0x0100);
    obj.mario.scene_state = 0xF0;
    assert!(!obj.contact_excluded(0), "boss mode: screen bytes rule");
    obj.mario.scene_state = 0;
    assert!(
        obj.contact_excluded(0),
        "level mode: out of Mario's reach"
    );
}

/// How the contact-box equivalence test stages its slot's screen bytes.
#[derive(Clone, Copy, Debug)]
enum ScreenBytes {
    /// As `active_update` derives them from the slot's world position.
    Derived,
    /// Derived, then shifted (wrapping): bytes a previous engine pass left.
    Stale(u8, u8),
    /// Boss mode: written directly, world X/Y unrelated, state byte = HP 1.
    Boss,
}

/// Over a grid of Mario offsets, sizes, ducking and pans, the debug contact
/// boxes overlap exactly when `$2EF5` touches the slot.
fn assert_contact_boxes_match_collide(bytes: ScreenBytes) {
    let layout = blank_layout();
    let (cam_x, cam_y) = (0x0200u16, 0x0100u16);
    let mut base = ObjectSim::new(Vec::new(), cam_x);
    base.camera_position = crate::actor::Position { x: cam_x, y: cam_y };
    let (sx, sy) = (cam_x + 3, cam_y + 5);
    let mut hits = 0;
    for pan in [0u8, 2] {
        for powerup in [0u8, 1] {
            for ducking in [false, true] {
                for dy in -48..48 {
                    for dx in -40..40 {
                        let mut o = base.clone();
                        o.mario = MarioContact {
                            x: (i32::from(cam_x) + dx) as u16,
                            y: (i32::from(cam_y) + dy) as u16,
                            powerup,
                            ducking,
                            pan,
                            ..Default::default()
                        };
                        let s = &mut o.slots[0];
                        s.b[B_STATE] = 2;
                        s.b[B_TYPE] = 0x40;
                        s.b[B_HB_TOP] = 0xF8;
                        s.b[B_HB_BOTTOM] = 8;
                        s.b[B_HB_LEFT] = 0xFA;
                        s.b[B_HB_RIGHT] = 6;
                        set_x(&mut s.b, sx);
                        set_y(&mut s.b, sy);
                        let scroll_x = cam_x.wrapping_sub(80);
                        let scroll_y = cam_y.wrapping_sub(72).wrapping_sub(u16::from(pan & 3));
                        s.b[B_SCRY] = sy.wrapping_add(16).wrapping_sub(scroll_y) as u8;
                        s.b[B_SCRX] = sx.wrapping_add(8).wrapping_sub(scroll_x) as u8;
                        match bytes {
                            ScreenBytes::Derived => {}
                            ScreenBytes::Stale(bx, by) => {
                                s.b[B_SCRX] = s.b[B_SCRX].wrapping_add(bx);
                                s.b[B_SCRY] = s.b[B_SCRY].wrapping_add(by);
                            }
                            ScreenBytes::Boss => {
                                s.b[B_STATE] = 1;
                                set_x(&mut s.b, 0x0900);
                                set_y(&mut s.b, 0x0040);
                                o.mario.scene_state = 0x10;
                            }
                        }
                        let m = o.mario_contact_box(cam_x, cam_y);
                        let b = o.slot_contact_box(0, cam_x, cam_y).expect("gated in");
                        let overlap = m.x0 < b.x1 && b.x0 < m.x1 && m.y0 < b.y1 && b.y0 < m.y1;
                        o.collide(cam_x, cam_y, &layout);
                        let touched = o.slots[0].b[B_FLAGS] & 0x10 != 0;
                        assert_eq!(
                            touched, overlap,
                            "{bytes:?} pan {pan} big {powerup} duck {ducking} dx {dx} dy {dy}"
                        );
                        hits += usize::from(touched);
                    }
                }
            }
        }
    }
    assert!(hits > 0, "{bytes:?}");
}

#[test]
fn contact_boxes_overlap_exactly_when_collide_touches() {
    assert_contact_boxes_match_collide(ScreenBytes::Derived);
}

#[test]
fn contact_boxes_follow_stale_screen_bytes() {
    assert_contact_boxes_match_collide(ScreenBytes::Stale(7, 0xFB));
    assert_contact_boxes_match_collide(ScreenBytes::Stale(0xFA, 3));
}

#[test]
fn contact_boxes_follow_boss_screen_bytes() {
    assert_contact_boxes_match_collide(ScreenBytes::Boss);
}

#[test]
fn a_slot_box_right_of_and_below_mario_sits_on_its_screen_bytes() {
    let mut obj = ObjectSim::new(Vec::new(), 0x0200);
    obj.camera_position = crate::actor::Position {
        x: 0x0200,
        y: 0x0100,
    };
    obj.mario = MarioContact {
        x: 0x0200,
        y: 0x0100,
        ..Default::default()
    };
    let s = &mut obj.slots[0];
    s.b[B_STATE] = 2;
    s.b[B_TYPE] = 0x40;
    s.b[B_HB_LEFT] = 0xFA;
    s.b[B_HB_RIGHT] = 6;
    s.b[B_HB_TOP] = 0xF8;
    s.b[B_HB_BOTTOM] = 8;
    s.b[B_SCRX] = 150;
    s.b[B_SCRY] = 140;
    let b = obj.slot_contact_box(0, 0x0200, 0x0100).unwrap();
    assert_eq!((b.x0, b.y0), (150 - 6, 140 - 8));
}

#[test]
fn contact_box_skips_inactive_and_gated_out_slots() {
    let mut obj = ObjectSim::new(Vec::new(), 0x0200);
    obj.slots[0].b[B_TYPE] = 0x40;
    assert_eq!(obj.slot_contact_box(0, 0x0200, 0x0100), None, "inactive");
    obj.slots[0].b[B_STATE] = 2;
    assert!(obj.slot_contact_box(0, 0x0200, 0x0100).is_some());
    obj.slots[0].b[B_TYPE] = 0x80;
    assert_eq!(obj.slot_contact_box(0, 0x0200, 0x0100), None, "bare $80");
    obj.slots[0].b[B_TYPE] = 0x04;
    assert_eq!(obj.slot_contact_box(0, 0x0200, 0x0100), None, "no $63 bit");
}

#[test]
fn an_excluded_slot_box_sits_at_its_world_position() {
    let mut obj = ObjectSim::new(Vec::new(), 0x0200);
    obj.view_margin = (112, 36);
    obj.camera_position = crate::actor::Position {
        x: 0x0200,
        y: 0x0100,
    };
    let s = &mut obj.slots[0];
    s.b[B_STATE] = 2;
    s.b[B_TYPE] = 0x40;
    s.b[B_HB_LEFT] = 0xFA;
    s.b[B_HB_TOP] = 0xF8;
    s.b[B_SCRX] = 0x12;
    s.b[B_SCRY] = 0x34;
    set_x(&mut s.b, 0x0310);
    set_y(&mut s.b, 0x0100);
    assert!(obj.contact_excluded(0));
    let b = obj.slot_contact_box(0, 0x0200, 0x0100).unwrap();
    assert_eq!((b.x0, b.y0), (0x110 + 88 - 6, 88 - 8));
}

#[path = "visibility.rs"]
mod visibility;
