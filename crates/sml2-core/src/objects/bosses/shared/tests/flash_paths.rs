//! Call-site regression tests for ROM `$9443` and `$9CAC` using `$B40E`.

use super::*;

fn staged() -> Staging {
    Staging {
        b: std::array::from_fn(|i| 0x40 + i as u8),
        px: 0x1234,
        py: 0x5678,
    }
}

fn check_nonterminal(handler: fn(&mut ObjectSim, &mut Staging)) {
    for frame in [4, 1] {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        sim.fff0 = frame;
        let mut st = staged();
        st.b[B_T28] = 2;
        let mut expected = st.b;
        expected[B_T28] = 1;
        if frame == 4 {
            expected[B_FLASH] ^= 0x10;
        }
        handler(&mut sim, &mut st);
        assert_eq!(st.b, expected, "frame {frame}");
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert_eq!(sim.music_request, None);
    }
}

#[test]
fn boss_flash_paths_nonterminal_morph_toggles_flash_and_preserves_sprite() {
    check_nonterminal(ObjectSim::boss_morph_to_anim3);
}

#[test]
fn boss_flash_paths_nonterminal_stun_toggles_flash_and_preserves_sprite() {
    check_nonterminal(ObjectSim::boss_hit_stun);
}

#[test]
fn boss_flash_paths_terminal_stun_restores_saved_fields() {
    for timer in [0, 1] {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        sim.fff0 = 4;
        sim.spawn_sprite = 0xA9;
        sim.saved_frame_state = 7;
        sim.saved_type = 0xB3;
        let mut st = staged();
        st.b[B_T28] = timer;
        let mut expected = st.b;
        expected[B_T28] = 0;
        expected[B_FLASH] = 0;
        expected[B_SPRITE] = 0xA9;
        expected[B_FSTATE] = 7;
        expected[B_TYPE] = 0xB3;
        sim.boss_hit_stun(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
    }
}

#[test]
fn boss_flash_paths_terminal_morph_clears_flash_and_selects_anim3() {
    for slot in [0, 0x20] {
        let mut sim = ObjectSim::new(Vec::new(), 0);
        sim.fff0 = 4;
        sim.boss_slot_ptr_lo = slot;
        let mut st = staged();
        st.b[B_T28] = 1;
        let mut expected = st.b;
        expected[B_T28] = 0;
        expected[B_FSTATE] = 0;
        expected[B_DEF] = 3;
        expected[B_ANIMFRAME] = 0;
        expected[B_FLASH] = 0;
        sim.boss_morph_to_anim3(&mut st);
        assert_eq!(st.b, expected);
        assert_eq!((st.px, st.py), (0x1234, 0x5678));
        assert_eq!(sim.music_request, if slot == 0 { Some(0xFF) } else { None });
    }
}
