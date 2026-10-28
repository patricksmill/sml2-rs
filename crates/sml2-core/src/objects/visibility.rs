//! Enlarged-view activation bounds; collision and draw cameras stay native.
use super::ObjectSim;

impl ObjectSim {
    /// Enlarge a native screen-X reset interval using world coordinates.
    /// OAM X = world X - camera X + 80 + 8. Retain the source's padding,
    /// but move it to the displayed view edges and avoid 8-bit aliasing.
    pub(super) fn horizontal_reset_bounds(&self, screen_limit: u8) -> Option<std::ops::Range<i32>> {
        if self.view_margin == (0, 0) {
            return None;
        }
        let screen_zero = i32::from(self.camera_position.x) + i32::from(self.view_offset.0) - 88;
        let margin = i32::from(self.view_margin.0);
        Some(screen_zero - margin..screen_zero + i32::from(screen_limit) + margin)
    }

    /// [`Self::horizontal_reset_bounds`] for a screen-Y limit. OAM Y = world
    /// Y - (camera Y - 72 - pan) + 16; the margin follows the view's upward
    /// bias. Boss mode keeps the byte test: its screen bytes are not derived
    /// from world positions.
    pub(super) fn vertical_reset_bounds(&self, screen_limit: u8) -> Option<std::ops::Range<i32>> {
        if self.view_margin == (0, 0) || self.mario.scene_state & 0xF0 != 0 {
            return None;
        }
        let screen_zero = i32::from(self.camera_position.y) + i32::from(self.view_offset.1)
            - 88
            - i32::from(self.mario.pan & 3);
        let (margin, bias) = (i32::from(self.view_margin.1), i32::from(self.view_bias_y));
        Some(screen_zero - margin - bias..screen_zero + i32::from(screen_limit) + margin - bias)
    }

    pub(super) fn horizontal_view_window(&self, camera_x: u16, half: i32) -> (u16, u16) {
        let center = i32::from(camera_x) + i32::from(self.view_offset.0);
        ((center - half).max(0) as u16, (center + half).max(0) as u16)
    }

    pub(super) fn vertical_view_window(&self, camera_y: u16, half: i32) -> (u16, u16) {
        let center =
            i32::from(camera_y) - i32::from(self.view_bias_y) + i32::from(self.view_offset.1);
        // Clamp the endpoints, not the center: near world Y=0 a biased center
        // can be negative, and must not expand the bottom edge again.
        ((center - half).max(0) as u16, (center + half).max(0) as u16)
    }
}

impl ObjectSim {
    /// Mario's reference point as slot `slot` should see it. Handlers measure
    /// it with 8-bit screen-byte compares, which wrap at 128 px; an enlarged
    /// view keeps slots active far enough away that a spike 200 px off reads
    /// as 56 px and drops. There the point is pulled to within 127 px of the
    /// slot on the same side, so the compares read "far, this side". Native
    /// views and boss mode keep the engine's point.
    pub(super) fn slot_mario_ref(&self, slot: (u16, u16)) -> (u16, u16) {
        let refp = (self.mario_ref_x, self.mario_ref_y);
        if self.view_margin == (0, 0) || self.mario.scene_state & 0xF0 != 0 {
            return refp;
        }
        let pull = |r: u16, s: u16| {
            let d = (i32::from(r) - i32::from(s)).clamp(-0x7F, 0x7F);
            (i32::from(s) + d) as u16
        };
        (pull(refp.0, slot.0), pull(refp.1, slot.1))
    }
}
