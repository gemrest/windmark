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

//! `cargo run --example async_stateful_module --features response-macros`

use windmark::{context::HookContext, Router};

#[derive(Default)]
struct Clicker {
  clicks: std::sync::Arc<std::sync::Mutex<usize>>,
}

#[async_trait::async_trait]
impl windmark::AsyncModule for Clicker {
  async fn on_attach(&mut self, _router: &mut Router) {
    println!("module 'clicker' has been attached!");
  }

  async fn on_pre_route(&mut self, context: HookContext) {
    *self.clicks.lock().unwrap() += 1;

    println!(
      "module 'clicker' has been called before the route '{}' with {} clicks!",
      context.url.path(),
      self.clicks.lock().unwrap()
    );
  }

  async fn on_post_route(&mut self, context: HookContext) {
    println!(
      "module 'clicker' clicker has been called after the route '{}' with {} \
       clicks!",
      context.url.path(),
      self.clicks.lock().unwrap()
    );
  }
}

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut router = Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  #[cfg(feature = "logger")]
  {
    router.enable_default_logger(true);
  }
  router.attach_async(Clicker::default());
  router.mount("/", windmark::success!("Hello!"));

  router.run().await
}
