//! Device-independent policy for Apple's event stream; also tested off macOS.
use crate::input::{Button, InputState, PadButton, StickAxis};

pub(super) const BUTTONS: [PadButton; 16] = {
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
    ]
};

#[derive(Clone, Copy, Default)]
pub(super) struct Snapshot {
    pub buttons: [bool; 16],
    /// Apple axes: right and up are positive.
    pub left: [f32; 2],
}

pub(super) struct Device {
    pub id: u64,
    pub name: String,
    pub preference: String,
    pub snapshot: Snapshot,
}

pub(super) fn preference(vendor: &str, category: &str) -> String {
    // JSON escaping avoids ambiguous model names without inventing a settings schema.
    format!(
        "apple-gc:v1:{}",
        serde_json::to_string(&(vendor, category)).unwrap()
    )
}

#[derive(Default)]
pub(super) struct NativeState {
    selected: Option<u64>,
    x: StickAxis,
    y: StickAxis,
}

impl NativeState {
    pub fn name(&self, devices: &[Device]) -> String {
        self.current(devices)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "No controller connected".into())
    }

    fn current<'a>(&self, devices: &'a [Device]) -> Option<&'a Device> {
        devices.iter().find(|d| Some(d.id) == self.selected)
    }

    fn select(&mut self, id: Option<u64>, input: &mut InputState) {
        input.disconnect_controller();
        self.x = StickAxis::default();
        self.y = StickAxis::default();
        self.selected = id;
    }

    pub fn cycle(&mut self, devices: &[Device], input: &mut InputState) -> Option<String> {
        if devices.is_empty() {
            return None;
        }
        let next = devices
            .iter()
            .position(|d| Some(d.id) == self.selected)
            .map_or(0, |i| (i + 1) % devices.len());
        self.select(Some(devices[next].id), input);
        Some(devices[next].preference.clone())
    }

    pub fn poll(
        &mut self,
        devices: &[Device],
        events: impl IntoIterator<Item = (u64, Snapshot)>,
        input: &mut InputState,
        preference: Option<&str>,
        dead_zone: f32,
    ) -> bool {
        let current = self.current(devices);
        let preferred = devices
            .iter()
            .find(|d| Some(d.preference.as_str()) == preference);
        let selected = current
            .filter(|d| Some(d.preference.as_str()) == preference)
            .or(preferred)
            .or(current)
            .or_else(|| devices.first())
            .map(|d| d.id);
        let changed = self.selected.is_some() && self.selected != selected;
        if self.selected != selected {
            self.select(selected, input);
        }
        let Some(device) = self.current(devices) else {
            return changed;
        };
        let id = device.id;
        let latest = device.snapshot;
        for (source, snapshot) in events {
            if source == id {
                self.apply(snapshot, input, dead_zone);
            }
        }
        // Reconcile held input, including controllers connected before launch.
        self.apply(latest, input, dead_zone);
        changed
    }

    fn apply(&mut self, snapshot: Snapshot, input: &mut InputState, dead_zone: f32) {
        for (button, down) in BUTTONS.into_iter().zip(snapshot.buttons) {
            input.event(Button::Pad(button), down);
        }
        let x = self.x.update(snapshot.left[0], dead_zone);
        let y = self.y.update(snapshot.left[1], dead_zone);
        for (button, down) in [
            (PadButton::StickLeft, x < 0),
            (PadButton::StickRight, x > 0),
            (PadButton::StickDown, y < 0),
            (PadButton::StickUp, y > 0),
        ] {
            input.event(Button::Pad(button), down);
        }
    }
}

#[cfg(test)]
#[path = "native_state_tests.rs"]
mod tests;
