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
    StickUp,
    StickDown,
    StickLeft,
    StickRight,
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
            Self::Key(key) => key.clone(),
            Self::Pad(pad) => format!("Pad {pad:?}"),
        }
    }
    pub fn reserved(&self) -> bool {
        match self {
            Self::Key(key) => ["Escape", "F1", "F2", "F3", "F11", "Plus", "Equals", "Minus"]
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
}

impl Action {
    pub const ALL: [Self; 8] = [
        Self::Right,
        Self::Left,
        Self::Up,
        Self::Down,
        Self::Jump,
        Self::Run,
        Self::Start,
        Self::Select,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bindings(pub [Vec<Button>; 8]);

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
        ])
    }
}

impl Bindings {
    pub fn validate(&self) -> Result<(), String> {
        let mut used = BTreeSet::new();
        for action in Action::ALL {
            let row = &self.0[action as usize];
            if row.is_empty() {
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

#[derive(Default)]
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

    pub fn first_press(&self) -> Option<Button> {
        self.pressed.first().cloned()
    }

    pub fn sample_player(&mut self, bindings: &Bindings) -> PlayerKeys {
        let values: [bool; 8] = std::array::from_fn(|index| {
            bindings.0[index].iter().any(|b| {
                !self.blocked.contains(b) && (self.held.contains(b) || self.pressed.contains(b))
            })
        });
        for row in &bindings.0 {
            for button in row {
                self.pressed.remove(button);
            }
        }
        // Opposite directions cancel consistently across keyboard and gamepad.
        PlayerKeys {
            right: values[0] && !values[1],
            left: values[1] && !values[0],
            up: values[2] && !values[3],
            down: values[3] && !values[2],
            a: values[4],
            b: values[5],
            start: values[6],
            select: values[7],
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
}
