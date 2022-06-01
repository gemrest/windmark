// This file is part of Windmark <https://github.com/gemrest/windmark>.
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
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
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

//! # Windmark
//!
//! [![crates.io](https://img.shields.io/crates/v/windmark.svg)](https://crates.io/crates/windmark)
//! [![docs.rs](https://docs.rs/windmark/badge.svg)](https://docs.rs/windmark)
//! [![github.com](https://github.com/gemrest/windmark/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/windmark/actions/workflows/check.yaml)
//!
//! Windmark is an elegant and highly performant, async Gemini server framework
//! for the modern age!
//!
//! ## Usage
//!
//! ### Add Windmark as a dependency
//!
//! ```toml
//! # Cargo.toml
//!
//! [dependencies]
//! windmark = "0.1.14"
//! tokio = { version = "0.2.4", features = ["full"] }
//!
//! # If you would like to use the built-in logger (recommended)
//! # windmark = { version = "0.1.14", features = ["logger"] }
//!
//! # If you would like to use the built-in MIME dedection when `Success`-ing a file
//! # (recommended)
//! # windmark = { version = "0.1.14", features = ["auto-deduce-mime"] }
//! ```
//!
//! ### Implement a Windmark server
//!
//! ```rust
//! // src/main.rs
//!
//! use windmark::Response;
//!
//! #[windmark::main]
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!   windmark::Router::new()
//!     .set_private_key_file("windmark_private.pem")
//!     .set_certificate_chain_file("windmark_public.pem")
//!     .mount("/", Box::new(|_| Response::Success("Hello, World!".into())))
//!     .set_error_handler(Box::new(|_| {
//!       Response::PermanentFailure("This route does not exist!".into())
//!     }))
//!     .run()
//!     .await
//! }
//! ```
//!
//! ## Examples
//!
//! Examples can be found within the
//! [`examples/`](https://github.com/gemrest/windmark/tree/main/examples) directory.
//!
//! An example of a fully featured Gemini capsule written using Windmark can be
//! found [here](https://github.com/gemrest/locus). This example Gemini capsule also
//! happens to be the source code for [Fuwn's](https://github.com/Fuwn) (this
//! library's author) personal Gemini capsule!
//!
//! ## Modules
//!
//! Modules are reusable extensions which can be procedurally mounted onto
//! Windmark routers.
//!
//! [Add yours!](https://github.com/gemrest/windmark/edit/main/README.md)
//!
//! - [Windmark Comments](https://github.com/gemrest/windmark-comments)
//!
//! ## Capsules using Windmark
//!
//! [Add yours!](https://github.com/gemrest/windmark/edit/main/README.md)
//!
//! - <https://fuwn.me/>
//!
//! ## License
//!
//! This project is licensed with the
//! [GNU General Public License v3.0](https://github.com/gemrest/windmark/blob/main/LICENSE).

#![feature(once_cell, fn_traits)]
#![deny(
  warnings,
  nonstandard_style,
  unused,
  future_incompatible,
  rust_2018_idioms,
  unsafe_code
)]
#![deny(clippy::all, clippy::nursery, clippy::pedantic)]
#![recursion_limit = "128"]

pub mod handler;
pub mod module;
pub mod response;
pub mod returnable;
pub mod router;
pub mod utilities;

#[macro_use]
extern crate log;

pub use module::Module;
pub use response::Response;
pub use router::Router;
pub use tokio::main;
