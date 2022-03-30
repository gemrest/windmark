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

use windmark::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut error_count = 0;

  windmark::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .enable_default_logger(true)
    .set_error_handler(Box::new(move |_| {
      error_count += 1;

      println!("{} errors so far", error_count);

      Response::PermanentFailure("e".into())
    }))
    .attach(|r| {
      r.mount(
        "/module",
        Box::new(|_| Response::Success("This is a module!".into())),
      );
    })
    .set_pre_route_callback(Box::new(|stream, url, _| {
      info!(
        "accepted connection from {} to {}",
        stream.peer_addr().unwrap().ip(),
        url.to_string()
      )
    }))
    .set_post_route_callback(Box::new(|stream, _url, _| {
      info!(
        "closed connection from {}",
        stream.peer_addr().unwrap().ip()
      )
    }))
    .set_header(Box::new(|_| "```\nART IS COOL\n```".to_string()))
    .set_footer(Box::new(|_| "Copyright 2022".to_string()))
    .mount(
      "/",
      Box::new(|_| {
        Response::Success(
          "# INDEX\n\nWelcome!\n\n=> /test Test Page\n=> /time Unix Epoch\n"
            .to_string(),
        )
      }),
    )
    .mount(
      "/ip",
      Box::new(|context| {
        Response::Success(
          { format!("Hello, {}", context.tcp.peer_addr().unwrap().ip()) }
            .into(),
        )
      }),
    )
    .mount(
      "/test",
      Box::new(|_| Response::Success("hi there\n=> / back".to_string())),
    )
    .mount(
      "/temporary-failure",
      Box::new(|_| Response::TemporaryFailure("Woops, temporarily...".into())),
    )
    .mount(
      "/time",
      Box::new(|_| {
        Response::Success(
          std::time::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos()
            .to_string(),
        )
      }),
    )
    .mount(
      "/query",
      Box::new(|context| {
        Response::Success(format!(
          "queries: {:?}",
          windmark::utilities::queries_from_url(&context.url)
        ))
      }),
    )
    .mount(
      "/param/:lang",
      Box::new(|context| {
        Response::Success(format!(
          "Parameter lang is {}",
          context.params.get("lang").unwrap()
        ))
      }),
    )
    .mount(
      "/names/:first/:last",
      Box::new(|context| {
        Response::Success(format!(
          "{} {}",
          context.params.get("first").unwrap(),
          context.params.get("last").unwrap()
        ))
      }),
    )
    .mount(
      "/input",
      Box::new(|context| {
        if let Some(name) = context.url.query() {
          Response::Success(format!("Your name is {}!", name))
        } else {
          Response::Input("What is your name?".into())
        }
      }),
    )
    .mount(
      "/sensitive-input",
      Box::new(|context| {
        if let Some(password) = context.url.query() {
          Response::Success(format!("Your password is {}!", password))
        } else {
          Response::SensitiveInput("What is your password?".into())
        }
      }),
    )
    .mount(
      "/error",
      Box::new(|_| Response::CertificateNotValid("no".into())),
    )
    .mount(
      "/redirect",
      Box::new(|_| {
        Response::PermanentRedirect("gemini://localhost/test".into())
      }),
    )
    .mount(
      "/file",
      Box::new(|_| Response::SuccessFile(include_bytes!("../LICENSE"))),
    )
    .run()
    .await
}
