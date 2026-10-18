# Native prerequisite contract: ROM $5B2BF -> $863C/$8240

Source contract for the remaining native record-reset prerequisite; this is not an implementation claim. Existing COMPLETE 0x5917A packet already includes all helper instructions; another generated packet was unnecessary.

## Conclusive source contract

Source witnesses: main.asm:34445–34457 ($5B2BF), home.asm:7608–7615 ($3AA0), main.asm:3070–3075 ($863C), main.asm:2485–2514 ($8240).

1. Read actor's complete 16-bit spawn_record, subtract two with u16 wrap, read FIVE consecutive physical memory bytes. These become actor X high, X low, marker, Y high, Y low. No validity/high-byte test guards this copy.
2. Preserve actor flip into global spawn_attr, invoke sprite rederive. If ORIGINAL spawn_record high >=$AD, rederive returns immediately: actor sprite and bytes14..30 remain unchanged, spawn_attr remains actor flip. Otherwise resolve current definition through selected mode sprite map, set actor sprite, preserve actor flip by copying spawn_attr back, clear actor bytes14..30, set spawn_attr=0. Definition, spawn_record, screen bytes, defeat_mode (+12), visibility_mask (+31) survive.
3. Set done=1, then actor activity=1. No writes to definition, active or standby counters. Caller anim54 has already decremented definition to53.

## Counters: leave both unchanged

This is not an inferred bookkeeping preference. The source helper and complete call chain above never touch either counter. main.asm:2369–2410 ($818D pipeline) calls bank16 update, draw, clears interaction flag when requested, calls $81BA hitbox refresh then stage-to-slot copy. No counter adjustment follows the helper. main.asm:2318–2358 ($8131 loop) chooses active/standby path before calling $818D; after $818D it jumps directly to $816A slot-pointer advancement. It does not reclassify the just-reset slot.

Actual counter mutation owners are separate paths: $81F7 main.asm:2436–2482 promotion (standby--, active++); $828F main.asm:2536–2573 explicit demotion (active--, standby++); $5B2D7 main.asm:34460–34509 removal (select one count based on old activity); allocation entries increment standby. Global named-counter reference search contains no hidden draw/pipeline owner.

Rust mod.rs:1040–1082 reproduces promotion/demotion counters and 1100–1167 active_update only runs handler, projects when activity2, clears flags, refreshes hitbox, copies staging. Thus retain counters exactly, even though source next-frame promotion can later change them again. Do not normalize a surprising source count.

Existing walkers.rs:326–341 respawn_object_5b2bf changes active/standby, omits done, misrestores marker, and skips physical copy for dynamic pointers. Those are legacy parity defects. helpers.rs:18–34 also omits observable spawn_attr staging/clearing. Neither is a ready native contract.

## Stream data is a physical projection of typed live state

Typed record definition: level.rs:214–233; serializer fields are X, mode_flag/marker, Y, anim_index. Initial raw field mirrors extraction (sml2-extract/src/main.rs:134–169), but is not live authoritative memory: record_set_anim modifies anim_index without raw, while spawned is a separate bool. Do not take raw[0..5] as current WRAM blindly.

Source allocation main.asm:3387–3429 ($8CA8) copies six record bytes to slot, stores pointer high and LOW+2 WITHOUT CARRY, initializes slot, writes value2 to the physical record marker. Therefore later reset normally restores marker2, not original mode_flag0/1. Native stream.rs:94–120 copies original mode_flag into slot but only sets spawned[ri]=true; records[ri].mode_flag/raw retain original values. Native record_index divides/floors the pointer distance, so it does not prove physical alignment.

Exact compatibility projection at adapter boundary:

- $AB00..$ACFF stream buffer is zeroed by $EC17 main.asm:9001–9026; first six bytes $AB00..$AB05 become $FF. $EBBB main.asm:8934–9000 writes expanded records at $AB06 and $EBB6 main.asm:8928–8932 writes two $FF terminal bytes. Serialize live typed record fields into these physical locations, incorporating current mutable marker state. Remaining untouched buffer bytes are zero for a fresh loaded buffer; do not invent zeros for addresses outside this proven buffer.
- Source pointer arithmetic edge: physical record start $ACFE -> stored pointer $AC00 (not $AD00) -> reset reads $ABFE..$AC02. This is index84, the FIRST aligned six-byte record whose low+2 overflows; its six bytes also exceed the $AB00..$ACFF buffer. Thus valid expanded records contained in that buffer cannot hit this low-byte-wrap case. Existing synthetic stream_cursor tests use85 records, so adapters must either support that compatibility input precisely or reject unsupported projection explicitly; do not claim it is a demonstrated ROM-level path. Arbitrary actor pointer $AB00 would similarly read $AAFE; that address is outside the proven buffer and needs actual modeled memory or an explicit unresolved-input result. The source copy itself has no no-op fallback.
- Current source/native mutable-marker mismatch matters beyond marker2: source remove-mode0 writes zero or $10 (when staged marker=$10) to pointed byte; native remove_object only clears spawned bool. `spawned == false` cannot distinguish original easy-only1 from subsequently-cleared0, or preserve spent$10. A complete live physical projection requires a typed current marker per record (or equivalent representation) with lifecycle producers wired, not a fabricated mapping `spawned ? 2 : original_mode_flag` claimed as universally exact. That simplified mapping is exact only under an explicitly tested restricted history.

## Dynamic pointers must still copy five bytes

Spawner $5B36D stores parent slot BASE address $AD00..$AEE0, not a record pointer+2 (source main.asm:34588–34599; typed handler_0x5b36d.rs:31–58). Consequently $5B2BF reads the two bytes preceding the parent slot and the parent's first three bytes:

- Parent $AD20: bytes $AD1E,$AD1F,$AD20,$AD21,$AD22 = previous slot reserved30/visibility31 and parent's X high/X low/marker. Restored X = previous reserved30/visibility31; restored marker = parent X high; restored Y = parent X low/marker. This odd layout is literal source behavior.
- Parent $AE00: same pattern across page boundary, previous slot $ADE0 trailing bytes.
- Parent $AD00: reads $ACFE,$ACFF from the stream buffer, followed by slot0 bytes0..2.

Adapter may serialize existing slots using Actor::to_raw_slot to project these addresses, keeping all address math outside domain logic. Read the committed slot table as it existed before staged handler copyback; do not substitute mutated staging when parent=self. Source $5B2BF reads physical WRAM, not staging.

For high >=$AD, sprite rederive is skipped *after* position copy. Native domain must preserve every actor field except position/marker/activity, set done, and return retained spawn_attr=actor flip. No early-return/no-op based on dynamic origin is valid.

## Reachability restriction is NOT proven

Known ordinary anim54 producer is anim53's contact82 nonspin stomp (prior report). Native anim53 does not inspect or restrict spawn_record; it can operate with either stream or parent provenance. A bounded immediate search found no `ld a,53` dynamic-anim assignment (the two literals53 found write sprite), but this is not proof against computed/shared spawn requests, definition transformations, alternate entrances or custom table data. Therefore a handler ticket cannot exclude >=$AD merely because current fixtures happen to be stream-backed. A valid aligned source record fully contained in the stream buffer does permit aligned record lookup, but this audit did not census ROM records or prove every native fixture obeys that restriction. Oversized synthetic streams have the low-byte-wrap edge described above.

No full dynamic producer closure audit or ROM level record-index census was performed; these are remaining uncertainties, not claims of reachability or unreachability. Do not gate dynamic position-copy behavior behind an unproved condition.

## Minimal typed API and ownership

Suggested domain shape (names negotiable):

    struct SpawnOrigin { position: Position, marker: u8 }
    enum SpriteReset { Rebuild { sprite: u8 }, Preserve }
    struct ResetResult { spawn_attr: SpriteFlipFlags } // done=true is unconditional
    fn reset_to_spawn(actor: &mut Actor, origin: SpawnOrigin,
                      visuals: SpriteReset) -> ResetResult

This helper receives already-resolved origin bytes as domain Position/marker and selected sprite behavior, restores position/marker, executes typed reset fields, sets activity1. Rebuild resets explicit named fields14..30, including private reserved29/30, so implement as Actor child module. Return cleared spawn_attr for Rebuild, actor flip for Preserve. Adapter sets done=1 and result spawn_attr; counters untouched. Neither raw address, slot offsets nor tables belong in the handler/domain method. Handler reports a `ResetToSpawn` effect; adapter resolves context before invoking reset.

Boundary resolver uses actor.spawn_record only in adapter: map actual five physical bytes through typed live records/current markers and committed-slot projection, then build SpawnOrigin. Select SpriteReset from original pointer high; resolve sprite via ObjectTables::sprite_for only for rebuild branch. Missing sprite or unsupported memory must produce explicit failure before mutating actor/definition/global outputs, never `.unwrap_or(0)` or invented no-op. Production completeness requires proving that supported resolver memory covers every reachable input.

Avoid prematurely changing generic finish_sprite_swap and all consumers. Provide a focused new boundary for this prerequisite, then migrate respawn_object_5b2bf only after domain/projection acceptance. A separate future ticket can migrate all $8240 callers.

## Exact bounded files and ticket split

Live marker ownership is ready. The remaining work is the bounded storage projection and native reset operation below.

A. **Live marker ownership is implemented:** `SpawnRecord.mode_flag` now carries the current marker. Allocation copies the old marker into the actor, then writes record marker2; mode0 removal writes0 or spent16, and modes1/2 retain it. Existing `initial_records` restoration and fresh Wario stream replacements own reset. The `raw` field remains an extraction snapshot. Build projection from live typed fields; do not introduce a parallel marker vector. Six focused tests, four sensitivity mutations and the full workspace/frozen gates verify this prerequisite.

B. **After projection readiness, reset prerequisite:** actor.rs (module registration only), new actor_spawn_reset.rs, new objects/record_reset.rs, new objects/record_reset_tests.rs; objects/mod.rs (module registration only); walkers.rs (respawn_object_5b2bf function only). If caller adapter names remain stable, existing five handler adapters need no edits. No counter or broad engine migration.

Focused tests: literal full32-byte reset (all initial bytes unique), marker2 source lifecycle, exact current sprite, high $AC/$AD threshold, high>=AD odd five-byte copy including AD00/AD20/AE00 and parent=self committed snapshot, low+2/no-carry crossing, two-FF sentinel/zero-tail projection, source removal marker0/$10 lifecycle, done/spawn_attr for both branches, preserved defeat/visibility, unchanged counters including wrap-edge initial counts. Real-step integration checks direct reset helper output separately from later hitbox refresh (outer refresh overwrites bytes19..22 after reset), plus next-frame promotion's independent count changes. Missing-context failures must preserve inputs, and sensitivity checks must reject legacy counter changes, stale mode_flag restoration, missing done and skipped dynamic copy.
