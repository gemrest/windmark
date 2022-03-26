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

//! `cargo run --example windmark --features logger`

#[macro_use]
extern crate log;

use windmark::response::Response;

fn main() -> std::io::Result<()> {
  windmark::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_chain_file("windmark_pair.pem")
    .enable_default_logger(true)
    .set_error_handler(|_, _, _| {
      Response::PermanentFailure("error...".to_string())
    })
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
    .set_header(|_, _, _| "```\nART IS COOL\n```".to_string())
    .set_footer(|_, _, _| "Copyright 2022".to_string())
    .mount("/", |_, _, _| {
      Response::Success(
        "# INDEX\n\nWelcome!\n\n=> /test Test Page\n=> /time Unix Epoch\n"
          .to_string(),
      )
    })
    .mount("/ip", |stream, _, _| {
      Response::Success(
        { format!("Hello, {}", stream.peer_addr().unwrap().ip()) }.into(),
      )
    })
    .mount("/test", |_, _, _| {
      Response::Success("hi there\n=> / back".to_string())
    })
    .mount("/time", |_, _, _| {
      Response::Success(
        std::time::UNIX_EPOCH
          .elapsed()
          .unwrap()
          .as_nanos()
          .to_string(),
      )
    })
    .mount("/query", |_, url, _| {
      Response::Success(format!(
        "queries: {:?}",
        windmark::utilities::queries_from_url(&url)
      ))
    })
    .mount("/param/:lang", |_, _url, dynamic_parameter| {
      Response::Success(format!("Parameter lang is {:?}", dynamic_parameter))
    })
    .run()
}
