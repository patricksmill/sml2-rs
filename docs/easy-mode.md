# Easy mode (`sEasyMode`, `$A2E4`)

Audited 2026-10-21 against every `ld a,[sEasyMode]` (`FA E4 A2`) and
`ld [sEasyMode],a` (`EA E4 A2`) in `baserom.gb`: 14 reads, 3 writes. SELECT on
the file select toggles it; it is session state, not saved. Each read has a
unit test that a sensitivity mutation of the port's branch fails. Addresses
are ROM offsets.

## Reads

| ROM | Routine | Effect | Port | Test |
|---|---|---|---|---|
| `$357F` | home `$3573` (state `$16`) → `$35CD` (`$17`) | Before every level (castle visits excepted), copy bank 26 `$4656..$4955` to `$8800`, turn the background off and show sprite 240, "EASY MODE", at (52, 80) for 64 frames; the load then continues at `$0B` | `easy_mode_card`, `MapTransition` | `the_card_draws_easy_mode_in_one_row_at_its_rom_position`, `easy_mode_entry_shows_the_card_before_the_level`, `easy_mode_castle_door_shows_the_card_before_the_wario_level` |
| `$8C78` (forward), `$8D27` (backward) | bank 2 object stream | A record at the edge spawns when its mode byte `& $0F` is clear (`& $0E` without Easy), so Easy-only records appear | `objects/stream.rs` (both directions share the gate) | `record_marker_removed_easy_only_record_respawns_in_easy_and_spent_marker_survives` |
| `$AC9F` | bank 2 Wario phase-2 exit | The next form's animation is 29 instead of 27 | `bosses/wario/phase2_exit.rs` | `helper_0x0ac55_native_clear_boundary_and_difficulty_bit_zero` |
| `$5AB0F`, `$5AB21` | bank `$16` launched shell (the short one shares it) | Each leg moves 2 px per update instead of 4 | `enemies/launched_shell.rs` | `handler_0x5aab9_easy_mode_halves_the_step_and_the_fall_plays_sfx` |
| `$5AC17` | bank `$16` defeat fade | On expiry, remove and count the boss | `effects/defeat_fade.rs` | `handler_0x5ac01_easy_mode_removes_and_counts_the_boss` |
| `$5AC90`, `$5ACA3` | bank `$16` growing block | Easy sprite 224 at init; growth state 3 instead of 2 | `platforms/growing_block.rs` | `handler_0x5ac7d_init_sets_contact_and_easy_sprite`, `handler_0x5ac7d_stood_on_selects_the_growth_state` |
| `$5AD88`, `$5ADB4` | bank `$16` pacing platform | Each leg moves only on even frames | `platforms/pacing.rs` | `handler_0x5ad4d_easy_odd_frames_only_bob` (right), `handler_0x5ad4d_easy_left_leg_moves_only_on_even_frames` (left) |
| `$30067`, `$3008E` | bank 12 file select draw | The Easy banner (sprite 241); cursor frames 32 ids later | `front_end/render.rs` | `the_easy_banner_draws_at_its_rom_position` |
| `$30382` | bank 12 file select input | SELECT toggles it, except in erase mode | `front_end/file_select.rs` | `select_toggles_easy_mode_but_not_in_erase_mode` |

## Writes

`$30054` clears it as the file select loads, `$30389` stores the SELECT
toggle, and `$302A9` clears it when erase mode is armed
(`arming_erase_mode_clears_easy_mode`).
