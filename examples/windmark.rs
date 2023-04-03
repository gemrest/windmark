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

use windmark::{
  response::Response,
  context::{CallbackContext, RouteContext},
  success,
  Router,
};

#[derive(Default)]
struct Clicker {
  clicks: isize,
}
impl windmark::Module for Clicker {
  fn on_attach(&mut self, _: &mut Router) {
    println!("clicker has been attached!");
  }

  fn on_pre_route(&mut self, context: CallbackContext<'_>) {
    self.clicks += 1;

    info!(
      "clicker has been called pre-route on {} with {} clicks!",
      context.url.path(),
      self.clicks
    );
  }

  fn on_post_route(&mut self, context: CallbackContext<'_>) {
    info!(
      "clicker has been called post-route on {} with {} clicks!",
      context.url.path(),
      self.clicks
    );
  }
}

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut error_count = 0;
  let mut router = Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  #[cfg(feature = "logger")]
  router.enable_default_logger(true);
  router.set_error_handler(move |_| {
    error_count += 1;

    println!("{} errors so far", error_count);

    Response::permanent_failure("e")
  });
  router.set_fix_path(true);
  router.attach_stateless(|r| {
    r.mount("/module", success!("This is a module!"));
  });
  router.attach(Clicker::default());
  router.set_pre_route_callback(|context| {
    info!(
      "accepted connection from {} to {}",
      context.tcp.peer_addr().unwrap().ip(),
      context.url.to_string()
    )
  });
  router.set_post_route_callback(|context, content| {
    content.content =
      content.content.replace("Welcome!", "Welcome to Windmark!");

    info!(
      "closed connection from {}",
      context.tcp.peer_addr().unwrap().ip()
    )
  });
  router.add_header(|_| "```\nART IS COOL\n```\nhi".to_string());
  router.add_footer(|_| "Copyright 2022".to_string());
  router.add_footer(|context| {
    format!("Another footer, but lower! (from {})", context.url.path())
  });
  router.mount(
    "/",
    success!("# INDEX\n\nWelcome!\n\n=> /test Test Page\n=> /time Unix Epoch"),
  );
  router.mount("/specific-mime", |_| {
    Response::success("hi".to_string())
      .with_mime("text/plain")
      .clone()
  });
  router.mount(
    "/ip",
    success!(
      context,
      format!("Hello, {}", context.tcp.peer_addr().unwrap().ip())
    ),
  );
  router.mount("/test", success!("hi there\n=> / back"));
  router.mount(
    "/temporary-failure",
    windmark::temporary_failure!("Woops, temporarily..."),
  );
  router.mount(
    "/time",
    success!(std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()),
  );
  router.mount(
    "/query",
    success!(
      context,
      format!(
        "queries: {:?}",
        windmark::utilities::queries_from_url(&context.url)
      )
    ),
  );
  router.mount(
    "/param/:lang",
    success!(
      context,
      format!("Parameter lang is {}", context.params.get("lang").unwrap())
    ),
  );
  router.mount(
    "/names/:first/:last",
    success!(
      context,
      format!(
        "{} {}",
        context.params.get("first").unwrap(),
        context.params.get("last").unwrap()
      )
    ),
  );
  router.mount("/input", |context: RouteContext| {
    if let Some(name) = context.url.query() {
      Response::success(format!("Your name is {}!", name))
    } else {
      Response::input("What is your name?")
    }
  });
  router.mount("/sensitive-input", |context: RouteContext| {
    if let Some(password) = context.url.query() {
      Response::success(format!("Your password is {}!", password))
    } else {
      Response::sensitive_input("What is your password?")
    }
  });
  router.mount("/error", windmark::certificate_not_valid!("no"));
  router.mount(
    "/redirect",
    windmark::permanent_redirect!("gemini://localhost/test"),
  );
  #[cfg(feature = "auto-deduce-mime")]
  router.mount("/auto-file", {
    windmark::binary_success!(include_bytes!("../LICENSE"))
  });
  router.mount("/file", {
    windmark::binary_success!(include_bytes!("../LICENSE"), "text/plain")
  });
  router.mount("/string-file", {
    windmark::binary_success!("hi", "text/plain")
  });
  router.mount("/secret", |context: RouteContext| {
    if let Some(certificate) = context.certificate {
      Response::success(format!("Your public key: {}.", {
        (|| -> Result<String, openssl::error::ErrorStack> {
          Ok(format!(
            "{:?}",
            certificate.public_key()?.rsa()?.public_key_to_pem()?
          ))
        })()
        .unwrap_or_else(|_| "Unknown".to_string())
      },))
    } else {
      Response::client_certificate_required(
        "This is a secret route! Identify yourself!",
      )
    }
  });

  router.run().await
}
