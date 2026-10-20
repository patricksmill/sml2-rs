//! Cursor motion shared by `$3E2F7` and `$3E3BA` (bank 15).

impl super::MapSim {
    /// `$3E426`: one pixel per call, called 48 times during a door transition.
    pub fn transition_walk_step(&mut self, direction: u8) {
        self.walk_frame = self.walk_frame.wrapping_add(1);
        self.cursor_pose = match direction {
            1 => {
                self.cursor_x = self.cursor_x.wrapping_add(1);
                4 + ((self.walk_frame & 6) >> 1)
            }
            2 => {
                self.cursor_x = self.cursor_x.wrapping_sub(1);
                (self.walk_frame & 6) >> 1
            }
            4 => {
                self.cursor_y = self.cursor_y.wrapping_sub(1);
                8 + ((self.walk_frame & 12) >> 2)
            }
            8 => {
                self.cursor_y = self.cursor_y.wrapping_add(1);
                12 + ((self.walk_frame & 6) >> 1)
            }
            _ => return,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::MapSim;

    #[test]
    fn door_walks_use_rom_distance_wrapping_and_directional_poses() {
        // Door setup bytes at $3D0D7, $3D760, $3D6C3 and $3D5EC.
        for (x, y, dir, end_x, end_y, pose) in [
            (48, 158, 4, 48, 110, 8),
            (12, 112, 1, 60, 112, 4),
            (129, 137, 2, 81, 137, 0),
            (75, 242, 8, 75, 34, 12),
        ] {
            let mut map = MapSim::new();
            map.cursor_x = x;
            map.cursor_y = y;
            map.walk_frame = 0;
            for _ in 0..48 {
                map.transition_walk_step(dir);
            }
            assert_eq!(
                [map.cursor_x, map.cursor_y, map.walk_frame, map.cursor_pose],
                [end_x, end_y, 48, pose]
            );
        }
    }
}
