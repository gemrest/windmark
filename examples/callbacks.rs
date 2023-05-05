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

//! `cargo run --example callbacks`

use windmark::context::HookContext;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/", windmark::success!("Hello!"))
    .set_pre_route_callback(|context: HookContext| {
      println!(
        "accepted connection from {} to {}",
        context.peer_address.unwrap().ip(),
        context.url.to_string()
      )
    })
    .set_post_route_callback(
      |context: HookContext, content: &mut windmark::response::Response| {
        content.content = content.content.replace("Hello", "Hi");

        println!(
          "closed connection from {}",
          context.peer_address.unwrap().ip()
        )
      },
    )
    .run()
    .await
}
