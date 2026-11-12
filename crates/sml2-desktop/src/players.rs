//! Who is playing, and with which input device.
//!
//! Every launch starts single player. Player 1 owns the device that first
//! started the game; Player 2 joins later on the title screen or an overworld
//! map and owns a different device until dropping out. This type is pure: it
//! only records ownership and reports what the host must do, so all of the
//! transition rules can be tested without a window or a controller.

/// One input device. A device belongs to at most one player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Device {
    Keyboard,
    /// A gamepad, identified by the host's stable device id.
    Pad(u64),
}

impl Device {
    pub fn is_keyboard(self) -> bool {
        matches!(self, Self::Keyboard)
    }

    pub fn is_pad(self) -> bool {
        matches!(self, Self::Pad(_))
    }
}

/// Where joining is allowed this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinSpot {
    Title,
    Map,
}

/// What a newly connected device became.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connect {
    None,
    PlayerOne,
    PlayerTwo,
}

/// What a disconnected device did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disconnect {
    None,
    /// Player 1's pad left, and the keyboard was free, so Player 1 moved to it.
    PlayerOneFellBackToKeyboard,
    /// Player 1's pad left while the keyboard was Player 2's.
    PlayerOneNeedsReconnect,
    /// Player 2's pad left in a level; its slot waits for a reconnect.
    PlayerTwoBubbled,
    /// Player 2's pad left outside a level, so Player 2 is gone.
    PlayerTwoLeft,
}

/// Whose pause-menu drop-out was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropOut {
    PlayerTwo,
    /// Player 1 left and the Player 2 device is now Player 1.
    PlayerOnePromoted,
}

/// Device ownership for one launch. Joined state is never persisted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Players {
    p1: Option<Device>,
    p2: Option<Device>,
    /// Connected pads, in connection order (stable host ids). Order matters:
    /// the no-title fallback takes the first pad that connected, not the
    /// smallest id, because a reconnect can reuse or advance the id.
    pads: Vec<u64>,
    /// Player 1's pad is gone and the keyboard is Player 2's.
    p1_absent: bool,
    /// Player 2's pad is gone mid-level; the slot waits until the map.
    p2_absent: bool,
}

impl Players {
    /// Player 1's assigned device, even while it is waiting to reconnect.
    pub fn player_one_device(&self) -> Option<Device> {
        self.p1
    }

    /// Player 1's device while it can provide input.
    pub fn player_one(&self) -> Option<Device> {
        self.p1.filter(|_| !self.p1_absent)
    }

    /// Player 2's assigned device, even while its pad is disconnected.
    pub fn player_two_device(&self) -> Option<Device> {
        self.p2
    }

    /// Player 2's device while it can provide input.
    pub fn player_two(&self) -> Option<Device> {
        self.p2.filter(|_| !self.p2_absent)
    }

    pub fn two_players(&self) -> bool {
        self.p2.is_some()
    }

    pub fn player_one_needs_reconnect(&self) -> bool {
        self.p1_absent
    }

    /// Player 2's pad is disconnected inside a level.
    pub fn player_two_waiting(&self) -> bool {
        self.p2_absent
    }

    pub fn connected(&self, device: Device) -> bool {
        match device {
            Device::Keyboard => true,
            Device::Pad(id) => self.pads.contains(&id),
        }
    }

    /// Pads in connection order, for the ambiguous no-title fallback.
    pub fn connected_pads(&self) -> impl Iterator<Item = u64> + '_ {
        self.pads.iter().copied()
    }

    /// Claim Player 1 without a title screen: the first gamepad in connection
    /// order, else the keyboard.
    pub fn resolve_initial(&mut self) {
        if self.p1.is_none() {
            self.p1 = Some(
                self.pads
                    .first()
                    .map_or(Device::Keyboard, |id| Device::Pad(*id)),
            );
        }
    }

    /// The first Start/Enter on the title screen claims Player 1.
    pub fn claim_player_one(&mut self, device: Device) -> bool {
        if self.p1.is_none() && self.connected(device) {
            self.p1 = Some(device);
            true
        } else {
            false
        }
    }

    /// A device that is not Player 1's joins as Player 2 when joining is
    /// allowed and nobody has joined yet.
    pub fn join(&mut self, device: Device) -> bool {
        if self.p2.is_some() || self.p1.is_none() || self.p1_absent {
            return false;
        }
        if Some(device) == self.p1 || !self.connected(device) {
            return false;
        }
        self.p2 = Some(device);
        self.p2_absent = false;
        true
    }

    /// Apply a pause-menu drop-out for the menu's owner. `None` means the menu
    /// is system-owned (the reconnect prompt) and drops Player 1.
    pub fn drop_out(&mut self, owner: Option<Device>) -> Option<DropOut> {
        let p1 = self.p1?;
        let p2 = self.p2?;
        match owner {
            Some(device) if device == p2 => {
                self.p2 = None;
                self.p2_absent = false;
                Some(DropOut::PlayerTwo)
            }
            Some(device) if device == p1 => {
                self.p1 = Some(p2);
                self.p1_absent = self.p2_absent;
                self.p2 = None;
                self.p2_absent = false;
                Some(DropOut::PlayerOnePromoted)
            }
            None => {
                self.p1 = Some(p2);
                self.p1_absent = self.p2_absent;
                self.p2 = None;
                self.p2_absent = false;
                Some(DropOut::PlayerOnePromoted)
            }
            Some(_) => None,
        }
    }

    /// Back on the map after Player 2's pad vanished in a level: remove the
    /// waiting slot. Returns the device that was removed.
    pub fn remove_absent_player_two(&mut self) -> Option<Device> {
        if !self.p2_absent {
            return None;
        }
        self.p2_absent = false;
        self.p2.take()
    }

    pub fn connect(&mut self, device: Device) -> Connect {
        if let Device::Pad(id) = device {
            if !self.pads.contains(&id) {
                self.pads.push(id);
            }
        }
        // A returning pad may carry a new host id (macOS assigns one on every
        // reconnect), so a waiting slot takes the first pad that arrives.
        if self.p1_absent && device.is_pad() {
            self.p1 = Some(device);
            self.p1_absent = false;
            return Connect::PlayerOne;
        }
        if self.p2_absent && device.is_pad() && self.p2.is_some_and(Device::is_pad) {
            self.p2 = Some(device);
            self.p2_absent = false;
            return Connect::PlayerTwo;
        }
        Connect::None
    }

    pub fn disconnect(&mut self, device: Device, in_level: bool) -> Disconnect {
        if let Device::Pad(id) = device {
            self.pads.retain(|connected| *connected != id);
        }
        if self.p1 == Some(device) {
            if self.p2 != Some(Device::Keyboard) {
                self.p1 = Some(Device::Keyboard);
                self.p1_absent = false;
                return Disconnect::PlayerOneFellBackToKeyboard;
            }
            self.p1_absent = true;
            return Disconnect::PlayerOneNeedsReconnect;
        }
        if self.p2 == Some(device) {
            if in_level {
                self.p2_absent = true;
                Disconnect::PlayerTwoBubbled
            } else {
                self.p2 = None;
                self.p2_absent = false;
                Disconnect::PlayerTwoLeft
            }
        } else {
            Disconnect::None
        }
    }

    /// The banner shown while a joinable device is connected and nobody has
    /// joined.
    pub fn join_prompt(&self, allowed: bool) -> Option<&'static str> {
        if !allowed || self.p2.is_some() || self.p1_absent {
            return None;
        }
        let p1 = self.p1?;
        if self
            .pads
            .iter()
            .any(|id| Device::Pad(*id) != p1 && self.p2 != Some(Device::Pad(*id)))
        {
            Some("Press + to join")
        } else if p1 != Device::Keyboard {
            Some("Press Enter to join")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players() -> Players {
        Players::default()
    }

    #[test]
    fn every_launch_starts_single_player() {
        let p = players();
        assert!(!p.two_players());
        assert_eq!(p.player_one(), None);
        assert_eq!(p.join_prompt(true), None, "nobody can join before Player 1");
    }

    #[test]
    fn the_first_start_on_the_title_claims_player_one() {
        let mut p = players();
        p.connect(Device::Pad(1));
        assert!(p.claim_player_one(Device::Pad(1)));
        assert_eq!(p.player_one(), Some(Device::Pad(1)));
        assert!(
            !p.claim_player_one(Device::Keyboard),
            "only the first claims"
        );
        assert_eq!(p.player_one(), Some(Device::Pad(1)));
        assert!(!p.two_players());
    }

    #[test]
    fn the_no_title_fallback_prefers_the_first_pad_to_connect_then_the_keyboard() {
        let mut p = players();
        p.connect(Device::Pad(9));
        p.connect(Device::Pad(4));
        p.resolve_initial();
        assert_eq!(
            p.player_one(),
            Some(Device::Pad(9)),
            "connection order, not numeric id"
        );
        let mut p = players();
        p.resolve_initial();
        assert_eq!(p.player_one(), Some(Device::Keyboard));
    }

    #[test]
    fn a_different_device_joins_player_two_only_when_allowed() {
        let mut p = players();
        p.connect(Device::Pad(1));
        p.connect(Device::Pad(2));
        p.claim_player_one(Device::Pad(1));
        assert!(p.join(Device::Pad(2)));
        assert!(p.two_players());
        assert_eq!(p.player_two(), Some(Device::Pad(2)));
        assert!(!p.join(Device::Pad(3)), "only two players");

        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        p.connect(Device::Pad(1));
        assert!(!p.join(Device::Keyboard), "Player 1's device cannot join");
        assert!(p.join(Device::Pad(1)));
    }

    #[test]
    fn only_the_first_extra_device_joins_of_three() {
        let mut p = players();
        for id in [1, 2, 3] {
            p.connect(Device::Pad(id));
        }
        p.claim_player_one(Device::Pad(1));
        assert!(p.join(Device::Pad(3)));
        assert_eq!(p.player_two(), Some(Device::Pad(3)));
        assert!(!p.join(Device::Pad(2)));
    }

    #[test]
    fn dropping_player_two_keeps_player_one_and_its_device() {
        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        p.connect(Device::Pad(1));
        p.join(Device::Pad(1));
        assert_eq!(p.drop_out(Some(Device::Pad(1))), Some(DropOut::PlayerTwo));
        assert!(!p.two_players());
        assert_eq!(p.player_one(), Some(Device::Keyboard));
    }

    #[test]
    fn dropping_player_one_promotes_player_two_and_keeps_its_device() {
        let mut p = players();
        p.connect(Device::Pad(1));
        p.claim_player_one(Device::Pad(1));
        p.connect(Device::Pad(2));
        assert!(p.join(Device::Pad(2)));
        assert_eq!(
            p.drop_out(Some(Device::Pad(1))),
            Some(DropOut::PlayerOnePromoted)
        );
        assert!(!p.two_players());
        assert_eq!(p.player_one(), Some(Device::Pad(2)));
        assert_eq!(p.drop_out(Some(Device::Pad(1))), None, "single player");
    }

    #[test]
    fn player_ones_menu_never_removes_player_two() {
        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        p.connect(Device::Pad(1));
        p.join(Device::Pad(1));
        assert_eq!(
            p.drop_out(Some(Device::Keyboard)),
            Some(DropOut::PlayerOnePromoted)
        );
        assert!(!p.two_players());
        assert_eq!(p.player_one(), Some(Device::Pad(1)));
    }

    #[test]
    fn player_two_disconnect_in_a_level_waits_for_the_map_or_a_reconnect() {
        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        p.connect(Device::Pad(1));
        p.join(Device::Pad(1));
        assert_eq!(
            p.disconnect(Device::Pad(1), true),
            Disconnect::PlayerTwoBubbled
        );
        assert!(p.two_players());
        assert!(p.player_two_waiting());
        assert_eq!(p.player_two(), None, "no input until it reconnects");
        assert_eq!(p.connect(Device::Pad(1)), Connect::PlayerTwo);
        assert!(!p.player_two_waiting());
        assert_eq!(p.player_two(), Some(Device::Pad(1)));

        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        p.connect(Device::Pad(1));
        p.join(Device::Pad(1));
        p.disconnect(Device::Pad(1), true);
        assert_eq!(p.remove_absent_player_two(), Some(Device::Pad(1)));
        assert!(!p.two_players());
    }

    #[test]
    fn player_two_resumes_when_its_pad_reconnects_with_a_new_id() {
        // macOS assigns a fresh host id to a reconnected controller.
        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        p.connect(Device::Pad(1));
        p.join(Device::Pad(1));
        assert_eq!(
            p.disconnect(Device::Pad(1), true),
            Disconnect::PlayerTwoBubbled
        );
        assert_eq!(p.connect(Device::Pad(7)), Connect::PlayerTwo);
        assert!(!p.player_two_waiting());
        assert_eq!(p.player_two(), Some(Device::Pad(7)));
        assert!(p.two_players());
    }

    #[test]
    fn player_one_resumes_when_its_pad_reconnects_with_a_new_id() {
        let mut p = players();
        p.connect(Device::Pad(1));
        p.claim_player_one(Device::Pad(1));
        p.join(Device::Keyboard);
        assert_eq!(
            p.disconnect(Device::Pad(1), false),
            Disconnect::PlayerOneNeedsReconnect
        );
        assert_eq!(p.connect(Device::Pad(5)), Connect::PlayerOne);
        assert!(!p.player_one_needs_reconnect());
        assert_eq!(p.player_one(), Some(Device::Pad(5)));
        assert_eq!(p.player_two(), Some(Device::Keyboard));
    }

    #[test]
    fn player_two_disconnect_outside_a_level_removes_player_two_at_once() {
        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        p.connect(Device::Pad(1));
        p.join(Device::Pad(1));
        assert_eq!(
            p.disconnect(Device::Pad(1), false),
            Disconnect::PlayerTwoLeft
        );
        assert!(!p.two_players());
        assert!(!p.player_two_waiting());
    }

    #[test]
    fn player_one_pad_disconnect_falls_back_to_a_free_keyboard() {
        let mut p = players();
        p.connect(Device::Pad(1));
        p.connect(Device::Pad(2));
        p.claim_player_one(Device::Pad(1));
        p.join(Device::Pad(2));
        assert_eq!(
            p.disconnect(Device::Pad(1), true),
            Disconnect::PlayerOneFellBackToKeyboard
        );
        assert_eq!(p.player_one(), Some(Device::Keyboard));
        assert_eq!(p.player_two(), Some(Device::Pad(2)));
    }

    #[test]
    fn player_one_pad_disconnect_waits_when_the_keyboard_is_player_twos() {
        let mut p = players();
        p.connect(Device::Pad(1));
        p.claim_player_one(Device::Pad(1));
        p.join(Device::Keyboard);
        assert_eq!(
            p.disconnect(Device::Pad(1), false),
            Disconnect::PlayerOneNeedsReconnect
        );
        assert!(p.player_one_needs_reconnect());
        assert_eq!(p.player_one(), None);
        assert_eq!(p.player_two(), Some(Device::Keyboard));

        // A gamepad reconnect returns control to Player 1.
        assert_eq!(p.connect(Device::Pad(1)), Connect::PlayerOne);
        assert!(!p.player_one_needs_reconnect());
        assert_eq!(p.player_one(), Some(Device::Pad(1)));

        // Or Player 1 drops out and Player 2 is promoted.
        let mut p = players();
        p.connect(Device::Pad(1));
        p.claim_player_one(Device::Pad(1));
        p.join(Device::Keyboard);
        p.disconnect(Device::Pad(1), false);
        assert_eq!(p.drop_out(None), Some(DropOut::PlayerOnePromoted));
        assert_eq!(p.player_one(), Some(Device::Keyboard));
    }

    #[test]
    fn the_join_prompt_names_the_class_of_the_joinable_device() {
        let mut p = players();
        p.connect(Device::Pad(1));
        p.connect(Device::Pad(2));
        p.claim_player_one(Device::Pad(1));
        assert_eq!(p.join_prompt(true), Some("Press + to join"));
        assert_eq!(p.join_prompt(false), None, "never in a level or launcher");

        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        p.connect(Device::Pad(1));
        assert_eq!(p.join_prompt(true), Some("Press + to join"));

        let mut p = players();
        p.connect(Device::Pad(1));
        p.claim_player_one(Device::Pad(1));
        assert_eq!(p.join_prompt(true), Some("Press Enter to join"));

        let mut p = players();
        p.claim_player_one(Device::Keyboard);
        assert_eq!(p.join_prompt(true), None, "the keyboard is Player 1's");

        p.connect(Device::Pad(1));
        p.join(Device::Pad(1));
        assert_eq!(p.join_prompt(true), None, "nobody left to join");
    }
}
