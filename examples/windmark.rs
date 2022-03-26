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

#[macro_use]
extern crate log;

fn main() -> std::io::Result<()> {
  windmark::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_chain_file("windmark_pair.pem")
    .enable_default_logger(true)
    .set_error_handler(|_, _| "error...".to_string())
    .set_pre_route_callback(|stream, url| {
      info!(
        "accepted connection from {} to {}",
        stream.peer_addr().unwrap().ip(),
        url.to_string()
      )
    })
    .set_post_route_callback(|stream, _url| {
      info!(
        "closed connection from {}",
        stream.peer_addr().unwrap().ip()
      )
    })
    .set_header(|_, _| "```\nART IS COOL\n```".to_string())
    .set_footer(|_, _| "Copyright 2022".to_string())
    .mount("/", |_, _| {
      "# INDEX\n\nWelcome!\n\n=> /test Test Page\n=> /time Unix Epoch\n"
        .to_string()
    })
    .mount("/ip", |stream, _| {
      { format!("Hello, {}", stream.peer_addr().unwrap().ip()) }.into()
    })
    .mount("/test", |_, _| "hi there\n=> / back".to_string())
    .mount("/time", |_, _| {
      std::time::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos()
        .to_string()
    })
    .run()
}
