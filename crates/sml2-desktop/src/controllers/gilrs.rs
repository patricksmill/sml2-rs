use super::DeviceChange;
use crate::input::{Button, InputState, PadButton, StickAxis};
use gilrs::{Axis, Button as GilButton, EventType, GamepadId, Gilrs};
use std::collections::BTreeMap;

const BUTTONS: [(GilButton, PadButton); 16] = [
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
    (GilButton::LeftTrigger2, PadButton::LeftTrigger),
    (GilButton::RightTrigger2, PadButton::RightTrigger),
];

pub struct Controllers {
    gilrs: Option<Gilrs>,
    selected: Option<GamepadId>,
    x: StickAxis,
    y: StickAxis,
    /// One stick pair per connected pad, keyed like `pads`.
    axes: BTreeMap<u64, (StickAxis, StickAxis)>,
    /// Stable host ids of the pads seen last poll.
    known: Vec<u64>,
    pub error: Option<String>,
}

fn identity(gamepad: &gilrs::Gamepad<'_>) -> String {
    gamepad.uuid().iter().map(|b| format!("{b:02x}")).collect()
}

fn pad_id(id: GamepadId) -> u64 {
    usize::from(id) as u64
}

impl Controllers {
    /// Construct a host without opening controller devices.
    pub fn disconnected() -> Self {
        Self {
            gilrs: None,
            selected: None,
            x: StickAxis::default(),
            y: StickAxis::default(),
            axes: BTreeMap::new(),
            known: Vec::new(),
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
            axes: BTreeMap::new(),
            known: Vec::new(),
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

    /// Poll every pad separately, feeding each one's own state as well as the
    /// merged host-event stream. Returns pads that appeared or vanished.
    pub fn poll_devices(
        &mut self,
        merged: &mut InputState,
        pads: &mut BTreeMap<u64, InputState>,
        dead_zone: f32,
    ) -> Vec<DeviceChange> {
        let Some(g) = &mut self.gilrs else {
            return Vec::new();
        };
        let mut events = Vec::new();
        while let Some(event) = g.next_event() {
            events.push(event);
        }
        let connected: Vec<(u64, GamepadId, String)> = g
            .gamepads()
            .map(|(id, pad)| (pad_id(id), id, pad.name().to_owned()))
            .collect();
        let live: Vec<u64> = connected.iter().map(|(id, _, _)| *id).collect();
        let mut changes = Vec::new();

        let stale: Vec<u64> = self
            .known
            .iter()
            .copied()
            .filter(|id| !live.contains(id))
            .collect();
        for id in stale {
            self.known.retain(|known| *known != id);
            self.axes.remove(&id);
            pads.remove(&id);
            changes.push(DeviceChange::Disconnected { id });
        }

        for (id, _, name) in &connected {
            if !self.known.contains(id) {
                self.known.push(*id);
                self.axes
                    .insert(*id, (StickAxis::default(), StickAxis::default()));
                pads.entry(*id).or_default();
                changes.push(DeviceChange::Connected {
                    id: *id,
                    name: name.clone(),
                });
            }
        }

        for (id, gamepad_id, _) in connected {
            let Some(state) = pads.get_mut(&id) else {
                continue;
            };
            let (x_axis, y_axis) = self.axes.entry(id).or_default();
            // Preserve press/release pairs that both arrived between redraws.
            for event in events.iter().filter(|event| event.id == gamepad_id) {
                match event.event {
                    EventType::ButtonPressed(button, _) | EventType::ButtonReleased(button, _) => {
                        if let Some((_, mapped)) =
                            BUTTONS.iter().find(|(native, _)| *native == button)
                        {
                            let down = matches!(event.event, EventType::ButtonPressed(..));
                            state.event(Button::Pad(*mapped), down);
                        }
                    }
                    EventType::AxisChanged(axis, value, _) => {
                        let (negative, positive, direction) = match axis {
                            Axis::LeftStickX => (
                                PadButton::StickLeft,
                                PadButton::StickRight,
                                x_axis.update(value, dead_zone),
                            ),
                            Axis::LeftStickY => (
                                PadButton::StickDown,
                                PadButton::StickUp,
                                y_axis.update(value, dead_zone),
                            ),
                            _ => continue,
                        };
                        state.event(Button::Pad(negative), direction < 0);
                        state.event(Button::Pad(positive), direction > 0);
                    }
                    _ => {}
                }
            }
            let pad = g.gamepad(gamepad_id);
            for (native, mapped) in BUTTONS {
                let down = pad.is_pressed(native);
                state.event(Button::Pad(mapped), down);
            }
            let x = x_axis.update(pad.value(Axis::LeftStickX), dead_zone);
            let y = y_axis.update(pad.value(Axis::LeftStickY), dead_zone);
            for (button, down) in [
                (PadButton::StickLeft, x < 0),
                (PadButton::StickRight, x > 0),
                (PadButton::StickUp, y > 0),
                (PadButton::StickDown, y < 0),
            ] {
                state.event(Button::Pad(button), down);
            }
        }
        // One aggregate for host menus: any pad holding a button holds it.
        merged.merge_pads(pads.values());
        changes
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
