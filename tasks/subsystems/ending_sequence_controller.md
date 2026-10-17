# Ending Sequence Controller (State 19, 20 Substates)

- **ROM addresses:**
  - Sequence entry & dispatcher: ROM `0x6800F` (`main.asm:39223..39225`), bank `$1A`, CPU `$400F`
  - Substate jump table: ROM `0x6801A` (`main.asm:39230..39250`), bank `$1A`, CPU `$401A`
  - Substates 0..3: CPU `$4520` (ROM `0x68520`)
  - Substate 4: CPU `$29DD` (ROM `0x029DD`, scene load)
  - Substate 5: CPU `$42EE` (ROM `0x682EE`)
  - Substate 6: CPU `$42BA` (ROM `0x682BA`)
  - Substate 7: CPU `$43BD` (ROM `0x683BD`)
  - Substate 8: CPU `$4149` (ROM `0x68149`)
  - Substate 9: CPU `$4139` (ROM `0x68139`)
  - Substate 10: CPU `$4398` (ROM `0x68398`)
  - Substate 11: CPU `$421E` (ROM `0x6821E`)
  - Substate 12: CPU `$41D2` (ROM `0x681D2`)
  - Substate 13: CPU `$41FD` (ROM `0x681FD`)
  - Substate 14: CPU `$4063` (ROM `0x68063`)
  - Substate 15: CPU `$4042` (ROM `0x68042`)
  - Substate 16: CPU `$4241` (ROM `0x68241`)
  - Substate 17: CPU `$418B` (ROM `0x6818B`)
  - Substate 18: CPU `$409E` (ROM `0x6809E`)
  - Substate 19: CPU `$4267` (ROM `0x68267`)
  - Ending VBlank service: ROM `0x68528` (`main.asm:39857..39892`), CPU `$4528`
- **Subsystem:** Ending presentation controller / State 19 sequence manager
- **Module:** `crates/sml2-core/src/ending_sequence.rs`

## 1. Sequence Overview

After Wario is defeated in his final form, Mario exits the castle and triggers the ending sequence.
The controller manages the entire 20-substate narrative progression:

| Substate | CPU | Responsibilities | Advance Condition |
|---|---|---|---|
| **0** | `$4520` | Blank frame wait | Single frame tick |
| **1** | `$4520` | Blank frame wait | Single frame tick |
| **2** | `$4520` | Blank frame wait | Single frame tick |
| **3** | `$4520` | Blank frame wait | Single frame tick |
| **4** | `$29DD` | Scene asset & palette load, init `credits_sprite_y = 44`, `fade_timer = 256` | Immediate |
| **5** | `$42EE` | Walk animation, sound update | `fade_timer == 0` -> sets `fade = 128` |
| **6** | `$42BA` | Walk anim, sound update, move credits Y on `fade & 3 == 0` | `fade_timer == 0` -> sets `fade = 32`, `raster_y = 13` |
| **7** | `$43BD` | Raster wobble, advance `raster_y` to 139 | `raster_y == 139` -> queues VRAM transfer 2 (`$5096 -> $9800`, 1024 bytes) |
| **8** | `$4149` | Raster wobble, wait for transfer completion | `!vram_transfer.active` |
| **9** | `$4139` | Raster wobble, wait for fade timer zero | `fade_timer == 0` |
| **10** | `$4398` | Raster wobble, retract `raster_y` down to 13 | `raster_y == 13` -> sets `fade = 192` |
| **11** | `$421E` | Sound update, wait fade | `fade_timer == 0` -> sets `fade = 384`, `ending_tile_flag = 0xFF` |
| **12** | `$41D2` | Sound update, burst sprite cycle, odd frame SFX1=24 | `fade_timer == 0` -> sets `fade = 64`, `sfx0 = 7`, `credits_y = 44` |
| **13** | `$41FD` | Sound update, secondary sprite ($EB) | `fade_timer == 0` -> sets `fade = 512` |
| **14** | `$4063` | Secondary sprite, sound update, wait transfer idle | Idle -> queues VRAM transfer 1 (`$4656 -> $8000`, 2048 bytes) |
| **15** | `$4042` | Secondary sprite, sound update, wait transfer idle | Idle -> sets `sfx0 = 6`, `fade = 64` |
| **16** | `$4241` | Set `credits_y = 44`, walking pair draw ($E8 + $EB), sound | `fade_timer == 0` -> sets `fade = 384` |
| **17** | `$418B` | Walking pair draw, sound, scroll background from fade timer | `scroll_y == 24` -> seeds text pointer `$56CE`, `phase = 4` |
| **18** | `$409E` | Dual LCDC split (scanline 111), walking pair, sound, wobble on fade threshold, text streaming | `credits_text.done` -> advances to 19; next page on `fade == 0` |
| **19** | `$4267` | Stable dual LCDC split display, walking pair draw, sound update | Terminal state (does not advance) |

## 2. VBlank Services (`$68528`)
On each frame:
1. Active VRAM transfer takes priority over text and tile flags (steps 64 bytes/frame).
2. Credits text streaming (`phase != 0`) renders 20 characters per frame into tilemap at `$9A20`.
3. Ending tile flag (`$A2E0 == 0xFF`) writes the 2x3 logo tiles (`0x89..0x8B` at offset 328, `0x99..0x9B` at offset 360) and clears the flag.
4. OAM presentation clears trailing cells per `$52E7`.
