//! Raw slot and legacy world effects for ROM $589A6.
use super::{
    handler_0x589a6::{self, World},
    terrain_probes::TerrainProbes,
    *,
};
use crate::actor::{Actor, FrameTick, Position};

struct Inputs<'a> {
    sim: &'a mut ObjectSim,
    probes: TerrainProbes<'a>,
}
impl World for Inputs<'_> {
    fn player_x(&self) -> u16 {
        self.sim.mario_ref_x
    }
    fn face_player(&mut self, p: Position) {
        self.sim.facing = match self.player_x().cmp(&p.x) {
            std::cmp::Ordering::Less => 255,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
    }
    fn wall_blocked(&mut self, p: Position, right: bool) -> bool {
        self.probes.wall_blocked(p, right, 11)
    }
    fn grounded(&mut self, p: Position) -> bool {
        self.probes.grounded(p)
    }
    fn fall(&self) -> &[u8] {
        &FALL_SPEED
    }
    fn request_walk_sound(&mut self) {
        self.sim.sfx2 = 1;
    }
    fn carry_player_up(&mut self, distance: u8) {
        let y = self.sim.mario_y_adjust.unwrap_or(self.sim.mario.y);
        self.sim.mario_y_adjust = Some(y.wrapping_sub(u16::from(distance)));
    }
}
impl ObjectSim {
    pub(super) fn handler_0x589a6(&mut self, st: &mut Staging, layout: &LevelLayout) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let frame = self.frame_tick();
        let mut inputs = Inputs {
            sim: self,
            probes: TerrainProbes::new(layout, Position { x: st.px, y: st.py }),
        };
        let result = handler_0x589a6::update(&mut actor, frame, &mut inputs);
        let probe = inputs.probes.position();
        st.px = probe.x;
        st.py = probe.y;
        if let Some(flags) = inputs.probes.legacy_move_flags() {
            inputs.sim.move_flags = flags;
        }
        st.b = actor.to_raw_slot();
        if result.is_err() {
            self.note_unported(0x589A6);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{object::ObjectTables, player_sim::PlayerSim};
    fn tables() -> ObjectTables {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        )))
        .unwrap()
    }
    fn layout(tile: u8) -> LevelLayout {
        LevelLayout::from_bytes(&[tile; 0x3000]).unwrap()
    }
    #[test]
    fn handler_0x589a6_adapter_full_slot_preservation_wake_and_wait() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        st.b = [0xa5; 32];
        st.b[15] = 0;
        sim.mario_ref_x = 0xa5a5;
        let mut expected = st.b;
        expected[15] = 5;
        expected[18] = 32;
        sim.handler_0x589a6(&mut st, &layout(0x60));
        assert_eq!(st.b, expected);
        st.b[26] = 31;
        expected[26] = 0;
        expected[15] = 6;
        expected[11] ^= 1;
        sim.handler_0x589a6(&mut st, &layout(0x60));
        assert_eq!(st.b, expected);
        assert!(sim.unported.is_empty());
    }
    #[test]
    fn handler_0x589a6_adapter_wall_threshold_move_flags_and_probe_reach() {
        for state in 1..=4 {
            for tile in [0x37, 0x38] {
                let mut sim = ObjectSim::new(vec![], 0);
                sim.sfx2 = 9;
                let mut st = Staging::default();
                st.b = [0xa5; 32];
                st.b[15] = state;
                st.b[0..2].copy_from_slice(&0x100u16.to_be_bytes());
                st.b[3..5].copy_from_slice(&0x100u16.to_be_bytes());
                let mut expected = st.b;
                let right = state == 2 || state == 4;
                let x = if right { 0x101u16 } else { 0xffu16 };
                expected[0..2].copy_from_slice(&x.to_be_bytes());
                if state <= 2 {
                    expected[11] ^= 1;
                }
                sim.handler_0x589a6(&mut st, &layout(tile));
                assert_eq!(st.b, expected);
                let blocked = tile < 0x38;
                assert_eq!(sim.sfx2, if blocked { 1 } else { 9 });
                assert_eq!(
                    sim.move_flags,
                    if right {
                        if blocked {
                            0x11
                        } else {
                            0x10
                        }
                    } else if blocked {
                        0x22
                    } else {
                        0x20
                    }
                );
                assert_eq!(
                    (st.px, st.py),
                    (
                        x.wrapping_add_signed(if right { 11 } else { -11 }),
                        if blocked { 0xf5 } else { 0xfb }
                    )
                );
            }
        }
    }
    #[test]
    fn handler_0x589a6_adapter_ground_threshold_and_alignment() {
        for (y, tile, grounded) in [
            (0x100u16, 0x47, true),
            (0x100, 0x48, false),
            (0x104, 0x47, false),
        ] {
            let mut sim = ObjectSim::new(vec![], 0);
            sim.fff0 = 1;
            let mut st = Staging::default();
            st.b = [0xa5; 32];
            st.b[15] = 4;
            st.b[16] = 21;
            st.b[3..5].copy_from_slice(&y.to_be_bytes());
            st.px = 0x1234;
            st.py = 0x5678;
            let mut expected = st.b;
            if grounded {
                expected[15] = 2;
                expected[16] = 0;
            } else {
                expected[3..5].copy_from_slice(&y.wrapping_add(3).to_be_bytes());
            }
            sim.handler_0x589a6(&mut st, &layout(tile));
            assert_eq!(st.b, expected);
            assert_eq!(sim.move_flags, if grounded { 0x88 } else { 0x80 });
            assert_eq!(
                (st.px, st.py),
                if y == 0x104 {
                    (0x1234, 0x5604)
                } else {
                    (0xa5a5, y)
                }
            );
        }
    }
    #[test]
    fn handler_0x589a6_dispatch_carry_accumulates_wraps_and_player_consumes() {
        let tables = tables();
        let layout = layout(0x60);
        assert_eq!(tables.profile_for(0, 32).unwrap().handler, 0x589A6);
        for high in [0u8, 1, 2, 255] {
            for stood in [false, true] {
                let mut sim = ObjectSim::new(vec![], 0);
                sim.fff0 = 8;
                sim.mario.y = 0;
                sim.mario_ref_x = 0xffff;
                let mut st = Staging::default();
                st.b = [0xa5; 32];
                st.b[0] = high;
                st.b[15] = 6;
                st.b[11] = 61;
                st.b[23] = if stood { 8 } else { 0 };
                let mut expected = st.b;
                expected[15] = 2;
                expected[11] = 62;
                expected[13] = 32;
                expected[18] = 77;
                sim.dispatch_handler(0x589A6, &mut st, &layout, &tables);
                assert_eq!(st.b, expected);
                assert_eq!(sim.facing, 1);
                assert_eq!(
                    sim.mario_y_adjust,
                    if stood {
                        Some(0u16.wrapping_sub(high.into()))
                    } else {
                        None
                    }
                );
                if stood {
                    st.b[15] = 6;
                    st.b[11] = 61;
                    sim.dispatch_handler(0x589A6, &mut st, &layout, &tables);
                    let y = 0u16.wrapping_sub(2 * u16::from(high));
                    assert_eq!(sim.mario_y_adjust, Some(y));
                    let mut player = PlayerSim::new();
                    player.y = 0;
                    player.apply_object_result(&mut sim, false);
                    assert_eq!(player.y, y);
                }
                assert!(sim.unported.is_empty());
            }
        }
    }
    #[test]
    fn handler_0x589a6_real_definition_slot_engine_dispatch() {
        let tables = tables();
        let layout = layout(0x60);
        for dead in [0, 255] {
            let mut sim = ObjectSim::new(vec![], 0);
            sim.fff0 = 3;
            sim.phase = false;
            sim.mario_dead = dead;
            let mut actor = Actor::default();
            actor.position = Position { x: 100, y: 100 };
            actor.activity = 2;
            actor.definition = 32;
            sim.slots[1].b = actor.to_raw_slot();
            let mario = MarioContact {
                x: 100,
                travelling: true,
                ..Default::default()
            };
            sim.step(100, 100, &layout, &tables, &mario, false);
            assert_eq!(
                (sim.slots[1].b[15], sim.slots[1].b[18]),
                if dead == 0 { (5, 32) } else { (0, 0) }
            );
            assert!(sim.unported.is_empty());
        }
    }
    #[test]
    fn handler_0x589a6_no_probe_frames_and_diagnostic_boundaries() {
        for state in [0, 5, 6, 7] {
            let mut sim = ObjectSim::new(vec![], 0);
            sim.fff0 = 7;
            sim.move_flags = 19;
            sim.sfx2 = 29;
            let mut st = Staging::default();
            st.b = [0xa5; 32];
            st.b[15] = state;
            st.px = 0x1234;
            st.py = 0x5678;
            let mut expected = st.b;
            if state == 0 {
                expected[18] = 32;
            }
            if state == 5 {
                expected[26] = 0xa6;
            }
            sim.handler_0x589a6(&mut st, &layout(0x60));
            assert_eq!(st.b, expected);
            assert_eq!(
                (st.px, st.py, sim.move_flags, sim.sfx2),
                (0x1234, 0x5678, 19, 29)
            );
            assert_eq!(
                sim.unported,
                if state == 7 { vec![0x589A6] } else { vec![] }
            );
        }
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 1;
        sim.sfx2 = 29;
        let mut st = Staging::default();
        st.b = [0xa5; 32];
        st.b[15] = 3;
        st.b[16] = 22;
        st.b[3..5].copy_from_slice(&0x100u16.to_be_bytes());
        let expected = st.b;
        sim.handler_0x589a6(&mut st, &layout(0x60));
        assert_eq!(st.b, expected);
        assert_eq!(
            (st.px, st.py, sim.move_flags, sim.sfx2),
            (0xa5a5, 0x100, 0x80, 29)
        );
        assert_eq!(sim.unported, vec![0x589A6]);
    }
    #[test]
    fn handler_0x589a6_carry_player_and_next_camera_consumer() {
        use crate::camera::CameraSim;
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 8;
        sim.mario.y = 320;
        let mut st = Staging::default();
        st.b[0] = 1;
        st.b[11] = 61;
        st.b[15] = 6;
        st.b[23] = 8;
        sim.dispatch_handler(0x589A6, &mut st, &layout(0x60), &tables());
        assert_eq!(sim.mario_y_adjust, Some(319));
        let mut player = PlayerSim::new();
        player.y = 320;
        player.apply_object_result(&mut sim, false);
        assert_eq!(player.y, 319);
        let mut camera = CameraSim::new(0, 400);
        camera.prev_mario_y_lo = 64;
        camera.step(&mut player);
        assert_eq!((camera.y, camera.prev_mario_y_lo), (399, 63));
        assert!(sim.unported.is_empty());
    }
}
