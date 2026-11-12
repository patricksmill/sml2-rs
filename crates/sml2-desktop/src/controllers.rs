//! Platform controller hosts. macOS uses GameController, never IOKit/gilrs.
//!
//! [`Controllers::poll`] keeps the original single-device host API used by the
//! scenery editor. The desktop game host uses [`Controllers::poll_devices`],
//! which tracks every connected pad separately so input devices can belong to
//! different players.

/// One pad appeared or vanished since the previous poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceChange {
    Connected { id: u64, name: String },
    Disconnected { id: u64 },
}

#[cfg(all(feature = "native", not(target_os = "macos")))]
mod gilrs;
#[cfg(all(feature = "native", not(target_os = "macos")))]
pub use self::gilrs::Controllers;

#[cfg(all(feature = "native", target_os = "macos"))]
mod macos;
#[cfg(all(feature = "native", target_os = "macos"))]
pub use macos::Controllers;

#[cfg(any(target_os = "macos", test))]
mod native_state;
