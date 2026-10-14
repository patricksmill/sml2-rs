# Level targets for the 12 pending capture items (offline resolution, 2026-10-14)

Method: joined every level's `spawns.records[].anim_index` through `anim_profiles_a/b`
(mode B when `sCurLevel >= $0A` — ROM `$EC4B`: `cp $0A / ret c`; the port's
`object.rs::sprite_map_index` now uses `>= 0x0A`; note the queue's audit text says "> 10",
which differs for level index 10 only) and counted handler classes not in the port's
`run_handler` match (13 handlers + `0x58100` fixture). Per-level unported counts reproduce
the queue's exactly (L1/L26/L27/L30 = 0 … L25 = 12). Scene maps were independently
re-derived from `baserom.gb` ([bank:$4002] pointer + layout_index*48): 0/32 mismatches.
Node indices from `extracted/map/nodes.bin` (first `$4218[node]` == level index).
All candidate nodes are reachable via `probe_level_select.py`: the A-press entry `$3C169`
(→ `$3C1A2`) does NOT test the `$6591` key mask (only the walk path `$3E1D7` does), so
non-`$00`-mask nodes (34, 41, 68) enter like the verified nodes 0/61.

| item_id | level index | level number | node index | evidence | confidence |
|---|---|---|---|---|---|
| autoscroll_capture | 25 | 26 | 4 | auto_scroll=1 (manifest level26; ROM `LevelPropertiesScroll` idx25=$01); `nodes.bin[4]=$19`=idx25; 0 unported handlers (anims 8/9/17/20/18 all ported). Candidates: L20 idx19 (4 unported, node 61), L26 idx25 (0), L31 idx30 (1). L26 is the cleanest; queue P3 planned L20 only because it co-covers scene+space. | high |
| scene_capture | 2 | 3 | 34 | level03.scene.bin cells i4→id7 (`40 16 FF`: teleport to Y-page1/X-page6) and i21→id6 (`40 03 FF`); `edge.bin[i4]=$0E`, `[i21]=$0D` (transition bits live); both $4x scenes → teleport observable in the existing x/y/camx/camy checks. 3 unported = tie with L24 idx23; L20's id 13 is an arena whose observables need the unported driver. The queue's own note also names level03 as cheapest. | high |
| space_physics_seeding | 28 | 29 | 68 | space_physics=8 (manifest level29; ROM `LevelPropertiesSpace` idx28=$08); `nodes.bin[68]=$1C`=idx28; 1 unported (anim 90 record only). Candidates: idx18 (3 unported), idx19 (4), idx28 (1). Item gate says "the level whose space_physics is 1" = L20 idx19 (node 61) — if that wording is binding, use L20; L29 is the fewest-unported candidate. | medium |
| scene_boss_spawn | 23 | 24 | 53 | Arena scripts ($2x, `$0EB0` arm) = scene ids 11-16 (on levels 6/10/14/17/20/24) + 19/27 (L25, no node). L24: scene cell i27→id15, script `64 20 30 58 60 40 02 FF`; `edge[27]=$0D`; 3 unported = cheapest of the six. `$8DFD`: def_index = sBossFightPhase + b(sCurLevel); sCurLevel $17 → b=5 → record [01 06 48] (real boss). Gate text "level 8s ids 11-16" matches no level's scene map — read as the arena ids 11-16. | medium |
| obj_on_object_capture | 31 | 32 | 122 | records anim46 → handler $58DD1 (ROM table A/B[46] = $04DD1+$54000); its fstate-0 init stages type $20 (`3E 20 / E0 E2` at $58DD1); $20 & $23 = $20 → bit5 set → `$2E1A` stand-on arm (needs y_delta < $0C, vvel < $18). 1 unported = tie with L31 idx30 (node 106): L31's 3 anim46 objects sit over $51 spikes at row31 in an autoscroll level; L32's 9 sit at y176-192 over open air/slopes. | medium |
| pushout_1413_capture | 2 | 3 | 34 | `$1413` needs body-in-solid-column: `$0C-$0F` side-pipe or `$04/$05` side-push tiles. level03 has $0C-$0F ×3 each = four side-pipe sites incl. (col65,row11)=x1040,y176 (one tile below the spawn row (46,164)); `$143D` arms on `tile & $FC == $0C`. 3 unported = tie with L24 idx23 ($04×14/$05×50 strips at row31 + $0C/$0D at col190); L3 preferred for the side-pipe arm (DESIGN's route). | medium |
| items_powerup_capture | 0 | 1 | 0 | `$8708` emergence: below-bonk of a type-$80 pose sets type = anim + $70. L1's `$01` item blocks (air below) carry $80 poses: rec2 anim17@(680,440)→$81 mushroom (small) / $82 fire (big, `cp $11` rewrite), rec16 anim18@(3016,440)→$82, rec9 anim20@(2440,440)→$84 star, rec8 anim21@(2424,440)→$85 1-up. `$2F72` withholds the $01 coin when a pose is in the bonk window. 0 unported; ties L26/L27/L30 (0) but L1 has the established traj17 route. | high |
| bonk_single_slot_draw | 0 | 1 | 0 | `$1993`: only `$01` allocates two slots; `$00`→2/1, `$03`→6/5, `$7E/$02`→3 are single-slot. level01 layout: `$00`×19 all row28, x576-3232 ("x >= 576" verified; air below), `$03`×4 at x3840-3952 = "the tower", all bonkable (below = $60/$61/$7A passable, < $38 above). traj15's two-slot bump was a `$01` (col 22). | high |
| bonk_big_mario_types_1_5 | 0 | 1 | 0 | Big-Mario arms: `$00`→type1, `$03`→type5 (`$19B0`/`$19C1`). Need: mushroom source + a $00/$03 bonk. L1 mushroom source = rec2's anim17 $01 block (x680) → collect → big, then bonk a $00 (row28, x>=576) / $03 (tower). Caveat: if the capture must also show the emergence ON the same $00/$03 bonk (title "(powerup emergence)"), the cheapest $00-carried powerup pose is L23 idx22 (rec22 anim20 above $00; 4 unported); L1's $00 at cols 149-153 sit ~8-24px from the star/1-up poses (may fall in the `$2F72` window; unverified offline). | medium |
| fireball_paths | 0 | 1 | 0 | Fire flower source = rec16's anim18 block (x3016, over $01) → $82 → collect → tier 3 (`$3467`); then shoot/observe `$2FED`/`$324F`. 0 unported. | high |
| fireball_type06_break | 0 | 1 | 0 | `$3415`: fireball probe `sCollisionTile == $06` → sfx + `$2C6C`(type 1) + bounce. level01 has $06 ground at row29, cols 211-220 (x3376-3520) — ~360px east of the fire-flower block (x3016), same route. | medium |
| carry_item_type06 | 0 | 1 | 0 | `$5834F` (anim2) init sets type `$06`; anim1/8 fallwalkers (type $56: `$56&$03=$02` → bit1 set/bit0 clear) stomped → `$85C4` → `$8633` `inc anim` → anim 2; then B-held touch → `$8611` → anim+2 = 4 (carried). L1 recs 1/4/14/15/21 are anim1 (x584/1368/2920/2976/3744), rec5 anim8. Queue's own note: L1's records reach "the anim 2 -> 3/4 transitions, which are the carry item's". | high |

## Notes / caveats

- **Shared-capture alternative (P3):** the queue planned ONE level-20 (idx 19, node 61)
  capture to close autoscroll + scene + space. Level 20 has 4 unported handler classes;
  the three cheapest targets above (L26/L3/L29) each beat it individually, but cost three
  runs instead of one. If the parent prefers the single L20 run, only the three P3 rows
  change (19/19/19, node 61).
- **Ties:** scene_capture & pushout: L3 ↔ L24 (both 3). obj_on_object: L31 ↔ L32 (both 1).
  items_powerup & bonk-single-slot-adjacent: L1 ↔ L26/L27/L30 (all 0; L1 chosen for the
  proven route and its own queue history).
- **Mode-rule note:** the audit's "> 10" differs from the ROM (`>= $0A`) only for level
  index 10 (L11): 5 vs 6 unported. No target above is index 10; L11 is not a candidate for
  any item's cheapest set.
- **Excluded (not level-property items):** `level_tail_capture` (extends traj17 on level 1),
  `overworld_map_state_0C` (map state, no level), `remaining_handler_classes` (per-level
  sweep; its plan starts with L26/L27/L30).
- **Cross-checks performed:** scene maps re-derived 0/32 mismatches vs baserom.gb;
  per-level unported counts reproduce the queue's published numbers; DESIGN's "level 1 has
  $00 cells at x >= 576, $03 only at the tower" verified against level01.decompressed.bin;
  ROM spot-checks: anim→handler table at $58040/$59C07 (anim17→$58712, anim46→$58DD1,
  anim2→$5834F), `$58712` init bytes `3E 80 E0 E2` (type $80), `$8708`/`$86E4` (`cp $11` +
  `add 112`), `$3E1D7` mask test + `$3C169`/`$3C1A2` A-press entry, `$8DFD` boss def_index.
- **Anonymous uncertainty:** the arena gate text ("level 8s ids 11-16") cannot be matched
  to any scene map; if the author meant a different level, L24 could be wrong — the six
  arena levels are 6/10/14/17/20/24 (+25 w/o node), and L24 is the cheapest of them.

## Appendix — per-level unported handler classes (recomputed, mode B when idx >= 10)

idx (level): 0(1)=0, 1(2)=4, 2(3)=3, 3(4)=6, 4(5)=10, 5(6)=6, 6(7)=7, 7(8)=9, 8(9)=4,
9(10)=5, 10(11)=6, 11(12)=4, 12(13)=5, 13(14)=6, 14(15)=4, 15(16)=7, 16(17)=4, 17(18)=5,
18(19)=3, 19(20)=4, 20(21)=7, 21(22)=6, 22(23)=4, 23(24)=3, 24(25)=12, 25(26)=0, 26(27)=0,
27(28)=2, 28(29)=1, 29(30)=0, 30(31)=1, 31(32)=1.

Node index of each target level: 0→0, 2→34, 23→53, 25→4, 28→68, 31→122. Level 25 (idx 24)
has no node (not used by any row).
