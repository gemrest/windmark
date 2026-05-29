#![deny(
  clippy::all,
  clippy::nursery,
  clippy::pedantic,
  future_incompatible,
  nonstandard_style,
  rust_2018_idioms,
  unsafe_code,
  unused,
  warnings
)]
#![allow(clippy::module_name_repetitions)]
#![doc = include_str!("../README.md")]

#[cfg(all(feature = "tokio", feature = "async-std"))]
compile_error!(
  "the `tokio` and `async-std` features are mutually exclusive; enable \
   exactly one (windmark enables `tokio` by default, so set `default-features \
   = false` to use `async-std`)"
);
#[cfg(not(any(feature = "tokio", feature = "async-std")))]
compile_error!(
  "a runtime feature must be enabled: `tokio` (the default) or `async-std`"
);

pub mod context;
pub mod handler;
pub mod module;
pub mod prelude;
pub mod response;
pub mod router;
pub mod router_option;
pub mod utilities;

#[macro_use]
extern crate log;

#[cfg(feature = "async-std")]
pub use async_std::main;
#[cfg(feature = "tokio")]
pub use tokio::main;
