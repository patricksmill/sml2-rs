//! Consume object-engine writes to camera-owned WRAM after the object pass.

use crate::{camera::CameraSim, objects::ObjectSim};

impl ObjectSim {
    /// Sample player-owned WRAM immediately before the object pass.
    pub fn sample_player_inputs(&mut self, player: &crate::player_sim::PlayerSim) {
        self.mario_dead = player.dead;
        self.fade_timer = player.fade_timer;
        self.timer_low = player.level_timer.low;
    }
}

impl CameraSim {
    pub fn apply_object_result(&mut self, obj: &mut ObjectSim) {
        if let Some(timer) = obj.camera_pan_set.take() {
            // Objects run after scroll and Mario draw: their new timer first
            // affects the next frame's scroll and draw phases.
            self.pan_timer = timer;
        }
        if let Some(phase) = obj.scene_phase_set.take() {
            self.boss_fight_phase = phase;
            self.scene_state = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pan_events_preserve_creation_frame_and_drive_next_frame_phases() {
        for seed in [0x34, 0x38, 0] {
            let mut cam = CameraSim::new(80, 72);
            cam.pan_timer = 3;
            cam.update_player_pan(72, false, true);
            assert_eq!((cam.scroll_pan, cam.player_pan, cam.scroll_y), (3, 2, 253));
            let mut obj = ObjectSim::new(Vec::new(), 0);
            obj.camera_pan_set = Some(seed);
            cam.apply_object_result(&mut obj);
            assert_eq!(cam.pan_timer, seed);
            assert_eq!(obj.camera_pan_set, None);
            assert_eq!((cam.scroll_pan, cam.player_pan, cam.scroll_y), (3, 2, 253));
            // A consumed event cannot re-arm the timer after another writer.
            cam.pan_timer = seed.wrapping_add(1);
            cam.apply_object_result(&mut obj);
            assert_eq!(cam.pan_timer, seed.wrapping_add(1));
            cam.pan_timer = seed;
            cam.update_player_pan(72, false, true);
            assert_eq!(cam.scroll_pan, seed & 3);
            assert_eq!(cam.scroll_y, 0u8.wrapping_sub(seed & 3));
            assert_eq!(cam.pan_timer, seed.saturating_sub(1));
            assert_eq!(cam.player_pan, seed.saturating_sub(1) & 3);
        }
    }

    #[test]
    fn scene_phase_event_sets_phase_and_releases_scene_once() {
        for phase in [0, 1] {
            let mut cam = CameraSim::new(80, 72);
            cam.scene_state = 0xF0;
            cam.boss_fight_phase = 7;
            cam.pan_timer = 0x11;
            let mut obj = ObjectSim::new(Vec::new(), 0);
            obj.scene_phase_set = Some(phase);
            cam.apply_object_result(&mut obj);
            assert_eq!((cam.boss_fight_phase, cam.scene_state), (phase, 0));
            assert_eq!(cam.pan_timer, 0x11);
            assert_eq!(obj.scene_phase_set, None);
            cam.boss_fight_phase = 9;
            cam.scene_state = 0xA0;
            cam.apply_object_result(&mut obj);
            assert_eq!((cam.boss_fight_phase, cam.scene_state), (9, 0xA0));
        }
    }

    #[test]
    fn absent_events_preserve_camera_and_level_reset_clears_pending_writes() {
        let mut cam = CameraSim::new(80, 72);
        cam.pan_timer = 0x27;
        cam.scroll_pan = 3;
        cam.player_pan = 2;
        cam.boss_fight_phase = 5;
        cam.scene_state = 0xF0;
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.camera_pan_set = Some(0x38);
        obj.scene_phase_set = Some(1);
        obj.reset_level_start();
        assert_eq!((obj.camera_pan_set, obj.scene_phase_set), (None, None));
        cam.apply_object_result(&mut obj);
        assert_eq!(
            (cam.pan_timer, cam.scroll_pan, cam.player_pan),
            (0x27, 3, 2)
        );
        assert_eq!((cam.boss_fight_phase, cam.scene_state), (5, 0xF0));
        assert_eq!((cam.x, cam.y, cam.scroll_x, cam.scroll_y), (80, 72, 0, 0));
    }
}
