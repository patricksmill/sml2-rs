# Original-game completion inventory

Implementation, verification, and normal-play integration are independent
claims. `pending` and `unaudited` rows are not completion evidence. Scope and
acceptance follow [the completion workflow](plans/2026-10-15-rust-completion-workflow.md).

| ID | Source (ROM; bank) | Rust entry | Evidence | Normal-play path / remaining gap |
|---|---|---|---|---|
| camera-pan | $2062 (home), $54E9 ($01), $C090 ($03) | `CameraSim::update_player_pan` | `pan_draw_*`, `pan_timer_*` tests | Shared timing integrated; positive Wario producers and fuller flutter still pending. See [contract](../tasks/subsystems/camera_pan_timer.md). |
| wario-20 | $A8AB ($02), state table $A90E | pending | [COMPLETE static brief](../tasks/handlers/0x0A8AB.md) only | Pending helpers, all handler states, object-result handoff and normal boss entry. Static traversal does not establish dynamic state closure. |
| wario-reverse-picker | $B1D6/$B1E4, tables $B20E/$B217 ($02) | `ObjectSim::boss_reverse_picker_b1d6` / `boss_reverse_picker_b1e4` | Three `helper_0x0b1*` byte tests, offset-mutation failure; 285 core tests pass | Verified prerequisite profiles and screen-Y consumer; Wario callers pending. |
| wario-child22 | $AB30/$AB4D ($02) | `ObjectSim::boss_anim22` | Seven `handler_0x0ab30_*` tests incl. boss dispatch/death freeze; parity/dispatch mutations fail; 292 core tests and frozen sweep pass | Boss dispatch integrated; produces `$AFD0` and counted removal. Normal child creation and Wario wait consumer remain pending in parent tasks. |
| wario-child25 | $AB6E/$AB4D ($02) | `ObjectSim::boss_anim25` / `wario_child_fall_ab4d` | Five `handler_0x0ab6e_*` tests incl. dispatch/reset; sprite mutation fails; 297 core tests pass | Boss dispatch integrated; shared child22/25 fall path. Normal creation and Wario wait consumer pending in parent tasks. |
| boss-flash | $B40E ($02), consumers $9443/$9CAC | `ObjectSim::boss_flash_b40e` | Two exhaustive helper tests and four consumer tests; XOR/old-callsite mutations fail; 310 core tests pass | Stun and morph now toggle flash, preserve sprite, and retain terminal restore/clear behavior. Wario caller pending. |
| wario-23 | $AB8B ($02) | pending | none yet | Depends on shared Wario helpers and camera producer handoff. |
| wario-sprite-cycle | $B339/$B356 ($02) | `ObjectSim::boss_sprite_cycle_b339` | Three `helper_0x0b339_*` tests, gate-mutation failure; 282 core tests pass | Verified helper; Wario callers pending. No dispatch arm of its own. |
| object-dispatch | $58040/$59C07 ($16), $92E8 ($02) | `ObjectSim::dispatch_handler` | `tools/handler_coverage.py`: 17/100 spawn targets; 18 boss targets pending after anim22/25 (initially 20) | Pending implementation and fixed-point audit of anim changes, children and scene objects. Counts are declared bodies, not verified behavior. |
| level-countdown | $5D6A..$5E11 ($01), load $04F1 (home) | `LevelTimer::step`, `PlayerSim::update_level_timer` | Seven core timer/BCD tests; three runtime tests; threshold/shared-flag/pause mutations fail; 304 core and 8 runtime tests pass; frozen sweep passes | Play/replay frame heads and reload derive the timer from header. Music requests are modeled; sound-driver acknowledgment and HUD digits remain pending presentation/audio work. Wario attack consumer pending. |
| moon-physics | $1090/$14FA/$1541/$181C/$2E7C (home), $5354/$53CF/$54FB ($01) | `PlayerSim` | existing tests require branch audit | Queue `moon_physics_remaining_arms` pending. |
| boss-flow | $9642/$9698/$B14E ($02), $02AD (home) | `ObjectSim`, `PlayerSim` | existing tests require source audit | Queue arena timing, anim-4/5 chain and ending handoff pending. |
| overworld | $3F04D/$3F07E and $3D0FF..$3D880 ($0F) | `MapSim` | existing tests require loader audit | Zone screen loaders pending; full node/entrance matrix unaudited. |
| level-route-matrix | all 32 manifests; source entrances and exits | level/scene/object simulation | pending matrix | Level entries, alternate routes, mechanics, bosses and exits must each acquire source-derived scenarios. |
| menus-save-flow | boot/title/file-select/pause/game-over | unaudited | pending | Source dispatch inventory, new/load/update/delete persistence and ordinary boot scenarios required. |
| minigames-ending | source minigames/final castle/ending/credits | unaudited | pending | State transitions, inputs and completion scenarios required. |
| visual-audio | source display and sound drivers | renderer seams / audio unaudited | Historical visual checks are scoped in `DESIGN.md`; no full-game claim | Native pixel comparison, deterministic audio and output evidence required. |
| final-integration | original game | extraction + runtime | pending | Fresh extraction/build, normal boot through final completion and save/reload; frozen regression gate must hold. |

Preserve asset IDs, extracted data, fixed simulation ticks and renderer seams
from `DESIGN.md`. Texture packs, interpolation and DX work are outside this goal.

Wario dependency audit: anim21/24 reads hardware DIV at `$AB1D` (bit0),
not the object frame counter. Source init does not reset DIV; no DIV writer was
found. A native sampled-divider producer is pending; retain explicit input
semantics, and test both branches. Wario20 state7 (`$A952`) sets state11 only
after the falling-child latch; it is shared by Wario23 whose state table has
11 entries. Wario20's own nine-entry table needs a reachability audit before
claiming that branch is reachable there.
