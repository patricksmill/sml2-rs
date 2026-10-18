//! Camera reward allocation, ROM `$5B346`, sharing `$5B36D`'s spawn tail.
use super::handler_0x5b36d::{spawn_dynamic_object, DynamicSpawnRequest};
use super::SLOT_COUNT;
use crate::actor::{Actor, Position};

/// `$ACE8 + $20` selects slot zero's activity byte, then scans in slot order.
/// Unlike the parent-coordinate entry, this entry leaves the child's marker.
pub(super) fn spawn_from_camera(
    camera: Position,
    parent_address: (u8, u8),
    actors: &mut [Actor; SLOT_COUNT],
    standby: &mut u8,
    request: &mut DynamicSpawnRequest,
) -> Option<usize> {
    let marker = actors
        .iter()
        .find(|a| a.activity == 0)
        .map_or(0, |a| a.marker);
    let mut source = Actor::default();
    source.position = camera;
    source.marker = marker;
    spawn_dynamic_object(&source, parent_address, actors, standby, request)
}

/// The source tests the decimal-adjusted result for zero, before issuing reward.
pub(super) fn add_money_bag(count: &mut u8) -> bool {
    *count = super::helpers::bcd_add(*count, 1).0;
    *count == 0
}
