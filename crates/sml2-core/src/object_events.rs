//! Consume object-engine writes to camera-owned WRAM after the object pass.

use crate::{camera::CameraSim, objects::ObjectSim};

impl ObjectSim {
    /// Sample player-owned WRAM immediately before the object pass.
    pub fn sample_player_inputs(&mut self, player: &crate::player_sim::PlayerSim) {
        self.mario_dead = player.dead;
        self.fade_timer = player.fade_timer;
        self.timer_low = player.level_timer.low;
        // `$58000` reads `sGoalBell` and clears every object's contact type
        // while it is set, so nothing hurts Mario after the goal.
        self.goal_bell = u8::from(player.goal_bell);
    }
}

impl ObjectSim {
    /// Sample camera-owned WRAM immediately before the object pass.
    pub fn sample_camera_inputs(&mut self, camera: &CameraSim) {
        self.boss_defeated = camera.boss_defeated;
    }
}

impl CameraSim {
    pub fn apply_object_result(&mut self, obj: &mut ObjectSim) {
        self.boss_defeated = obj.boss_defeated;
        if let Some(timer) = obj.camera_pan_set.take() {
            // Objects run after scroll and Mario draw: their new timer first
            // affects the next frame's scroll and draw phases.
            self.pan_timer = timer;
        }
        if let Some(phase) = obj.scene_phase_set.take() {
            self.boss_fight_phase = phase;
            self.scene_state = 0;
        }
        if obj.scroll_delta_right != 0 {
            self.scroll_delta_right = self.scroll_delta_right.wrapping_add(obj.scroll_delta_right);
            obj.scroll_delta_right = 0;
        }
        if obj.scroll_delta_left != 0 {
            self.scroll_delta_left = self.scroll_delta_left.wrapping_add(obj.scroll_delta_left);
            obj.scroll_delta_left = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_remaining_enemy_count_round_trips_through_the_object_pass() {
        // `sBossDefeated` is one byte: the camera's park reads it and the
        // object pass's `$581E2` removal counts it down.
        let mut cam = CameraSim::new(0, 0);
        cam.boss_defeated = 2;
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.sample_camera_inputs(&cam);
        assert_eq!(obj.boss_defeated, 2);
        obj.boss_defeated = 0;
        cam.apply_object_result(&mut obj);
        assert_eq!(cam.boss_defeated, 0);
    }

    #[test]
    fn the_goal_bell_reaches_the_object_engine() {
        let mut obj = ObjectSim::new(Vec::new(), 0);
        let mut player = crate::player_sim::PlayerSim::new();
        player.goal_bell = true;
        obj.sample_player_inputs(&player);
        assert_ne!(obj.goal_bell, 0);
        player.goal_bell = false;
        obj.sample_player_inputs(&player);
        assert_eq!(obj.goal_bell, 0);
    }

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

    #[test]
    fn scroll_delta_transfers_from_object_sim_to_camera_sim_and_clears_object() {
        let mut cam = CameraSim::new(80, 72);
        let mut obj = ObjectSim::new(Vec::new(), 0);
        obj.scroll_delta_right = 2;
        obj.scroll_delta_left = 1;

        cam.apply_object_result(&mut obj);
        assert_eq!(cam.scroll_delta_right, 2);
        assert_eq!(cam.scroll_delta_left, 1);
        assert_eq!(obj.scroll_delta_right, 0);
        assert_eq!(obj.scroll_delta_left, 0);

        obj.scroll_delta_right = 3;
        cam.apply_object_result(&mut obj);
        assert_eq!(cam.scroll_delta_right, 5);
        assert_eq!(obj.scroll_delta_right, 0);
    }
}
