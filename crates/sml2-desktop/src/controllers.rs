//! Platform controller hosts. macOS uses GameController, never IOKit/gilrs.
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
