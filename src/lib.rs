// This file is part of Windmark <https://github.com/gemrest/windmark>.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Copyright (C) 2022-2023 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

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
#![doc = include_str!("../README.md")]
#![recursion_limit = "128"]

pub mod context;
pub mod handler;
pub mod module;
#[cfg(feature = "prelude")]
pub mod prelude;
pub mod response;
pub mod router;
pub mod utilities;

#[macro_use]
extern crate log;

#[cfg(feature = "async-std")]
pub use async_std::main;
#[cfg(feature = "tokio")]
pub use tokio::main;
