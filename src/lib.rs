//! The modules the `claude-acc` binary is built from, as a library.
//!
//! Not a published API. [`gui`] is the one module meant for other front ends;
//! the rest are `pub` only because the binary is a separate crate, and they
//! change whenever the CLI needs them to.

pub mod gui;

#[doc(hidden)]
pub mod config;
#[doc(hidden)]
pub mod environment;
#[doc(hidden)]
pub mod identity;
#[doc(hidden)]
pub mod import;
#[doc(hidden)]
pub mod quoting;
#[doc(hidden)]
pub mod resolve;
#[doc(hidden)]
pub mod seed;
#[doc(hidden)]
pub mod sessions;
#[doc(hidden)]
pub mod statusline;
#[doc(hidden)]
pub mod trash;
#[doc(hidden)]
pub mod vscode;
