//! Dynamic object spawner, ROM `0x5B36D` (bank `$16`, CPU `$736D`).
//!
//! Searches the 16 object slots ($AD00..$AEFF) for an inactive slot (`activity == 0` / `B_STATE == 0`).
//! If found:
//! - Copies world coordinates and marker from the parent actor (staged at `$FFD0..$FFD4`).
//! - Sets definition/anim from `sObjectSpawnAnim` ($AF36).
//! - Sets parent record pointer from `sObjectSlotPtrHi` ($AF28) and `sObjectSlotPtrLo & 0xF0` ($AF29).
//! - Sets state to Standby (1).
//! - Sets sprite from `sObjectSpawnSprite` ($AF37).
//! - Sets sprite flip/attribute from `sObjectSpawnAttr` ($AF38).
//! - Increments standby object count ($AF40).
//!
//! Common exit ($5B3AE):
//! - Unconditionally clears `sObjectSpawnAnim`, `sObjectSpawnSprite`, `sObjectSpawnAttr`.

use super::{ObjectSim, Staging, SLOT_COUNT};
use crate::actor::{Actor, SpriteFlipFlags};

/// Parameters for spawning a dynamic object via ROM `$5B36D`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DynamicSpawnRequest {
    pub anim: u8,
    pub sprite: u8,
    pub flip: SpriteFlipFlags,
}

/// Derives the 16-bit WRAM address `(hi, lo)` for a given slot index (0..15).
///
/// In Game Boy memory, slots 0..7 reside at `$AD00..$ADE0` (`hi = 0xAD`),
/// and slots 8..15 reside at `$AE00..$AEE0` (`hi = 0xAE`).
pub const fn slot_address(slot_index: usize) -> (u8, u8) {
    let addr = 0xAD00u16 + ((slot_index as u16) & 0x0F) * 32;
    ((addr >> 8) as u8, (addr & 0xFF) as u8)
}

/// Dynamic object spawner routine, ROM `$5B36D`.
///
/// Returns `Some(slot_index)` on successful spawn, or `None` on capacity abort (all 16 slots full).
/// In either case, the spawn request is consumed and reset to default.
pub fn spawn_dynamic_object(
    parent: &Actor,
    parent_slot_address: (u8, u8),
    slots: &mut [Actor; SLOT_COUNT],
    standby_count: &mut u8,
    request: &mut DynamicSpawnRequest,
) -> Option<usize> {
    let free_idx = slots.iter().position(|a| a.activity == 0);
    let result = if let Some(idx) = free_idx {
        let child = &mut slots[idx];
        child.position = parent.position;
        child.marker = parent.marker;
        child.definition = request.anim;
        child.spawn_record =
            u16::from_be_bytes([parent_slot_address.0, parent_slot_address.1 & 0xF0]);
        child.activity = 1;
        child.sprite = request.sprite;
        child.sprite_flip = request.flip;
        *standby_count = standby_count.wrapping_add(1);
        Some(idx)
    } else {
        None
    };
    *request = DynamicSpawnRequest::default();
    result
}

impl ObjectSim {
    /// Handler entry for ROM `0x5B36D`.
    pub(super) fn handler_0x5b36d(&mut self, st: &mut Staging) {
        self.spawn_dynamic_5b36d(st);
    }

    /// Spawns a dynamic object using the current active slot's staging and scratch registers.
    pub(super) fn spawn_dynamic_5b36d(&mut self, st: &Staging) -> Option<usize> {
        let parent = Actor::from_raw_slot(&st.b);
        let mut actors = std::array::from_fn(|i| Actor::from_raw_slot(&self.slots[i].b));
        let mut request = DynamicSpawnRequest {
            anim: self.spawn_anim,
            sprite: self.spawn_sprite,
            flip: SpriteFlipFlags::from_bits(self.spawn_attr),
        };
        let slot_hi = if self.slot_ptr_hi == 0 {
            0xAD
        } else {
            self.slot_ptr_hi
        };
        let parent_ptr = (slot_hi, self.boss_slot_ptr_lo);
        let result = spawn_dynamic_object(
            &parent,
            parent_ptr,
            &mut actors,
            &mut self.standby,
            &mut request,
        );
        if let Some(child_idx) = result {
            self.slots[child_idx].b = actors[child_idx].to_raw_slot();
        }
        self.spawn_anim = request.anim;
        self.spawn_sprite = request.sprite;
        self.spawn_attr = request.flip.bits();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::Position;
    use crate::level::LevelLayout;
    use crate::object::ObjectTables;
    use crate::objects::{
        B_DEF, B_FLIP, B_MARKER, B_RECHI, B_RECLO, B_SPRITE, B_STATE, B_XHI, B_XLO, B_YHI, B_YLO,
    };

    #[test]
    fn slot_address_calculation_and_boundaries() {
        assert_eq!(slot_address(0), (0xAD, 0x00));
        assert_eq!(slot_address(1), (0xAD, 0x20));
        assert_eq!(slot_address(2), (0xAD, 0x40));
        assert_eq!(slot_address(3), (0xAD, 0x60));
        assert_eq!(slot_address(4), (0xAD, 0x80));
        assert_eq!(slot_address(5), (0xAD, 0xA0));
        assert_eq!(slot_address(6), (0xAD, 0xC0));
        assert_eq!(slot_address(7), (0xAD, 0xE0));
        assert_eq!(slot_address(8), (0xAE, 0x00));
        assert_eq!(slot_address(9), (0xAE, 0x20));
        assert_eq!(slot_address(14), (0xAE, 0xC0));
        assert_eq!(slot_address(15), (0xAE, 0xE0));
    }

    #[test]
    fn spawn_dynamic_object_into_first_available_slot() {
        let mut parent = Actor::default();
        parent.position = Position {
            x: 0x0154,
            y: 0x028A,
        };
        parent.marker = 0x42;
        let mut slots = [Actor::default(); SLOT_COUNT];
        slots[0].activity = 2; // active
        slots[1].activity = 1; // standby
        let mut standby = 5;
        let mut req = DynamicSpawnRequest {
            anim: 0x0C,
            sprite: 0x34,
            flip: SpriteFlipFlags::from_bits(0x20),
        };

        let spawned =
            spawn_dynamic_object(&parent, (0xAD, 0x20), &mut slots, &mut standby, &mut req);

        assert_eq!(spawned, Some(2), "placed in first inactive slot (index 2)");
        assert_eq!(standby, 6, "standby count incremented");
        assert_eq!(req.anim, 0, "request cleared");
        assert_eq!(req.sprite, 0);
        assert_eq!(req.flip.bits(), 0);

        // Verify child actor fields
        let child = &slots[2];
        assert_eq!(child.position.x, 0x0154);
        assert_eq!(child.position.y, 0x028A);
        assert_eq!(child.marker, 0x42);
        assert_eq!(child.definition, 0x0C);
        assert_eq!(child.spawn_record, 0xAD20);
        assert_eq!(child.activity, 1);
        assert_eq!(child.sprite, 0x34);
        assert_eq!(child.sprite_flip.bits(), 0x20);

        // Verify raw slot bytes
        let raw = child.to_raw_slot();
        assert_eq!(raw[B_XHI], 0x01);
        assert_eq!(raw[B_XLO], 0x54);
        assert_eq!(raw[B_MARKER], 0x42);
        assert_eq!(raw[B_YHI], 0x02);
        assert_eq!(raw[B_YLO], 0x8A);
        assert_eq!(raw[B_DEF], 0x0C);
        assert_eq!(raw[B_RECHI], 0xAD);
        assert_eq!(raw[B_RECLO], 0x20);
        assert_eq!(raw[B_STATE], 1);
        assert_eq!(raw[B_SPRITE], 0x34);
        assert_eq!(raw[B_FLIP], 0x20);
    }

    #[test]
    fn spawn_dynamic_object_capacity_abort_when_full() {
        let mut parent = Actor::default();
        parent.position = Position { x: 100, y: 200 };
        let mut slots = [Actor::default(); SLOT_COUNT];
        for s in &mut slots {
            s.activity = 2; // all full
        }
        let mut standby = 10;
        let mut req = DynamicSpawnRequest {
            anim: 5,
            sprite: 6,
            flip: SpriteFlipFlags::from_bits(0x40),
        };

        let result =
            spawn_dynamic_object(&parent, (0xAE, 0x00), &mut slots, &mut standby, &mut req);

        assert_eq!(result, None, "capacity abort returns None");
        assert_eq!(standby, 10, "standby count unmodified");
        assert_eq!(req.anim, 0, "request still unconditionally cleared");
        assert_eq!(req.sprite, 0);
        assert_eq!(req.flip.bits(), 0);
    }

    #[test]
    fn spawn_dynamic_object_masks_low_nibble_of_parent_slot_ptr() {
        let parent = Actor::default();
        let mut slots = [Actor::default(); SLOT_COUNT];
        let mut standby = 0;
        let mut req = DynamicSpawnRequest {
            anim: 1,
            sprite: 2,
            flip: SpriteFlipFlags::default(),
        };

        // Pass unaligned lo byte 0x2F; should mask to 0x20
        let result =
            spawn_dynamic_object(&parent, (0xAD, 0x2F), &mut slots, &mut standby, &mut req);

        assert_eq!(result, Some(0));
        assert_eq!(slots[0].spawn_record, 0xAD20);
        let raw = slots[0].to_raw_slot();
        assert_eq!(raw[B_RECHI], 0xAD);
        assert_eq!(raw[B_RECLO], 0x20);
    }

    #[test]
    fn spawn_dynamic_object_standby_counter_wraps() {
        let parent = Actor::default();
        let mut slots = [Actor::default(); SLOT_COUNT];
        let mut standby = 255u8;
        let mut req = DynamicSpawnRequest {
            anim: 1,
            sprite: 2,
            flip: SpriteFlipFlags::default(),
        };

        let result =
            spawn_dynamic_object(&parent, (0xAD, 0x00), &mut slots, &mut standby, &mut req);

        assert_eq!(result, Some(0));
        assert_eq!(standby, 0, "standby count wrapped 255 -> 0");
    }

    #[test]
    fn object_sim_facade_and_dispatch() {
        let mut sim = ObjectSim::new(vec![], 0);
        sim.slot_ptr_hi = 0xAD;
        sim.boss_slot_ptr_lo = 0x60;
        sim.spawn_anim = 0x12;
        sim.spawn_sprite = 0x28;
        sim.spawn_attr = 0x20;
        sim.standby = 3;

        let mut st = Staging::default();
        st.b[B_XHI] = 0x03;
        st.b[B_XLO] = 0x40;
        st.b[B_MARKER] = 0x05;
        st.b[B_YHI] = 0x01;
        st.b[B_YLO] = 0x20;

        let idx = sim.spawn_dynamic_5b36d(&st);
        assert_eq!(idx, Some(0));
        assert_eq!(sim.standby, 4);
        assert_eq!(sim.spawn_anim, 0);
        assert_eq!(sim.spawn_sprite, 0);
        assert_eq!(sim.spawn_attr, 0);

        let slot = &sim.slots[0];
        assert_eq!(slot.b[B_XHI], 0x03);
        assert_eq!(slot.b[B_XLO], 0x40);
        assert_eq!(slot.b[B_MARKER], 0x05);
        assert_eq!(slot.b[B_YHI], 0x01);
        assert_eq!(slot.b[B_YLO], 0x20);
        assert_eq!(slot.b[B_DEF], 0x12);
        assert_eq!(slot.b[B_RECHI], 0xAD);
        assert_eq!(slot.b[B_RECLO], 0x60);
        assert_eq!(slot.b[B_STATE], 1);
        assert_eq!(slot.b[B_SPRITE], 0x28);
        assert_eq!(slot.b[B_FLIP], 0x20);

        // Verify dispatch via dispatch_handler(0x5B36D)
        let layout = LevelLayout::from_bytes(&[0x60; 0x3000]).unwrap();
        let tables: ObjectTables = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../extracted/objects.json"
        )))
        .unwrap();
        sim.spawn_anim = 0x04;
        sim.spawn_sprite = 0x05;
        sim.dispatch_handler(0x5B36D, &mut st, &layout, &tables);
        assert_eq!(
            sim.unported,
            Vec::<u32>::new(),
            "0x5B36D handled without unported note"
        );
        assert_eq!(sim.standby, 5);
        assert_eq!(sim.slots[1].b[B_STATE], 1);
        assert_eq!(sim.slots[1].b[B_DEF], 0x04);
    }
}
