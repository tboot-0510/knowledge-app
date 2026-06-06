//! Platform integration layer.
//!
//! All macOS-specific behaviour (NSPanel overlay, Accessory activation policy,
//! launchd scheduling) is isolated here behind `cfg` so the rest of the crate is
//! platform-neutral. On non-macOS targets a no-op-ish [`stub`] implementation
//! with the same function signatures is used instead.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(target_os = "macos"))]
mod stub;
#[cfg(not(target_os = "macos"))]
pub use stub::*;
