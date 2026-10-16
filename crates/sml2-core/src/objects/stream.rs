//! Object stream routines.

use super::*;

impl ObjectSim {
    pub(super) fn advance_stream(&mut self) -> bool {
        if self.stream_cursor_cleared {
            return false;
        }
        if self.stream_index + RECORD_LEN < self.records.len() * RECORD_LEN {
            self.stream_index += RECORD_LEN;
            return true;
        }
        false
    }

    pub(super) fn scan(&mut self, tables: &ObjectTables) {
        if self.stream_cursor_cleared {
            // Scanning physical $0000 is outside the supported source paths.
            self.note_unported(0x8C31);
            return;
        }
        let n = self.records.len();
        if self.stream_index >= n * RECORD_LEN {
            return;
        }
        let forward = self.scan_dir == 1;
        // Bounded (the engine's exits are the same; this only guards the
        // port against a malformed stream).
        for _ in 0..(n * 2 + 8) {
            let ri = self.stream_index / RECORD_LEN;
            if ri >= n {
                return;
            }
            let rec_x = self.records[ri].x;
            let edge = self.spawn_x;
            if forward {
                if rec_x >> 8 != edge >> 8 {
                    if rec_x > edge {
                        return; // past the edge
                    }
                    if !self.advance_stream() {
                        return;
                    }
                    continue;
                }
                if (rec_x & 0xFF) < (edge & 0xFF) {
                    if !self.advance_stream() {
                        return;
                    }
                    continue;
                }
                if (rec_x & 0xFF) != (edge & 0xFF) {
                    return; // between records
                }
            } else {
                // backward mirror: the engine walks the pointer down
                if rec_x >> 8 != edge >> 8 {
                    if rec_x < edge {
                        return;
                    }
                    if self.stream_index == 0 {
                        return;
                    }
                    self.stream_index -= RECORD_LEN;
                    continue;
                }
                if (rec_x & 0xFF) > (edge & 0xFF) {
                    if self.stream_index == 0 {
                        return;
                    }
                    self.stream_index -= RECORD_LEN;
                    continue;
                }
                if (rec_x & 0xFF) != (edge & 0xFF) {
                    return;
                }
            }
            // Record at the edge: the mode byte gates (and the engine's
            // post-spawn marker 2 blocks re-spawns on a later pass).
            let m = self.records[ri].mode_flag;
            let skip = if self.easy_mode {
                m & 0x0F != 0
            } else {
                m & 0x0E != 0
            } || self.spawned[ri];
            if skip {
                if !self.advance_stream() {
                    return;
                }
                continue;
            }
            let free = self.slots.iter().position(|s| s.b[B_STATE] == 0);
            let Some(slot) = free else {
                return; // no free slot: scan stops ($8C93 → $AF)
            };
            let r = self.records[ri];
            // $8CA8: 6 record bytes, record pointer (+2), state = 1, clear
            // +9..+31, marker 2, $8240 sprite derive, $81CF hitbox.
            let mut s = Staging {
                b: [0; 32],
                ..Default::default()
            };
            s.b[B_XHI] = (r.x >> 8) as u8;
            s.b[B_XLO] = r.x as u8;
            s.b[B_MARKER] = r.mode_flag;
            s.b[B_YHI] = (r.y >> 8) as u8;
            s.b[B_YLO] = r.y as u8;
            s.b[B_DEF] = r.anim_index;
            // The engine stores (streamPtrHi, streamPtrLo + 2) with no carry
            // into the hi byte.
            let a = self.stream_ptr();
            s.b[B_RECHI] = (a >> 8) as u8;
            s.b[B_RECLO] = (a as u8).wrapping_add(2);
            s.b[B_STATE] = 1;
            self.derive_sprite(&mut s, tables);
            self.refresh_hitbox(&mut s, tables, false);
            self.slots[slot].b = s.b;
            self.spawned[ri] = true;
            self.standby = self.standby.wrapping_add(1);
            // The last record leaves the pointer parked on itself (see
            // `advance_stream`): the engine's `$FF` terminator check refuses
            // the store, and the re-scan then walks into the same record with
            // the post-spawn marker set and stops there every frame.
            self.advance_stream();
        }
    }
}
