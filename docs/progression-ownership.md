# Progression ownership across screens

Source map after the first native modularity slices, 2026-10-23. This records
current behavior, including asymmetric copies and reset rules; it does not
propose making those rules uniform. Paths are relative to the repository root.

## Writers, consumers, and lifetime

| Boundary | Authoritative writer and actual call site | Consumer | Reset and carry-over contract |
|---|---|---|---|
| Map to level | `MapSim::enter_check` emits `MapEntry`; the Overworld arm of `Session::advance` calls `LevelContext::from_content` and stages `MapTransition` | The pending level becomes `SessionState::InLevel` at `live_at` | Constructor initializes local state; entry copies the map's full progress array and session tally/walk timer. `seed_level_state` runs when the staged level becomes live. |
| Live level progression | `PlayerSim::exit_fade_step`, `apply_object_result`, and `map_handoff` update tally, reward/progress state and the return outcome | `LevelContext::step_frame` caches the first handoff; the InLevel arm of `Session::advance` consumes it | `map_handoff_done` prevents duplicate handoffs. The goal-bell path returns early; it does not immediately run the ordinary progress/save path. |
| Death and map return | `PlayerSim::death_arc` and `fade_step`; level tick calls `map_handoff` after the death fade | `Session::advance` constructs `MapSim::from_player`, loads the target screen/node and sets the map arrival latch | Non-game-over death copies lives and resets the session powerup; map conversion carries the player's progress and coin fields. Do not replace this with the clear branch's broader session copy. |
| Clear and map return | `PlayerSim::map_handoff` selects node, progress writes, save count and `saved_node` | `Session::advance` copies player stats, then constructs a map, bell room or castle reveal | Clear copies lives, both coin bytes, tally and powerup. Boss reveal retains `castle_progress` and the node seen at save time. A map-arrival dead latch is distinct from a failed clear. |
| Game over | `PlayerSim::game_over_enter`, called by the InLevel arm after the fade | `SessionState::GameOver` and its save record | Applies the tally penalty, resets money bag/powerup at the session boundary, retains the game-over player image and emits a save. This is a separate path from ordinary death. |
| Local player reload | `PlayerSim::respawn` | Player movement, camera and object interactions | Resets position, velocity, death/exit/pipe flags, bump slots and projectiles. Preserves fields the source does not clear, including map node and flutter state. This method is not the complete session map-return protocol. |
| Save emission | `Session::save_record` selects the active state's encoder; `handoff_save_record` substitutes the node observed by the save | `SaveSlots::write_slot`, then `FrameEffects::saves` | Save effects contain records captured at the transition. Consumers must use those bytes rather than encode later state. Progress records are not full simulation snapshots. |
| Save loading | `Session::apply_save_record` validates, uses `MapSim::apply_save_record`, then runs the return-node path through a temporary `PlayerSim` | Newly constructed Overworld state and session stats | Restores saved lives, coins, tally, money bag and progress. Explicitly zeros flutter timer/latch/charge because the record contains none of them. |

The immediate `Session::begin_level` helper skips the normal map entry delay
and supplies node zero. It is useful for acceptance scenarios, but it must not
be treated as proof that the normal map-to-level transfer copies every field.

## State carried outside the active player

| Fields | Current owner / transfer | Consequence for a future extraction |
|---|---|---|
| Tile-animation clock, scroll enables | `Session::remember_level_state` reads active level/demo state; `seed_level_state` restores it | Moving a level object must preserve these calls around screen replacement. |
| Object update clock | `session/object_clock.rs`: `ObjectClock::remember`, `observe`, `initialize_level` | New levels continue the established object cadence, rather than receiving an arbitrary fresh phase. |
| Flutter timer, armed flag, charge | Level's `PlayerSim` while playing; session copies on exit and restores through `seed_level_state` | Carry through clear/death/castle entry, but clear on save load. They are not purely temporary movement state. |
| Money-bag counter | `ObjectSim::money_bag` during play; `Session::advance` observes it and seeds future levels | A player-only progress struct would miss this producer. Game over clears the session value. |
| Map working RAM | Session retains `MapRam`; level metatiles seed/update it, including tileset swaps | Replacing a graphics or level buffer may affect later map behavior. Keep this compatibility transfer explicit. |
| Progress bytes and coin purse | Active `PlayerSim`/`MapSim`, with `MapSim::from_player` and `to_player` conversions | These conversions copy specific fields; neither is a complete session snapshot. |

## Source entry points and verification

- [Session transitions and save effects](../crates/sml2-runtime/src/session.rs):
  `advance`, `seed_level_state`, `apply_save_record`, `handoff_save_record`.
- [Ordered level tick](../crates/sml2-runtime/src/session/level.rs):
  `LevelContext::from_content`, `step_frame`, `is_finished`.
- [Player progression and reload](../crates/sml2-core/src/player_sim.rs):
  `exit_fade_step`, `map_handoff`, `game_over_enter`, `respawn`.
- [Map conversion and record encoding](../crates/sml2-core/src/map_sim.rs):
  `from_player`, `to_player`, `apply_save_record`, `save_record`.
- [CLI effect consumer](../crates/sml2-runtime/src/host.rs):
  `SessionRunner::step_frame` writes each emitted record before returning.

Existing tests pin these boundaries:

- `player/tests/simulation.rs`: `map_handoff_marks_a_plain_level_cleared`,
  `map_handoff_bell_path_is_the_castle_branch_and_returns`,
  `map_handoff_rewrites_the_node_for_the_secret_exit`,
  `map_handoff_tail_saves_once_for_every_node_class`.
- Runtime `src/tests/session.rs`:
  `hub_navigation_to_level_1_entry_and_clear_bell_return`,
  `in_level_hazard_death_and_fade_returns_to_hub_with_decremented_lives`,
  `a_death_at_zero_lives_runs_game_over_to_the_title`,
  `camera_scroll_enables_survive_death_and_map_reentry`,
  `the_enemy_counter_carries_across_levels_and_into_the_save`.
- Runtime `tests/session_boundary.rs`:
  `advance_emits_save_bytes_and_consumes_the_supplied_div_sample`.
- Runtime `src/tests/castle_flow.rs`:
  `castle_door_level_keeps_the_flutter_bytes`.

## Next bounded ownership change

A candidate is the three session-held flutter fields: group only the session's
carry-over copy into a private value with explicit capture, restore, and
save-load-reset operations. Keep the public `PlayerSim` fields and their wire
representations stable. Before implementing, inventory debug and constructor
access as well as the transition writers above; add a clear/death/save-load
round-trip case if the existing tests do not cover a transfer. This is a future
ticket, not an implemented change or a general progression-state redesign.
