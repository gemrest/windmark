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

//! `cargo run --example responses`

use windmark::success;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount(
      "/",
      success!(
        "# Index\n\nWelcome!\n\n=> /test Test Page\n=> /time Unix Epoch"
      ),
    )
    .mount("/test", success!("This is a test page.\n=> / back"))
    .mount(
      "/failure",
      windmark::temporary_failure!("Woops ... temporarily."),
    )
    .mount(
      "/time",
      success!(std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()),
    )
    .mount(
      "/redirect",
      windmark::permanent_redirect!("gemini://localhost/test"),
    )
    .run()
    .await
}
