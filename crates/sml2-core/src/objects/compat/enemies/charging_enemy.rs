//! Bullet Bill (mode B anim 83, ROM `$5A9E8`) slot conversion and view inputs.
use super::*;

impl ObjectSim {
    pub(super) fn handler_charging_enemy(&mut self, st: &mut Staging, tables: &ObjectTables) {
        let mut actor = crate::actor::Actor::from_raw_slot(&st.b);
        let context = charging_enemy::ChargingEnemyContext {
            mario_ref_x: self.mario_ref_x,
            charge_bounds: self.horizontal_reset_bounds(charging_enemy::OFFSCREEN_SCREEN_X),
        };
        let effect = charging_enemy::update(&mut actor, context);
        match effect {
            charging_enemy::ChargingEnemyEffect::FacingSelected { facing } => {
                self.facing = facing;
            }
            charging_enemy::ChargingEnemyEffect::ChargeTriggered { sfx, .. } => {
                self.sfx1 = sfx;
            }
            charging_enemy::ChargingEnemyEffect::OffscreenReset => {
                self.respawn_object_5b2bf(st, tables);
                return;
            }
            _ => {}
        }
        st.b = actor.to_raw_slot();
    }
}

#[cfg(test)]
#[path = "../../enemies/tests/charging_enemy.rs"]
mod tests;
