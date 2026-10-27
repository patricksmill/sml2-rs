//! Slot conversion for the item poses, markers, pickup, squashed, grower and
//! floaters; their behaviour lives in `items/`.

use super::*;
use crate::actor::{Actor, FrameTick};
use item_poses::PoseEffect;

impl ObjectSim {
    fn run_item(&mut self, st: &mut Staging, update: impl FnOnce(&mut Actor, FrameTick)) {
        let mut actor = Actor::from_raw_slot(&st.b);
        update(&mut actor, self.frame_tick());
        st.b = actor.to_raw_slot();
    }

    fn run_pose(
        &mut self,
        st: &mut Staging,
        tables: &ObjectTables,
        update: impl FnOnce(&mut Actor, FrameTick) -> PoseEffect,
    ) {
        let mut effect = PoseEffect::None;
        self.run_item(st, |actor, frame| effect = update(actor, frame));
        match effect {
            PoseEffect::None => {}
            PoseEffect::Parked => self.done = 1,
            PoseEffect::Remove => {
                self.remove_mode = 2;
                self.remove_object(st, tables);
            }
        }
    }

    pub(super) fn handler_itempose20(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.run_pose(st, tables, item_poses::update_flashing_block_item);
    }

    pub(super) fn itempose_body(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.run_pose(st, tables, item_poses::update_block_item);
    }

    pub(super) fn handler_pose86(&mut self, st: &mut Staging) {
        self.run_item(st, item_poses::update_pose86);
    }

    pub(super) fn handler_pose87(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let secret_route = self.secret_route;
        self.run_pose(st, tables, |actor, frame| {
            item_poses::update_pose87(actor, frame, secret_route)
        });
    }

    pub(super) fn handler_marker85(&mut self, st: &mut Staging) {
        self.run_item(st, |actor, _| item_poses::update_marker85(actor));
    }

    pub(super) fn handler_marker66(&mut self, st: &mut Staging) {
        self.run_item(st, |actor, _| item_poses::update_marker66(actor));
    }

    pub(super) fn handler_pickup16(&mut self, st: &mut Staging, tables: &ObjectTables) {
        self.run_pose(st, tables, item_poses::update_pickup);
    }

    pub(super) fn handler_squashed(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = Actor::from_raw_slot(&st.b);
        let effect = squashed::update(&mut actor);
        st.b = actor.to_raw_slot();
        if let squashed::SquashedEffect::Despawn = effect {
            self.remove_mode = 1;
            self.remove_object(st, tables);
        }
    }

    pub(super) fn handler_grower92(&mut self, st: &mut Staging) {
        self.run_item(st, grower::update);
    }

    pub(super) fn handler_floater94(&mut self, st: &mut Staging) {
        self.run_item(st, |actor, _| {
            floater::update(actor, floater::Axis::Vertical)
        });
    }

    pub(super) fn handler_floater95(&mut self, st: &mut Staging) {
        self.run_item(st, |actor, _| {
            floater::update(actor, floater::Axis::Horizontal)
        });
    }

    pub(super) fn handler_0x586eb(&mut self, st: &mut Staging) {
        self.run_item(st, handler_0x586eb::update);
    }
}
