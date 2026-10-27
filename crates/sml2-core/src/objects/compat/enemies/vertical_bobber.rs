//! Compatibility facade for the native vertical bobber (ROM `0x591F9`).

use super::vertical_bobber::{update, BobberContext, BobberEffect};
use super::*;
use crate::actor::Actor;
use std::cmp::Ordering;

impl ObjectSim {
    pub(super) fn handler_0x591f9(&mut self, st: &mut Staging) {
        let context = BobberContext {
            frame: self.frame_tick(),
            player_ref_y: self.mario_ref_y,
        };
        let mut actor = Actor::from_raw_slot(&st.b);
        let result = update(&mut actor, context);
        st.b = actor.to_raw_slot();
        match result {
            Ok(BobberEffect::PlayerRelation(relation)) => {
                self.facing = match relation {
                    Ordering::Less => 0xFF,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };
            }
            Ok(BobberEffect::None) => {}
            Err(_) => self.note_unported(0x591F9),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_preserves_unwritten_slot_and_scratch_bytes() {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.fff0 = 4;
        sim.facing = 0xA3;
        let mut st = Staging {
            b: [0xA5; 32],
            px: 0x1234,
            py: 0xFEDC,
        };
        st.b[B_FSTATE] = 3;
        st.b[B_T28] = 0;
        st.b[B_PHASE] = 16;
        set_y(&mut st.b, 0xFFFF);
        let mut expected = st.b;
        expected[B_PHASE] = 17;
        expected[B_SPRITE] ^= 1;
        set_y(&mut expected, 1); // source fall sample 16 is 2, not clamped sample 15 (3).
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py, sim.facing), (0x1234, 0xFEDC, 0xA3));

        st.b[B_FSTATE] = 99;
        expected = st.b;
        expected[B_SPRITE] ^= 1;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!(sim.unported, [0x591F9]);
    }

    #[test]
    fn real_step_dispatches_bobber_and_freezes_handler_when_mario_dead() {
        let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
        let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        )))
        .unwrap();
        for level in [0, 0x20] {
            assert_eq!(tables.profile_for(level, 58).unwrap().handler, 0x591F9);
            for dead in [0, 0xFF] {
                let mut sim = ObjectSim::new(vec![], 0);
                sim.level_id = level;
                sim.phase = false;
                sim.fff0 = 3;
                sim.slots[1].b[B_STATE] = 2;
                sim.slots[1].b[B_DEF] = 58;
                sim.slots[1].b[B_SPRITE] = 0x10;
                set_x(&mut sim.slots[1].b, 100);
                set_y(&mut sim.slots[1].b, 100);
                sim.mario_dead = dead;
                let mario = MarioContact {
                    x: 0,
                    y: 68,
                    travelling: true,
                    ..Default::default()
                };
                sim.step(100, 100, &layout, &tables, &mario, false);
                assert_eq!(sim.fff0, 4);
                assert_eq!(sim.slots[1].b[B_FSTATE], if dead == 0 { 1 } else { 0 });
                assert_eq!(sim.slots[1].b[B_TYPE], if dead == 0 { 86 } else { 0 });
                assert_eq!(sim.slots[1].b[B_FLASH], if dead == 0 { 16 } else { 0 });
                assert_eq!(
                    sim.slots[1].b[B_SPRITE],
                    if dead == 0 { 0x11 } else { 0x10 }
                );
                assert_eq!(sim.facing, 0); // equal full Y chooses up but keeps equality relation.
                sim.phase = false;
                sim.fff0 = 7;
                sim.step(100, 100, &layout, &tables, &mario, false);
                assert_eq!(sim.slots[1].y(), if dead == 0 { 97 } else { 100 });
                assert_eq!(sim.slots[1].b[B_PHASE], if dead == 0 { 1 } else { 0 });
                assert!(sim.unported.is_empty());
            }
        }
    }

    #[test]
    fn state0_initialization_sets_type_flash_and_faces_mario_y() {
        let mut sim = ObjectSim::new(vec![], 0);

        // Case 1: Mario below object (mario_ref_y > obj_y) -> state 2, facing 1
        sim.mario_ref_y = 200;
        let mut st = Staging::default();
        set_y(&mut st.b, 100);
        st.b[B_FSTATE] = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_TYPE], 0x56, "type set to 86");
        assert_eq!(st.b[B_FLASH], 0x10, "flash set to 16");
        assert_eq!(st.b[B_FSTATE], 2, "state set to 2 when Mario below");
        assert_eq!(sim.facing, 1, "facing set to 1");

        // Case 2: Mario above object (mario_ref_y < obj_y) -> state 1, facing 0xFF
        sim.mario_ref_y = 50;
        let mut st = Staging::default();
        set_y(&mut st.b, 100);
        st.b[B_FSTATE] = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_TYPE], 0x56);
        assert_eq!(st.b[B_FLASH], 0x10);
        assert_eq!(st.b[B_FSTATE], 1, "state set to 1 when Mario above");
        assert_eq!(sim.facing, 0xFF, "facing set to 255");

        // Case 3: Mario at same Y (mario_ref_y == obj_y) -> state 1, facing 0
        sim.mario_ref_y = 100;
        let mut st = Staging::default();
        set_y(&mut st.b, 100);
        st.b[B_FSTATE] = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 1, "state set to 1 when Mario level");
        assert_eq!(sim.facing, 0, "facing set to 0");
    }

    #[test]
    fn sprite_toggle1_toggles_on_even_frames() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        st.b[B_SPRITE] = 0x10;
        st.b[B_FSTATE] = 1;

        // Even frame (fff0 = 0): toggles bit 0
        sim.fff0 = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_SPRITE], 0x11, "even frame toggled sprite bit 0");

        // Odd frame (fff0 = 1): does not toggle
        sim.fff0 = 1;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_SPRITE], 0x11, "odd frame preserved sprite bit 0");
    }

    #[test]
    fn movement_gated_to_fff0_mod_4() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 200);
        st.b[B_FSTATE] = 1;
        st.b[B_PHASE] = 0;

        // Frames 1, 2, 3: movement gate closed, Y does not change
        for frame in [1u8, 2, 3] {
            sim.fff0 = frame;
            sim.handler_0x591f9(&mut st);
            assert_eq!(st.y(), 200, "Y unchanged on frame {frame}");
            assert_eq!(st.b[B_PHASE], 0, "phase unchanged on frame {frame}");
        }

        // Frame 4 (4 & 3 == 0): movement gate opens, Y moves UP by BOUNCE_SPEED[0] (3)
        sim.fff0 = 4;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.y(), 197, "Y moved up by 3");
        assert_eq!(st.b[B_PHASE], 1, "phase incremented to 1");
    }

    #[test]
    fn state1_bounce_up_reaches_cap_and_transitions_to_state3() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 500);
        st.b[B_FSTATE] = 1;
        st.b[B_PHASE] = 14;

        // At phase 14, step reads BOUNCE_SPEED[14] (0), phase reaches 15 -> resets to 0, state becomes 3
        sim.fff0 = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_PHASE], 0, "phase reset to 0");
        assert_eq!(st.b[B_FSTATE], 3, "state transitioned 1 -> 3");
        assert_eq!(st.y(), 500 - (BOUNCE_SPEED[14] as u16));
    }

    #[test]
    fn state2_bounce_down_reaches_cap_and_transitions_to_state4() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 500);
        st.b[B_FSTATE] = 2;
        st.b[B_PHASE] = 14;

        // At phase 14, step reads BOUNCE_SPEED[14] (0), phase reaches 15 -> resets to 0, state becomes 4
        sim.fff0 = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_PHASE], 0, "phase reset to 0");
        assert_eq!(st.b[B_FSTATE], 4, "state transitioned 2 -> 4");
        assert_eq!(st.y(), 500 + (BOUNCE_SPEED[14] as u16));
    }

    #[test]
    fn state3_t28_delay_then_fall_down_and_transitions_to_state2() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 100);
        st.b[B_FSTATE] = 3;
        st.b[B_T28] = 2;
        st.b[B_PHASE] = 0;

        // Tick 1 with fff0 & 3 == 0: decrements T28 from 2 to 1, no move
        sim.fff0 = 4;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_T28], 1);
        assert_eq!(st.y(), 100);
        assert_eq!(st.b[B_PHASE], 0);

        // Tick 2: decrements T28 from 1 to 0, no move
        sim.fff0 = 8;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_T28], 0);
        assert_eq!(st.y(), 100);
        assert_eq!(st.b[B_PHASE], 0);

        // Tick 3: T28 is 0 -> moves DOWN by FALL_SPEED[0] (0)
        sim.fff0 = 12;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.y(), 100 + (FALL_SPEED[0] as u16));
        assert_eq!(st.b[B_PHASE], 1);
        assert_eq!(st.b[B_FSTATE], 3);

        // Fast forward to phase 14
        st.b[B_PHASE] = 14;
        sim.fff0 = 16;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_PHASE], 0, "phase reset to 0");
        assert_eq!(st.b[B_FSTATE], 2, "state transitioned 3 -> 2");
    }

    #[test]
    fn state4_t28_delay_then_fall_up_and_transitions_to_state1() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        set_y(&mut st.b, 200);
        st.b[B_FSTATE] = 4;
        st.b[B_T28] = 1;
        st.b[B_PHASE] = 0;

        // Decrements T28 to 0
        sim.fff0 = 4;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_T28], 0);
        assert_eq!(st.y(), 200);

        // Fast forward to phase 14
        st.b[B_PHASE] = 14;
        sim.fff0 = 8;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_PHASE], 0, "phase reset to 0");
        assert_eq!(st.b[B_FSTATE], 1, "state transitioned 4 -> 1");
    }

    #[test]
    fn complete_closed_cycle_1_to_3_to_2_to_4_to_1() {
        let mut sim = ObjectSim::new(vec![], 0);
        let mut st = Staging::default();
        sim.mario_ref_y = 50; // above
        set_y(&mut st.b, 200);
        st.b[B_FSTATE] = 0;

        // Init -> state 1
        sim.fff0 = 0;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 1);

        // State 1 completes -> state 3
        st.b[B_PHASE] = 14;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 3);

        // State 3 completes -> state 2
        st.b[B_T28] = 0;
        st.b[B_PHASE] = 14;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 2);

        // State 2 completes -> state 4
        st.b[B_PHASE] = 14;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 4);

        // State 4 completes -> state 1
        st.b[B_T28] = 0;
        st.b[B_PHASE] = 14;
        sim.handler_0x591f9(&mut st);
        assert_eq!(st.b[B_FSTATE], 1);
    }

    #[test]
    fn dispatch_and_unported_handling() {
        let mut sim = ObjectSim::new(vec![], 0);
        let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
        let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        )))
        .unwrap();

        // Valid dispatch: runs without flagging unported
        let mut st = Staging::default();
        st.b[B_DEF] = 58;
        st.b[B_FSTATE] = 0;
        sim.dispatch_handler(0x591F9, &mut st, &layout, &tables);
        assert!(sim.unported.is_empty(), "0x591F9 is implemented");
        assert_eq!(st.b[B_TYPE], 0x56);

        // Invalid state flags unported
        st.b[B_FSTATE] = 99;
        sim.dispatch_handler(0x591F9, &mut st, &layout, &tables);
        assert_eq!(sim.unported, vec![0x591F9]);
    }
}
