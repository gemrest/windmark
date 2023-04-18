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

//! `cargo run --example error_handler`

use windmark::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut error_count = 0;

  windmark::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .set_error_handler(move |_| {
      error_count += 1;

      println!("{} errors so far", error_count);

      Response::permanent_failure("e")
    })
    .mount("/error", |_| {
      let nothing = None::<String>;

      Response::success(nothing.unwrap())
    })
    .run()
    .await
}
