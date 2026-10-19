use crate::input::{Button, InputState, PadButton, StickAxis};
use gilrs::{Axis, Button as GilButton, EventType, GamepadId, Gilrs};

const BUTTONS: [(GilButton, PadButton); 14] = [
    (GilButton::South, PadButton::South),
    (GilButton::East, PadButton::East),
    (GilButton::West, PadButton::West),
    (GilButton::North, PadButton::North),
    (GilButton::DPadUp, PadButton::Up),
    (GilButton::DPadDown, PadButton::Down),
    (GilButton::DPadLeft, PadButton::Left),
    (GilButton::DPadRight, PadButton::Right),
    (GilButton::Start, PadButton::Start),
    (GilButton::Select, PadButton::Select),
    (GilButton::LeftThumb, PadButton::LeftThumb),
    (GilButton::RightThumb, PadButton::RightThumb),
    (GilButton::LeftTrigger, PadButton::LeftShoulder),
    (GilButton::RightTrigger, PadButton::RightShoulder),
];

pub struct Controllers {
    gilrs: Option<Gilrs>,
    selected: Option<GamepadId>,
    x: StickAxis,
    y: StickAxis,
    pub error: Option<String>,
}

fn identity(gamepad: &gilrs::Gamepad<'_>) -> String {
    gamepad.uuid().iter().map(|b| format!("{b:02x}")).collect()
}

impl Controllers {
    #[cfg(test)]
    pub fn disconnected() -> Self {
        Self {
            gilrs: None,
            selected: None,
            x: StickAxis::default(),
            y: StickAxis::default(),
            error: None,
        }
    }
    pub fn new() -> Self {
        let (gilrs, error) = match Gilrs::new() {
            Ok(g) => (Some(g), None),
            Err(e) => (None, Some(format!("Controller input unavailable: {e}"))),
        };
        Self {
            gilrs,
            selected: None,
            x: StickAxis::default(),
            y: StickAxis::default(),
            error,
        }
    }

    pub fn name(&self) -> String {
        self.gilrs
            .as_ref()
            .and_then(|g| self.selected.map(|id| g.gamepad(id).name().to_owned()))
            .unwrap_or_else(|| "No controller connected".into())
    }

    pub fn cycle(&mut self, input: &mut InputState) -> Option<String> {
        let g = self.gilrs.as_ref()?;
        let ids: Vec<_> = g.gamepads().map(|(id, _)| id).collect();
        if ids.is_empty() {
            return None;
        }
        let next = self
            .selected
            .and_then(|id| ids.iter().position(|x| *x == id))
            .map_or(0, |i| (i + 1) % ids.len());
        self.selected = Some(ids[next]);
        input.disconnect_controller();
        self.x = StickAxis::default();
        self.y = StickAxis::default();
        Some(identity(&g.gamepad(ids[next])))
    }

    pub fn poll(
        &mut self,
        input: &mut InputState,
        preference: Option<&str>,
        dead_zone: f32,
    ) -> bool {
        let Some(g) = &mut self.gilrs else {
            return false;
        };
        let mut events = Vec::new();
        while let Some(event) = g.next_event() {
            events.push(event);
        }
        let preferred = g
            .gamepads()
            .find(|(_, pad)| Some(identity(pad).as_str()) == preference)
            .map(|(id, _)| id);
        let selected = preferred
            .or_else(|| self.selected.filter(|id| g.gamepad(*id).is_connected()))
            .or_else(|| g.gamepads().next().map(|(id, _)| id));
        let disconnected = self.selected.is_some() && selected != self.selected;
        if selected != self.selected {
            input.disconnect_controller();
            self.x = StickAxis::default();
            self.y = StickAxis::default();
            self.selected = selected;
        }
        let Some(id) = selected else {
            return disconnected;
        };
        // Preserve press/release pairs that both arrived between redraws.
        for event in events.into_iter().filter(|event| event.id == id) {
            match event.event {
                EventType::ButtonPressed(button, _) | EventType::ButtonReleased(button, _) => {
                    if let Some((_, mapped)) = BUTTONS.iter().find(|(native, _)| *native == button)
                    {
                        input.event(
                            Button::Pad(*mapped),
                            matches!(event.event, EventType::ButtonPressed(..)),
                        );
                    }
                }
                EventType::AxisChanged(axis, value, _) => {
                    let (negative, positive, direction) = match axis {
                        Axis::LeftStickX => (
                            PadButton::StickLeft,
                            PadButton::StickRight,
                            self.x.update(value, dead_zone),
                        ),
                        Axis::LeftStickY => (
                            PadButton::StickDown,
                            PadButton::StickUp,
                            self.y.update(value, dead_zone),
                        ),
                        _ => continue,
                    };
                    input.event(Button::Pad(negative), direction < 0);
                    input.event(Button::Pad(positive), direction > 0);
                }
                _ => {}
            }
        }
        let pad = g.gamepad(id);
        use PadButton::*;
        for (native, mapped) in BUTTONS {
            input.event(Button::Pad(mapped), pad.is_pressed(native));
        }
        let x = self.x.update(pad.value(Axis::LeftStickX), dead_zone);
        let y = self.y.update(pad.value(Axis::LeftStickY), dead_zone);
        for (button, down) in [
            (StickLeft, x < 0),
            (StickRight, x > 0),
            (StickUp, y > 0),
            (StickDown, y < 0),
        ] {
            input.event(Button::Pad(button), down);
        }
        disconnected
    }
}
