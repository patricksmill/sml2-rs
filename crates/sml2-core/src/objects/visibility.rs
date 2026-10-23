//! Enlarged-view activation bounds; collision and draw cameras stay native.
use super::ObjectSim;

impl ObjectSim {
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
