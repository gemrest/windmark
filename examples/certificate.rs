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

//! `cargo run --example certificate --features response-macros`

use windmark::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/secret", |context: windmark::context::RouteContext| {
      if let Some(certificate) = context.certificate {
        Response::success(format!("Your public key is '{}'.", {
          (|| -> Result<String, openssl::error::ErrorStack> {
            Ok(format!(
              "{:?}",
              certificate.public_key()?.rsa()?.public_key_to_pem()?
            ))
          })()
          .unwrap_or_else(|_| {
            "An error occurred while reading your public key.".to_string()
          })
        }))
      } else {
        Response::client_certificate_required(
          "This is a secret route ... Identify yourself!",
        )
      }
    })
    .mount(
      "/invalid",
      windmark::certificate_not_valid!("Your certificate is invalid."),
    )
    .run()
    .await
}
