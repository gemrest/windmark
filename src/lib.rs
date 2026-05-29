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

pub mod context;
pub mod handler;
pub mod module;
#[cfg(feature = "prelude")]
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
