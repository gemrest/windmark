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

//! `cargo run --example async --features response-macros`

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut router = windmark::router::Router::new();
  #[cfg(feature = "tokio")]
  let async_clicks = std::sync::Arc::new(tokio::sync::Mutex::new(0));
  #[cfg(feature = "async-std")]
  let async_clicks = std::sync::Arc::new(async_std::sync::Mutex::new(0));

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  router.mount("/clicks", move |_| {
    let async_clicks = async_clicks.clone();

    async move {
      let mut clicks = async_clicks.lock().await;

      *clicks += 1;

      windmark::response::Response::success(*clicks)
    }
  });
  router.mount(
    "/macro",
    windmark::success_async!(
      async { "This response was sent using an asynchronous macro." }.await
    ),
  );

  router.run().await
}
