//! Two player contacts share one object engine tick. Rider ownership is
//! recorded before handlers run, so a platform carries its riders, never the
//! player selected as the enemy AI's reference point.
use super::*;
use crate::player_sim::PlayerSim;

impl ObjectSim {
    /// Resolve both players against the same slots, then advance objects once.
    /// A missing contact represents a bubbled or pipe-carried player. `primary`
    /// selects the live player used by enemy AI and shared scene effects.
    #[allow(clippy::too_many_arguments)]
    pub fn step_two_players(
        &mut self,
        cam_x: u16,
        cam_y: u16,
        layout: &LevelLayout,
        tables: &ObjectTables,
        mut players: [&mut PlayerSim; 2],
        contacts: [Option<MarioContact>; 2],
        a_held: [bool; 2],
        boss_mode: bool,
        primary: usize,
    ) {
        assert!(primary < 2);
        self.cooperative = true;
        self.coop_riders.fill(0);
        self.coop_carry.fill((None, None));
        self.camera_position = crate::actor::Position { x: cam_x, y: cam_y };
        for slot in &mut self.slots {
            slot.b[B_FLAGS] &= !8;
        }
        let mut first_touches = [false; SLOT_COUNT];
        for i in 0..2 {
            if i == 1 {
                // Harmful enemies are not consumed by contact; pickups and
                // stomps remain exclusive to the first player who reaches them.
                for (index, slot) in self.slots.iter_mut().enumerate() {
                    if slot.b[B_FLAGS] == 0x10 && slot.b[B_TYPE] & 0x80 == 0 {
                        first_touches[index] = true;
                        slot.b[B_FLAGS] = 0;
                    }
                }
            }
            if let Some(contact) = contacts[i].filter(|c| !c.travelling) {
                self.coop_contact = 1 << i;
                self.contact_pass(cam_x, cam_y, layout, &contact, players[i].dead);
                players[i].apply_object_result(self, a_held[i]);
                self.coop_contacts[i] = MarioContact {
                    x: players[i].x,
                    y: players[i].y,
                    moon_physics: players[i].moon,
                    invincibility: players[i].invincibility,
                    ..contact
                };
            }
            // Shared counters are consumed once, in player order.
            let other = 1 - i;
            players[other].lives = players[i].lives;
            players[other].coins_low = players[i].coins_low;
            players[other].coins_high = players[i].coins_high;
            players[other].level_tally = players[i].level_tally;
        }
        for (slot, touched) in self.slots.iter_mut().zip(first_touches) {
            if touched && slot.b[B_FLAGS] == 0 {
                slot.b[B_FLAGS] = 0x10;
            }
        }
        self.coop_contact = 0;
        self.bounce = false;
        self.flutter_disarmed = false;
        self.stomp_jump = false;
        self.stomp_bounce = false;
        self.on_ground_cleared = false;
        self.mario_x_adjust = None;
        self.mario_y_adjust = None;
        let contact = contacts[primary].unwrap_or(self.mario);
        self.mario = MarioContact {
            x: players[primary].x,
            y: players[primary].y,
            moon_physics: players[primary].moon,
            invincibility: players[primary].invincibility,
            ..contact
        };
        self.mario_dead = players[primary].dead;
        if boss_mode {
            self.boss_engine(cam_x, cam_y, layout, tables);
        } else {
            self.engine(cam_x, cam_y, layout, tables);
        }
        // Contact ground state was already applied separately to each player.
        let grounded = players[primary].on_ground_obj;
        players[primary].apply_object_result(self, a_held[primary]);
        players[primary].on_ground_obj = grounded;
        let other = 1 - primary;
        players[other].lives = players[primary].lives;
        players[other].coins_low = players[primary].coins_low;
        players[other].coins_high = players[primary].coins_high;
        players[other].level_tally = players[primary].level_tally;
        for (i, player) in players.iter_mut().enumerate() {
            if let Some(x) = self.coop_carry[i].0 {
                player.x = x;
            }
            if let Some(y) = self.coop_carry[i].1 {
                player.y = y;
            }
        }
        self.cooperative = false;
    }

    pub(super) fn carry_rider_dx(&mut self, dx: i16) {
        if !self.cooperative {
            let x = self.mario_x_adjust.unwrap_or(self.mario.x);
            self.mario_x_adjust = Some(x.wrapping_add_signed(dx));
            return;
        }
        for i in 0..2 {
            if self.coop_riders[self.coop_slot] & (1 << i) != 0 {
                let x = self.coop_carry[i].0.unwrap_or(self.coop_contacts[i].x);
                self.coop_carry[i].0 = Some(x.wrapping_add_signed(dx));
            }
        }
    }

    pub(super) fn carry_rider_dy(&mut self, dy: i16) {
        if !self.cooperative {
            let y = self.mario_y_adjust.unwrap_or(self.mario.y);
            self.mario_y_adjust = Some(y.wrapping_add_signed(dy));
            return;
        }
        for i in 0..2 {
            if self.coop_riders[self.coop_slot] & (1 << i) != 0 {
                let y = self.coop_carry[i].1.unwrap_or(self.coop_contacts[i].y);
                self.coop_carry[i].1 = Some(y.wrapping_add_signed(dy));
            }
        }
    }

    pub(super) fn rider_scrolls(&self, right: bool) -> bool {
        let accepts = |contact: &MarioContact| {
            if right {
                contact.hvel >= 0x80
            } else {
                contact.hvel < 0x81
            }
        };
        if !self.cooperative {
            return accepts(&self.mario);
        }
        self.coop_contacts
            .iter()
            .enumerate()
            .any(|(i, c)| self.coop_riders[self.coop_slot] & (1 << i) != 0 && accepts(c))
    }
}
