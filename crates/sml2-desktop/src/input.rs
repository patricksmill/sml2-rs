use serde::{Deserialize, Serialize};
use sml2_core::player_sim::PlayerKeys;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    Up,
    Down,
    Left,
    Right,
    Start,
    Select,
    LeftThumb,
    RightThumb,
    LeftShoulder,
    RightShoulder,
    /// ZL / L2.
    LeftTrigger,
    /// ZR / R2.
    RightTrigger,
    StickUp,
    StickDown,
    StickLeft,
    StickRight,
}

impl PadButton {
    /// Every native button, for aggregating all connected pads.
    #[cfg(any(feature = "native", test))]
    pub(crate) const ALL: [Self; 20] = {
        use PadButton::*;
        [
            South,
            East,
            West,
            North,
            Up,
            Down,
            Left,
            Right,
            Start,
            Select,
            LeftThumb,
            RightThumb,
            LeftShoulder,
            RightShoulder,
            LeftTrigger,
            RightTrigger,
            StickUp,
            StickDown,
            StickLeft,
            StickRight,
        ]
    };
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Button {
    Key(String),
    Pad(PadButton),
}

impl Button {
    pub fn key(name: &str) -> Self {
        Self::Key(name.into())
    }
    pub fn label(&self) -> String {
        match self {
            Self::Key(key) => match key.strip_prefix("Num") {
                Some(digit) if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => {
                    digit.to_string()
                }
                _ => key.clone(),
            },
            Self::Pad(pad) => format!("Pad {pad:?}"),
        }
    }
    pub fn reserved(&self) -> bool {
        match self {
            Self::Key(key) => [
                "Escape", "F1", "F2", "F3", "F10", "F11", "Plus", "Equals", "Minus",
            ]
            .contains(&key.as_str()),
            Self::Pad(PadButton::Start) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Action {
    Right,
    Left,
    Up,
    Down,
    Jump,
    Run,
    Start,
    Select,
    /// Holds Down while Mario is airborne, so the spin jump needs no D-pad.
    Spin,
    ToggleToolbar,
}

impl Action {
    pub const ALL: [Self; 10] = [
        Self::Right,
        Self::Left,
        Self::Up,
        Self::Down,
        Self::Jump,
        Self::Run,
        Self::Start,
        Self::Select,
        Self::Spin,
        Self::ToggleToolbar,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Right => "Move right",
            Self::Left => "Move left",
            Self::Up => "Up",
            Self::Down => "Down / duck",
            Self::Jump => "Jump / A",
            Self::Run => "Run / B",
            Self::Start => "Game Start",
            Self::Select => "Game Select",
            Self::Spin => "Spin jump",
            Self::ToggleToolbar => "Toggle toolbar",
        }
    }
    /// Actions the original controls already cover may be left unbound.
    pub fn optional(self) -> bool {
        matches!(self, Self::Spin | Self::ToggleToolbar)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bindings(pub [Vec<Button>; 10]);

impl Default for Bindings {
    fn default() -> Self {
        use PadButton::*;
        let row = |keys: &[&str], pads: &[PadButton]| {
            keys.iter()
                .map(|k| Button::key(k))
                .chain(pads.iter().copied().map(Button::Pad))
                .collect()
        };
        Self([
            row(&["ArrowRight"], &[Right, StickRight]),
            row(&["ArrowLeft"], &[Left, StickLeft]),
            row(&["ArrowUp"], &[Up, StickUp]),
            row(&["ArrowDown"], &[Down, StickDown]),
            row(&["Z", "Space"], &[South]),
            row(&["X", "Shift"], &[West, East]),
            row(&["Enter"], &[LeftThumb]),
            row(&["Backspace"], &[Select]),
            row(&["C"], &[LeftTrigger, RightTrigger]),
            row(&[], &[]),
        ])
    }
}

impl Bindings {
    /// Player 2's fixed keyboard layout. It deliberately avoids every default
    /// Player 1 button and every desktop menu shortcut, so both players can
    /// share one keyboard. Controllers stay on Player 1.
    pub fn player_two() -> Self {
        let row = |keys: &[&str]| keys.iter().map(|k| Button::key(k)).collect();
        Self([
            row(&["D"]),
            row(&["A"]),
            row(&["W"]),
            row(&["S"]),
            row(&["F"]),
            row(&["G"]),
            row(&["Num1"]),
            row(&["Num2"]),
            row(&["H"]),
            row(&[]),
        ])
    }

    /// Player 1 actions whose buttons `reserved` already claims. Player 2's
    /// fixed layout is reserved, so loaded or hand-edited settings cannot
    /// silently give one key to both players.
    pub fn conflicts_with(&self, reserved: &Bindings) -> Vec<(Action, Vec<Button>)> {
        let mut conflicts = Vec::new();
        for action in Action::ALL {
            let buttons: Vec<Button> = self.0[action as usize]
                .iter()
                .filter(|button| reserved.0.iter().flatten().any(|other| other == *button))
                .cloned()
                .collect();
            if !buttons.is_empty() {
                conflicts.push((action, buttons));
            }
        }
        conflicts
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut used = BTreeSet::new();
        for action in Action::ALL {
            let row = &self.0[action as usize];
            if row.is_empty() && !action.optional() {
                return Err(format!("{} needs a binding", action.label()));
            }
            for button in row {
                if button.reserved() {
                    return Err(format!("{} is a menu shortcut", button.label()));
                }
                if let Button::Key(key) = button {
                    if key.is_empty() || key.len() > 32 {
                        return Err("Invalid key name".into());
                    }
                }
                if !used.insert(button) {
                    return Err(format!("{} is bound more than once", button.label()));
                }
            }
        }
        Ok(())
    }

    /// Replace bindings for that device class, keeping the other device usable.
    pub fn rebind(&mut self, action: Action, button: Button) -> Result<(), String> {
        if button.reserved() {
            return Err(format!(
                "{} is reserved for the desktop menu",
                button.label()
            ));
        }
        for other in Action::ALL {
            if other != action && self.0[other as usize].contains(&button) {
                return Err(format!("{} already uses {}", other.label(), button.label()));
            }
        }
        let row = &mut self.0[action as usize];
        row.retain(|b| std::mem::discriminant(b) != std::mem::discriminant(&button));
        row.push(button);
        Ok(())
    }
}

/// Which input device a button came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonClass {
    Keyboard,
    Pad,
}

impl ButtonClass {
    fn matches(self, button: &Button) -> bool {
        match self {
            Self::Keyboard => matches!(button, Button::Key(_)),
            Self::Pad => matches!(button, Button::Pad(_)),
        }
    }
}

/// One tick's gameplay input: the joypad plus the host-only spin button.
#[derive(Debug, Clone, Copy, Default)]
pub struct Controls {
    pub keys: PlayerKeys,
    pub spin: bool,
}

#[derive(Clone, Default)]
pub struct InputState {
    held: BTreeSet<Button>,
    pressed: BTreeSet<Button>,
    blocked: BTreeSet<Button>,
}

impl InputState {
    pub fn event(&mut self, button: Button, down: bool) {
        if down {
            if self.held.insert(button.clone()) && !self.blocked.contains(&button) {
                self.pressed.insert(button);
            }
        } else {
            self.held.remove(&button);
            self.blocked.remove(&button);
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn disconnect_controller(&mut self) {
        self.held.retain(|b| matches!(b, Button::Key(_)));
        self.pressed.retain(|b| matches!(b, Button::Key(_)));
        self.blocked.retain(|b| matches!(b, Button::Key(_)));
    }

    /// Menu confirmation or a binding capture must not become a gameplay press.
    pub fn suppress_until_release(&mut self) {
        self.blocked.extend(self.held.iter().cloned());
        self.pressed.clear();
    }

    pub fn take_press(&mut self, button: &Button) -> bool {
        self.pressed.remove(button)
    }

    /// Whether `button` was newly pressed this frame, without consuming it.
    pub fn press_pending(&self, button: &Button) -> bool {
        self.pressed.contains(button)
    }

    /// Whether this device has any pending press, without consuming it.
    #[cfg(feature = "native")]
    pub(crate) fn any_press(&self, class: ButtonClass) -> bool {
        self.pressed.iter().any(|button| class.matches(button))
    }

    pub fn first_press(&self) -> Option<Button> {
        self.pressed.first().cloned()
    }

    /// Copy one device class's state into this stream, so a filtered menu
    /// stream can carry only the devices that own a player.
    #[cfg(any(feature = "native", test))]
    pub(crate) fn merge_class(&mut self, other: &Self, class: ButtonClass) {
        self.pressed
            .extend(other.pressed.iter().filter(|b| class.matches(b)).cloned());
        self.held
            .extend(other.held.iter().filter(|b| class.matches(b)).cloned());
        self.blocked
            .extend(other.blocked.iter().filter(|b| class.matches(b)).cloned());
    }

    /// Aggregate connected pads into the merged host stream. Any pad holding
    /// a button holds it, so another pad's release cannot erase the press.
    #[cfg(any(feature = "native", test))]
    pub(crate) fn merge_pads<'a>(&mut self, pads: impl IntoIterator<Item = &'a Self>) {
        let pads: Vec<&Self> = pads.into_iter().collect();
        for button in PadButton::ALL {
            let button = Button::Pad(button);
            let down = pads
                .iter()
                .any(|pad| pad.held.contains(&button) || pad.pressed.contains(&button));
            self.event(button, down);
        }
    }

    /// Remove presses that a filtered menu stream already consumed, so they
    /// cannot reach the game tick or fire again on the next frame.
    #[cfg(any(feature = "native", test))]
    pub(crate) fn retain_pressed(&mut self, view: &Self) {
        self.pressed.retain(|button| view.pressed.contains(button));
    }

    pub fn sample_player(&mut self, bindings: &Bindings) -> PlayerKeys {
        self.sample_controls(bindings).keys
    }

    pub fn sample_controls(&mut self, bindings: &Bindings) -> Controls {
        let controls = self.read_controls(bindings, None, None);
        self.consume_presses(bindings);
        controls
    }

    /// Sample only keyboard buttons. The shared event stream also carries
    /// every gamepad's buttons, so a keyboard player must not see those.
    pub fn sample_keyboard(&mut self, bindings: &Bindings) -> Controls {
        let controls = self.read_controls(bindings, None, Some(ButtonClass::Keyboard));
        self.consume_presses_for(bindings, ButtonClass::Keyboard);
        controls
    }

    /// Sample only gamepad buttons, for one pad's own event stream.
    pub fn sample_pad(&mut self, bindings: &Bindings) -> Controls {
        let controls = self.read_controls(bindings, None, Some(ButtonClass::Pad));
        self.consume_presses_for(bindings, ButtonClass::Pad);
        controls
    }

    /// Sample both keyboard players from the same event stream. Both reads
    /// happen before presses are consumed, and a button bound by both players
    /// reaches neither, so a hand-edited duplicate can never drive two players.
    pub fn sample_two_players(
        &mut self,
        first: &Bindings,
        second: &Bindings,
    ) -> (Controls, Controls) {
        let one = self.read_controls(first, Some(second), None);
        let two = self.read_controls(second, Some(first), None);
        self.consume_presses(first);
        self.consume_presses(second);
        (one, two)
    }

    fn read_controls(
        &self,
        bindings: &Bindings,
        also_bound: Option<&Bindings>,
        class: Option<ButtonClass>,
    ) -> Controls {
        let values: [bool; 9] = std::array::from_fn(|index| {
            bindings.0[index].iter().any(|b| {
                if class.is_some_and(|class| !class.matches(b)) {
                    return false;
                }
                if also_bound
                    .is_some_and(|other| other.0.iter().flatten().any(|shared| shared == b))
                {
                    return false;
                }
                !self.blocked.contains(b) && (self.held.contains(b) || self.pressed.contains(b))
            })
        });
        // Opposite directions cancel consistently across keyboard and gamepad.
        let keys = PlayerKeys {
            right: values[0] && !values[1],
            left: values[1] && !values[0],
            up: values[2] && !values[3],
            down: values[3] && !values[2],
            a: values[4],
            b: values[5],
            start: values[6],
            select: values[7],
        };
        Controls {
            keys,
            spin: values[8],
        }
    }

    fn consume_presses(&mut self, bindings: &Bindings) {
        for row in &bindings.0 {
            for button in row {
                self.pressed.remove(button);
            }
        }
    }

    fn consume_presses_for(&mut self, bindings: &Bindings, class: ButtonClass) {
        for row in &bindings.0 {
            for button in row {
                if class.matches(button) {
                    self.pressed.remove(button);
                }
            }
        }
    }
}

/// A stick uses separate press/release thresholds so noise cannot chatter keys.
#[derive(Default)]
pub struct StickAxis {
    direction: i8,
}
impl StickAxis {
    pub fn update(&mut self, value: f32, dead_zone: f32) -> i8 {
        let threshold = if self.direction != 0 && (value > 0.0) == (self.direction > 0) {
            dead_zone * 0.7
        } else {
            dead_zone
        };
        self.direction = if value.abs() < threshold || !value.is_finite() {
            0
        } else if value > 0.0 {
            1
        } else {
            -1
        };
        self.direction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_and_controller_defaults_reach_the_same_native_joypad_bytes() {
        let mut keyboard = InputState::default();
        for name in ["ArrowRight", "Z", "X"] {
            keyboard.event(Button::key(name), true);
        }
        let mut controller = InputState::default();
        for button in [PadButton::Right, PadButton::South, PadButton::West] {
            controller.event(Button::Pad(button), true);
        }
        let pack = |keys: PlayerKeys| {
            sml2_runtime::session::pack_joypad(
                keys.right,
                keys.left,
                keys.up,
                keys.down,
                keys.a,
                keys.b,
                keys.select,
                keys.start,
            )
        };
        assert_eq!(pack(keyboard.sample_player(&Bindings::default())), 0x13);
        assert_eq!(pack(controller.sample_player(&Bindings::default())), 0x13);
    }
    #[test]
    fn short_taps_survive_until_a_tick_and_menu_buttons_wait_for_release() {
        let mut input = InputState::default();
        let bindings = Bindings::default();
        input.event(Button::key("Z"), true);
        input.event(Button::key("Z"), false);
        assert!(input.sample_player(&bindings).a);
        assert!(!input.sample_player(&bindings).a);
        input.event(Button::Pad(PadButton::South), true);
        input.suppress_until_release();
        assert!(!input.sample_player(&bindings).a);
        input.event(Button::Pad(PadButton::South), false);
        input.event(Button::Pad(PadButton::South), true);
        assert!(input.sample_player(&bindings).a);
    }
    #[test]
    fn focus_loss_and_disconnect_release_inputs_and_rebinding_preserves_device_classes() {
        let mut bindings = Bindings::default();
        bindings.rebind(Action::Jump, Button::key("K")).unwrap();
        assert!(bindings.0[Action::Jump as usize].contains(&Button::Pad(PadButton::South)));
        assert!(bindings.rebind(Action::Jump, Button::key("X")).is_err());
        let mut input = InputState::default();
        input.event(Button::key("ArrowRight"), true);
        input.event(Button::Pad(PadButton::South), true);
        input.disconnect_controller();
        let keys = input.sample_player(&bindings);
        assert!(keys.right && !keys.a);
        input.clear();
        assert!(!input.sample_player(&bindings).right);
    }
    #[test]
    fn zl_and_zr_drive_the_spin_action_without_touching_the_joypad() {
        let bindings = Bindings::default();
        for trigger in [PadButton::LeftTrigger, PadButton::RightTrigger] {
            let mut input = InputState::default();
            input.event(Button::Pad(trigger), true);
            let controls = input.sample_controls(&bindings);
            assert!(controls.spin);
            assert!(!controls.keys.down);
        }
        let mut unbound = bindings.clone();
        unbound.0[Action::Spin as usize].clear();
        assert!(unbound.validate().is_ok());
        unbound.0[Action::Jump as usize].clear();
        assert!(unbound.validate().is_err());
    }
    #[test]
    fn stick_hysteresis_and_opposite_directions_are_stable() {
        let mut axis = StickAxis::default();
        assert_eq!(axis.update(0.25, 0.3), 0);
        assert_eq!(axis.update(0.35, 0.3), 1);
        assert_eq!(axis.update(0.25, 0.3), 1);
        assert_eq!(axis.update(-0.25, 0.3), 0);
        assert_eq!(axis.update(-0.35, 0.3), -1);
        assert_eq!(axis.update(0.15, 0.3), 0);
        let mut input = InputState::default();
        input.event(Button::key("ArrowRight"), true);
        input.event(Button::Pad(PadButton::Left), true);
        let keys = input.sample_player(&Bindings::default());
        assert!(!keys.right && !keys.left);
    }
    #[test]
    fn both_keyboard_players_sample_independently_without_overlapping_defaults() {
        let first = Bindings::default();
        let second = Bindings::player_two();
        second.validate().unwrap();
        for action in Action::ALL {
            for button in &first.0[action as usize] {
                assert!(
                    !second.0.iter().flatten().any(|other| other == button),
                    "{} is bound to both players",
                    button.label()
                );
            }
        }

        let mut input = InputState::default();
        input.event(Button::key("ArrowRight"), true);
        input.event(Button::key("D"), true);
        input.event(Button::key("W"), true);
        let (one, two) = input.sample_two_players(&first, &second);
        assert!(one.keys.right && !one.keys.up);
        assert!(two.keys.right && two.keys.up && !two.keys.left);

        // Pausing blocks both players until each held key is released.
        input.event(Button::key("ArrowRight"), true);
        input.event(Button::key("D"), true);
        input.suppress_until_release();
        let (one, two) = input.sample_two_players(&first, &second);
        assert!(!one.keys.right && !two.keys.right);
        for name in ["ArrowRight", "D"] {
            input.event(Button::key(name), false);
            input.event(Button::key(name), true);
        }
        let (one, two) = input.sample_two_players(&first, &second);
        assert!(one.keys.right && two.keys.right);

        // Window focus loss releases held keys for both players.
        input.clear();
        let (one, two) = input.sample_two_players(&first, &second);
        assert!(!one.keys.right && !two.keys.right && !two.keys.up);
    }
    #[test]
    fn each_device_samples_only_its_own_button_class() {
        let bindings = Bindings::default();
        let mut input = InputState::default();
        input.event(Button::key("ArrowRight"), true);
        input.event(Button::Pad(PadButton::South), true);
        let keyboard = input.sample_keyboard(&bindings);
        assert!(keyboard.keys.right);
        assert!(!keyboard.keys.a, "the pad is another device's input");
        let pad = input.sample_pad(&bindings);
        assert!(pad.keys.a);
        assert!(!pad.keys.right, "the keyboard is another device's input");
    }

    #[test]
    fn merged_pad_state_aggregates_every_connected_pad() {
        let south = Button::Pad(PadButton::South);
        let mut first = InputState::default();
        let mut second = InputState::default();
        let mut merged = InputState::default();
        first.event(south.clone(), true);
        second.event(south.clone(), true);
        merged.merge_pads([&first, &second]);
        assert!(merged.held.contains(&south));
        // Model the host consuming the latched taps; holding is what keeps
        // the aggregate down.
        merged.take_press(&south);
        first.take_press(&south);
        second.take_press(&south);
        // One pad releasing must not erase the other pad's held button.
        second.event(south.clone(), false);
        merged.merge_pads([&first, &second]);
        assert!(merged.held.contains(&south));
        first.event(south.clone(), false);
        merged.merge_pads([&first, &second]);
        assert!(!merged.held.contains(&south));
    }

    #[test]
    fn a_filtered_stream_consumes_only_the_presses_it_sees() {
        let south = Button::Pad(PadButton::South);
        let mut pad = InputState::default();
        pad.event(south.clone(), true);
        let mut view = InputState::default();
        view.merge_class(&pad, ButtonClass::Pad);
        assert!(view.take_press(&south));
        pad.retain_pressed(&view);
        assert!(!pad.press_pending(&south), "the menu consumed it");
    }

    #[test]
    fn loaded_conflicts_are_reported_and_never_drive_both_players() {
        let second = Bindings::player_two();
        let mut first = Bindings::default();
        first.0[Action::Jump as usize] = vec![Button::key("D")];
        assert_eq!(
            first.conflicts_with(&second),
            vec![(Action::Jump, vec![Button::key("D")])]
        );

        let mut input = InputState::default();
        input.event(Button::key("D"), true);
        let (one, two) = input.sample_two_players(&first, &second);
        assert!(!one.keys.a && !two.keys.right);
    }
}
