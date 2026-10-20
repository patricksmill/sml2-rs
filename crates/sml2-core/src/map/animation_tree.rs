//! Tree-A decoration animation, ROM $3DA2D and secret growth $3DADF.
use super::super::MapSim;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TreeAnimation {
    /// $A6B0, index into the shared $6227 movement table.
    pub(super) index_b: u8,
    /// $A783: low two bits select the frame; bit 4 selects direction.
    pub(super) step: u8,
    /// $A784: ten-frame wing animation clock, retained across screen loads.
    pub(super) tick: u8,
}

pub(super) fn reset(map: &mut MapSim) {
    map.zone_animation.tree.index_b = 0;
    map.zone_animation.tree.step = 16;
}

pub(super) fn tick(map: &mut MapSim, _div: u8) {
    let tree = &mut map.zone_animation.tree;
    tree.tick = tree.tick.wrapping_add(1);
    if tree.tick >= 10 {
        tree.tick = 0;
        tree.step = tree.step.wrapping_add(1);
        // $A6BD (sMapSprite2Page) is zone triple 2's id. The
        // adjacent $A6BB/$A6BC bytes are that triple's x/y, not hub slots.
        map.zone_slots[2].2 = 87 + (tree.step & 3) + ((tree.step & 0x10) >> 2);
        if tree.step & 0x10 == 0 {
            map.zone_slots[2].0 = map.zone_slots[2].0.wrapping_add(1);
            map.zone_slots[2].1 = map.zone_slots[2].1.wrapping_sub(1);
        } else {
            map.zone_slots[2].0 = map.zone_slots[2].0.wrapping_sub(1);
            map.zone_slots[2].1 = map.zone_slots[2].1.wrapping_add(1);
        }
    }

    // $A6BA is triple 1's id; $A6B9/$A6B8 its y/x.
    map.zone_slots[1].2 = 4 + ((map.tick_timer & 2) >> 1);
    if map.tick_timer & 1 != 0 {
        tree.index_b = tree.index_b.wrapping_add(1);
        if tree.index_b >= 80 {
            tree.index_b = 0;
        }
        let movement = crate::map::PUMPKIN_STEPS[usize::from(tree.index_b)];
        let dy = movement & 7;
        map.zone_slots[1].1 = if movement & 8 != 0 {
            map.zone_slots[1].1.wrapping_sub(dy)
        } else {
            map.zone_slots[1].1.wrapping_add(dy)
        };
        let dx = (movement >> 4) & 7;
        map.zone_slots[1].0 = if movement & 0x80 != 0 {
            map.zone_slots[1].0.wrapping_sub(dx)
        } else {
            map.zone_slots[1].0.wrapping_add(dx)
        };
    }

    map.zone_slots[0].2 = if map.level_progress[5] & 0x80 != 0 {
        255
    } else {
        (map.rand_latch & 0x30) >> 4
    };

    // $A6D2 (sMapSprite7X) aliases triple 9's id: $6A-$6D are
    // successive secret-route growth metasprites, with a sound on each step.
    if map.frame_counter & 0x0F == 0 && map.zone_slots[9].2 < 0x6D {
        map.zone_slots[9].2 += 1;
        map.zone_animation.sfx0 = if map.zone_slots[9].2 == 0x6D { 21 } else { 6 };
    }
    map.seed_zone_path_markers(2);
}

#[cfg(test)]
#[path = "animation_tree_tests.rs"]
mod tests;
