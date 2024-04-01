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

//! `cargo run --example struct_router`

use rossweisse::route;
use windmark::response::Response;

#[rossweisse::router]
struct Router;

#[rossweisse::router]
impl Router {
  #[route(index)]
  pub fn index(_context: windmark::context::RouteContext) -> Response {
    Response::success("Hello, World!")
  }
}

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  {
    let mut router = Router::new();

    router.router().set_private_key_file("windmark_private.pem");
    router.router().set_certificate_file("windmark_public.pem");

    router
  }
  .run()
  .await
}
