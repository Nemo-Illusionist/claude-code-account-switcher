//! The modules the `claude-acc` binary and the optional desktop app share.
//!
//! Not a published API. `gui` is the one surface the desktop app is meant to
//! use; the rest are `pub` only because the binary is a separate crate, and
//! they change whenever the CLI needs them to.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub mod gui;

#[doc(hidden)]
pub mod config;
#[doc(hidden)]
pub mod environment;
#[doc(hidden)]
pub mod identity;
#[doc(hidden)]
pub mod quoting;
#[doc(hidden)]
pub mod resolve;
#[doc(hidden)]
pub mod sessions;
