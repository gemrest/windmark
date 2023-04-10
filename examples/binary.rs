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

//! `cargo run --example binary`
//!
//! Optionally, you can run this example with the `auto-deduce-mime` feature
//! enabled.

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut router = windmark::Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  #[cfg(feature = "auto-deduce-mime")]
  router.mount("/automatic", {
    windmark::binary_success!(include_bytes!("../LICENSE"))
  });
  router.mount("/specific", {
    windmark::binary_success!(include_bytes!("../LICENSE"), "text/plain")
  });
  router.mount("/direct", {
    windmark::binary_success!("This is a string.", "text/plain")
  });

  router.run().await
}
