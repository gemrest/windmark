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

use windmark::{returnable::CallbackContext, Response, Router};

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
  router.enable_default_logger(true);
  router.set_error_handler(Box::new(move |_| {
    error_count += 1;

    println!("{} errors so far", error_count);

    Response::PermanentFailure("e".into())
  }));
  router.set_fix_path(true);
  router.attach_stateless(|r| {
    r.mount(
      "/module",
      Box::new(|_| Response::Success("This is a module!".into())),
    );
  });
  router.attach(Clicker::default());
  router.set_pre_route_callback(Box::new(|stream, url, _| {
    info!(
      "accepted connection from {} to {}",
      stream.peer_addr().unwrap().ip(),
      url.to_string()
    )
  }));
  router.set_post_route_callback(Box::new(|stream, _url, _| {
    info!(
      "closed connection from {}",
      stream.peer_addr().unwrap().ip()
    )
  }));
  router.add_header(Box::new(|_| "```\nART IS COOL\n```\nhi".to_string()));
  router.add_footer(Box::new(|_| "Copyright 2022".to_string()));
  router.add_footer(Box::new(|context| {
    format!("Another footer, but lower! (from {})", context.url.path())
  }));
  router.mount(
    "/",
    Box::new(|_| {
      Response::Success(
        "# INDEX\n\nWelcome!\n\n=> /test Test Page\n=> /time Unix Epoch"
          .to_string(),
      )
    }),
  );
  router.mount(
    "/ip",
    Box::new(|context| {
      Response::Success(
        { format!("Hello, {}", context.tcp.peer_addr().unwrap().ip()) }.into(),
      )
    }),
  );
  router.mount(
    "/test",
    Box::new(|_| Response::Success("hi there\n=> / back".to_string())),
  );
  router.mount(
    "/temporary-failure",
    Box::new(|_| Response::TemporaryFailure("Woops, temporarily...".into())),
  );
  router.mount(
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
  );
  router.mount(
    "/query",
    Box::new(|context| {
      Response::Success(format!(
        "queries: {:?}",
        windmark::utilities::queries_from_url(&context.url)
      ))
    }),
  );
  router.mount(
    "/param/:lang",
    Box::new(|context| {
      Response::Success(format!(
        "Parameter lang is {}",
        context.params.get("lang").unwrap()
      ))
    }),
  );
  router.mount(
    "/names/:first/:last",
    Box::new(|context| {
      Response::Success(format!(
        "{} {}",
        context.params.get("first").unwrap(),
        context.params.get("last").unwrap()
      ))
    }),
  );
  router.mount(
    "/input",
    Box::new(|context| {
      if let Some(name) = context.url.query() {
        Response::Success(format!("Your name is {}!", name))
      } else {
        Response::Input("What is your name?".into())
      }
    }),
  );
  router.mount(
    "/sensitive-input",
    Box::new(|context| {
      if let Some(password) = context.url.query() {
        Response::Success(format!("Your password is {}!", password))
      } else {
        Response::SensitiveInput("What is your password?".into())
      }
    }),
  );
  router.mount(
    "/error",
    Box::new(|_| Response::CertificateNotValid("no".into())),
  );
  router.mount(
    "/redirect",
    Box::new(|_| Response::PermanentRedirect("gemini://localhost/test".into())),
  );
  router.mount("/file", {
    #[cfg(feature = "auto-deduce-mime")]
    {
      Box::new(|_| Response::SuccessFile(include_bytes!("../LICENSE")))
    }

    #[cfg(not(feature = "auto-deduce-mime"))]
    Box::new(|_| {
      Response::SuccessFile(
        include_bytes!("../LICENSE"),
        "text/plain".to_string(),
      )
    })
  });
  router.mount(
    "/secret",
    Box::new(|context| {
      if let Some(certificate) = context.certificate {
        Response::Success(format!("Your public key: {}.", {
          (|| -> Result<String, openssl::error::ErrorStack> {
            Ok(format!(
              "{:?}",
              certificate.public_key()?.rsa()?.public_key_to_pem()?
            ))
          })()
          .unwrap_or_else(|_| "Unknown".to_string())
        },))
      } else {
        Response::ClientCertificateRequired(
          "This is a secret route! Identify yourself!".to_string(),
        )
      }
    }),
  );

  router.run().await
}
