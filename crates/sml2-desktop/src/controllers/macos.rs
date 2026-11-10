//! GameController host. All Objective-C ownership stays on the main/UI thread.
use super::native_state::{self, Device, NativeState, Snapshot};
use crate::input::InputState;
use block2::RcBlock;
use objc2::{
    MainThreadMarker,
    rc::Retained,
    runtime::{Bool, ProtocolObject},
    sel,
};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol, NSOperationQueue};
use objc2_game_controller::{
    GCController, GCControllerAxisInput, GCControllerButtonInput,
    GCControllerDidConnectNotification, GCControllerDidDisconnectNotification, GCDevice,
    GCExtendedGamepad,
};
use std::{
    ptr::NonNull,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
};

enum Message {
    Refresh,
    Removed(usize),
    Input(u64, Snapshot),
}

struct AppleDevice {
    controller: Retained<GCController>,
    profile: Retained<GCExtendedGamepad>,
    device: Device,
}

impl AppleDevice {
    fn address(&self) -> usize {
        Retained::as_ptr(&self.controller) as usize
    }
}

impl Drop for AppleDevice {
    fn drop(&mut self) {
        // SAFETY: The profile is retained, and the host is confined to the UI thread.
        unsafe {
            for button in buttons(&self.profile).into_iter().flatten() {
                button.setPressedChangedHandler(std::ptr::null_mut());
            }
            let stick = self.profile.leftThumbstick();
            stick.xAxis().setValueChangedHandler(std::ptr::null_mut());
            stick.yAxis().setValueChangedHandler(std::ptr::null_mut());
        }
    }
}

pub struct Controllers {
    state: NativeState,
    devices: Vec<AppleDevice>,
    observers: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    next_id: u64,
    // MainThreadMarker is !Send/!Sync; disconnected test hosts need no marker.
    main_thread: Option<MainThreadMarker>,
    pub error: Option<String>,
}

impl Controllers {
    pub fn disconnected() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            state: NativeState::default(),
            devices: Vec::new(),
            observers: Vec::new(),
            tx,
            rx,
            next_id: 0,
            main_thread: None,
            error: None,
        }
    }

    pub fn new() -> Self {
        let mut host = Self::disconnected();
        let Some(marker) = MainThreadMarker::new() else {
            host.error =
                Some("Controller input unavailable: GameController requires the UI thread".into());
            return host;
        };
        host.main_thread = Some(marker);
        // SAFETY: Registration and retained observer ownership stay on the main thread.
        // Blocks capture only sendable Rust channel senders, never Objective-C objects.
        unsafe {
            let center = NSNotificationCenter::defaultCenter();
            for (name, removed) in [
                (GCControllerDidConnectNotification, false),
                (GCControllerDidDisconnectNotification, true),
            ] {
                let tx = host.tx.clone();
                let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
                    let message = if removed {
                        match notification.as_ref().object() {
                            Some(object) => Message::Removed(Retained::as_ptr(&object) as usize),
                            None => Message::Refresh,
                        }
                    } else {
                        Message::Refresh
                    };
                    let _ = tx.send(message);
                });
                host.observers
                    .push(center.addObserverForName_object_queue_usingBlock(
                        Some(name),
                        None,
                        Some(&NSOperationQueue::mainQueue()),
                        &block,
                    ));
            }
        }
        host.refresh(&[]);
        host
    }

    pub fn name(&self) -> String {
        self.state.name(
            &self
                .devices
                .iter()
                .map(|d| d.descriptor())
                .collect::<Vec<_>>(),
        )
    }

    pub fn cycle(&mut self, input: &mut InputState) -> Option<String> {
        if self.main_thread.is_none() {
            return None;
        }
        // Discard queued input before an explicit device switch, retaining lifecycle changes.
        let (removed, _) = self.drain();
        self.refresh(&removed);
        let devices = self.descriptors();
        self.state.cycle(&devices, input)
    }

    pub fn poll(
        &mut self,
        input: &mut InputState,
        preference: Option<&str>,
        dead_zone: f32,
    ) -> bool {
        if self.main_thread.is_none() {
            return false;
        }
        let (removed, events) = self.drain();
        self.refresh(&removed);
        let devices = self.descriptors();
        self.state
            .poll(&devices, events, input, preference, dead_zone)
    }

    fn descriptors(&self) -> Vec<Device> {
        self.devices.iter().map(|d| d.descriptor()).collect()
    }

    fn drain(&self) -> (Vec<usize>, Vec<(u64, Snapshot)>) {
        let mut removed = Vec::new();
        let mut events = Vec::new();
        for message in self.rx.try_iter() {
            match message {
                Message::Removed(address) => removed.push(address),
                Message::Input(id, snapshot) => events.push((id, snapshot)),
                Message::Refresh => {}
            }
        }
        (removed, events)
    }

    fn refresh(&mut self, removed: &[usize]) {
        // SAFETY: Main-thread-only host; all framework objects are retained for their use.
        // Enumerating also reconciles changes before a queued notification is delivered.
        unsafe {
            let controllers = GCController::controllers().to_vec();
            self.devices.retain(|d| {
                !removed.contains(&d.address())
                    && controllers
                        .iter()
                        .any(|c| Retained::as_ptr(c) as usize == d.address())
            });
            for controller in controllers {
                let address = Retained::as_ptr(&controller) as usize;
                if self.devices.iter().any(|d| d.address() == address) {
                    continue;
                }
                let Some(profile) = controller.extendedGamepad() else {
                    continue;
                };
                let vendor = controller
                    .vendorName()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Game controller".into());
                let category = if controller.respondsToSelector(sel!(productCategory)) {
                    controller.productCategory().to_string()
                } else {
                    "Extended gamepad".into()
                };
                self.next_id += 1;
                let id = self.next_id;
                bind_inputs(&profile, id, &self.tx);
                self.devices.push(AppleDevice {
                    device: Device {
                        id,
                        name: vendor.clone(),
                        preference: native_state::preference(&vendor, &category),
                        snapshot: Snapshot::default(),
                    },
                    controller,
                    profile,
                });
            }
            for d in &mut self.devices {
                d.device.snapshot = snapshot(&d.profile);
            }
        }
    }
}

impl AppleDevice {
    fn descriptor(&self) -> Device {
        Device {
            id: self.device.id,
            name: self.device.name.clone(),
            preference: self.device.preference.clone(),
            snapshot: self.device.snapshot,
        }
    }
}

impl Drop for Controllers {
    fn drop(&mut self) {
        if self.main_thread.is_none() {
            return;
        }
        // SAFETY: Retained observer tokens, on their registration thread.
        unsafe {
            let center = NSNotificationCenter::defaultCenter();
            for observer in self.observers.drain(..) {
                center.removeObserver((*observer).as_ref());
            }
        }
        self.devices.clear(); // unregister input handlers before dropping the receiver
    }
}

/// SAFETY: Call only with a live profile on its main handler/UI thread.
unsafe fn buttons(pad: &GCExtendedGamepad) -> [Option<Retained<GCControllerButtonInput>>; 16] {
    let dpad = pad.dpad();
    // Guard selectors introduced after the original extended-gamepad API.
    let options = pad
        .respondsToSelector(sel!(buttonOptions))
        .then(|| pad.buttonOptions())
        .flatten();
    let left_click = pad
        .respondsToSelector(sel!(leftThumbstickButton))
        .then(|| pad.leftThumbstickButton())
        .flatten();
    let right_click = pad
        .respondsToSelector(sel!(rightThumbstickButton))
        .then(|| pad.rightThumbstickButton())
        .flatten();
    let menu = pad
        .respondsToSelector(sel!(buttonMenu))
        .then(|| pad.buttonMenu());
    // Same positional order as native_state::BUTTONS.
    [
        Some(pad.buttonA()),
        Some(pad.buttonB()),
        Some(pad.buttonX()),
        Some(pad.buttonY()),
        Some(dpad.up()),
        Some(dpad.down()),
        Some(dpad.left()),
        Some(dpad.right()),
        menu,
        options,
        left_click,
        right_click,
        Some(pad.leftShoulder()),
        Some(pad.rightShoulder()),
        Some(pad.leftTrigger()),
        Some(pad.rightTrigger()),
    ]
}

/// SAFETY: Call only with a live profile on its main handler/UI thread.
unsafe fn snapshot(pad: &GCExtendedGamepad) -> Snapshot {
    let stick = pad.leftThumbstick();
    Snapshot {
        buttons: buttons(pad).map(|button| button.is_some_and(|b| b.isPressed())),
        left: [stick.xAxis().value(), stick.yAxis().value()],
    }
}

/// SAFETY: The profile is retained by AppleDevice; its handlers default to the main queue.
unsafe fn bind_inputs(pad: &GCExtendedGamepad, id: u64, tx: &Sender<Message>) {
    // Use the scalar values supplied by callbacks, not a later read of the live profile:
    // a press and release can both be dispatched before the next redraw.
    // No block captures an Objective-C object, avoiding retain cycles.
    let current = Arc::new(Mutex::new(snapshot(pad)));
    for (index, button) in buttons(pad).into_iter().enumerate() {
        let Some(button) = button else {
            continue;
        };
        let current = Arc::clone(&current);
        let tx = tx.clone();
        let block = RcBlock::new(
            move |_: NonNull<GCControllerButtonInput>, _: f32, pressed: Bool| {
                let mut state = current.lock().unwrap();
                state.buttons[index] = pressed.as_bool();
                let _ = tx.send(Message::Input(id, *state));
            },
        );
        button.setPressedChangedHandler(RcBlock::as_ptr(&block));
    }
    let stick = pad.leftThumbstick();
    for (index, axis) in [stick.xAxis(), stick.yAxis()].into_iter().enumerate() {
        let current = Arc::clone(&current);
        let tx = tx.clone();
        let block = RcBlock::new(move |_: NonNull<GCControllerAxisInput>, value: f32| {
            let mut state = current.lock().unwrap();
            state.left[index] = value;
            let _ = tx.send(Message::Input(id, *state));
        });
        axis.setValueChangedHandler(RcBlock::as_ptr(&block));
    }
}

#[cfg(test)]
#[path = "macos_tests.rs"]
mod tests;
