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

#![deny(
  warnings,
  nonstandard_style,
  unused,
  future_incompatible,
  rust_2018_idioms,
  unsafe_code
)]
#![deny(clippy::all, clippy::nursery, clippy::pedantic)]
#![recursion_limit = "128"]

pub mod response;
pub mod status;

#[cfg(feature = "logger")]
#[macro_use]
extern crate log;

use std::{collections::HashMap, net::TcpStream, sync::Arc};

use openssl::ssl::{self, SslAcceptor, SslMethod};
use url::Url;

#[derive(Clone)]
pub struct Router {
  routes: HashMap<String, fn(&TcpStream) -> String>,
  error_handler: fn(&TcpStream) -> String,
  private_key_file_name: String,
  certificate_chain_file_name: String,
  header: fn(&TcpStream) -> String,
  footer: fn(&TcpStream) -> String,
  ssl_acceptor: Arc<SslAcceptor>,
  default_logger: bool,
  pre_route_callback: fn(&TcpStream),
  post_route_callback: fn(&TcpStream),
}
impl Router {
  /// Create a new `Router`
  ///
  /// # Examples
  ///
  /// ```rust
  /// let _router = windmark::Router::new(); 
  /// ```
  ///
  /// # Panics
  ///
  /// if a default `SslAcceptor` could not be built.
  #[must_use]
  pub fn new() -> Self { Self::default() }

  /// Set the filename of the private key file.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_private_key_file("windmark_private.pem");
  /// ```
  pub fn set_private_key_file(
    &mut self,
    private_key_file_name: &str,
  ) -> &mut Self {
    self.private_key_file_name = private_key_file_name.to_string();

    self
  }

  /// Set the filename of the certificate chain file.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_certificate_chain_file("windmark_pair.pem");
  /// ```
  pub fn set_certificate_chain_file(
    &mut self,
    certificate_chain_file_name: &str,
  ) -> &mut Self {
    self.certificate_chain_file_name = certificate_chain_file_name.to_string();

    self
  }

  /// Map routes to URL paths
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new()
  ///   .mount("/", |_| "This is the index page!".into())
  ///   .mount("/test", |_| "This is a test page!".into());
  /// ```
  pub fn mount(
    &mut self,
    route: &str,
    handler: fn(&TcpStream) -> String,
  ) -> &mut Self {
    self.routes.insert(route.to_string(), handler);

    self
  }

  /// Create an error handler which will be displayed on any error.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new()
  ///   .set_error_handler(|_| "You have encountered an error!".into());
  /// ```
  pub fn set_error_handler(
    &mut self,
    handler: fn(&TcpStream) -> String,
  ) -> &mut Self {
    self.error_handler = handler;

    self
  }

  /// Set a header for the `Router` which should be displayed on every route.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_header(|_| {
  ///   "This will be displayed on every route! (at the top)".into()
  /// });
  /// ```
  pub fn set_header(&mut self, handler: fn(&TcpStream) -> String) -> &mut Self {
    self.header = handler;

    self
  }

  /// Set a footer for the `Router` which should be displayed on every route.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_footer(|_| {
  ///   "This will be displayed on every route! (at the bottom)".into()
  /// });
  /// ```
  pub fn set_footer(&mut self, handler: fn(&TcpStream) -> String) -> &mut Self {
    self.footer = handler;

    self
  }

  /// Run the `Router` and wait for requests
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().run(); 
  /// ```
  ///
  /// # Panics
  ///
  /// if the client could not be accepted.
  ///
  /// # Errors
  ///
  /// if the `TcpListener` could not be bound.
  pub fn run(&mut self) -> std::io::Result<()> {
    self.create_acceptor();

    #[cfg(feature = "logger")]
    if self.default_logger {
      pretty_env_logger::init();
    }

    let acceptor = self.ssl_acceptor.clone();
    let listener = std::net::TcpListener::bind("0.0.0.0:1965")?;

    #[cfg(feature = "logger")]
    info!("windmark is listening for connections");

    for stream in listener.incoming() {
      match stream {
        Ok(stream) => {
          let acceptor = acceptor.clone();
          let self_clone = self.clone();

          std::thread::spawn(move || {
            let mut stream = acceptor.accept(stream).unwrap();

            self_clone.handle(&mut stream);
          });
        }
        Err(e) => eprintln!("tcp error: {:?}", e),
      }
    }

    Ok(())
  }

  fn handle(&self, stream: &mut ssl::SslStream<std::net::TcpStream>) {
    let mut buffer = [0u8; 1024];
    let mut url = Url::parse("gemini://fuwn.me/").unwrap();

    while let Ok(size) = stream.ssl_read(&mut buffer) {
      let content = String::from_utf8(buffer[0..size].to_vec()).unwrap();

      url = url::Url::parse(&content.replace("\r\n", "")).unwrap();

      if content.contains("\r\n") {
        break;
      }
    }

    (self.pre_route_callback)(stream.get_ref());

    stream
      .ssl_write(
        format!(
          "20 text/gemini; charset=utf-8\r\n{}{}{}",
          {
            let header = (self.header)(stream.get_ref());

            if header.is_empty() {
              "".to_string()
            } else {
              format!("{}\n", header)
            }
          },
          self.routes.get(url.path()).unwrap_or(&self.error_handler)(
            stream.get_ref()
          ),
          {
            let footer = (self.footer)(stream.get_ref());

            if footer.is_empty() {
              "".to_string()
            } else {
              format!("\n{}", footer)
            }
          },
        )
        .as_bytes(),
      )
      .unwrap();

    (self.post_route_callback)(stream.get_ref());

    stream.shutdown().unwrap();
  }

  fn create_acceptor(&mut self) {
    let mut builder =
      SslAcceptor::mozilla_intermediate(ssl::SslMethod::tls()).unwrap();

    builder
      .set_private_key_file(&self.private_key_file_name, ssl::SslFiletype::PEM)
      .unwrap();
    builder
      .set_certificate_chain_file(&self.certificate_chain_file_name)
      .unwrap();
    builder.check_private_key().unwrap();

    self.ssl_acceptor = Arc::new(builder.build());
  }

  /// Use a self-made `SslAcceptor`
  ///
  /// # Examples
  ///
  /// ```rust
  /// use openssl::ssl;
  ///
  /// windmark::Router::new().set_ssl_acceptor({
  ///   let mut builder =
  ///     ssl::SslAcceptor::mozilla_intermediate(ssl::SslMethod::tls()).unwrap();
  ///
  ///   builder
  ///     .set_private_key_file("windmark_private.pem", ssl::SslFiletype::PEM)
  ///     .unwrap();
  ///   builder
  ///     .set_certificate_chain_file("windmark_pair.pem")
  ///     .unwrap();
  ///   builder.check_private_key().unwrap();
  ///
  ///   builder.build()
  /// });
  /// ```
  pub fn set_ssl_acceptor(&mut self, ssl_acceptor: SslAcceptor) -> &mut Self {
    self.ssl_acceptor = Arc::new(ssl_acceptor);

    self
  }

  /// Enabled the default logger (the
  /// [`pretty_env_logger`](https://crates.io/crates/pretty_env_logger) and
  /// [`log`](https://crates.io/crates/log) crates).
  pub fn enable_default_logger(&mut self, enable: bool) -> &mut Self {
    self.default_logger = enable;
    std::env::set_var("RUST_LOG", "trace");

    self
  }

  /// Set a callback to run before a client response is delivered
  ///
  /// # Examples
  ///
  /// ```rust
  /// use log::info;
  ///
  /// windmark::Router::new().set_pre_route_callback(|stream| {
  ///   info!(
  ///     "accepted connection from {}",
  ///     stream.peer_addr().unwrap().ip(),
  ///   )
  /// });
  /// ```
  pub fn set_pre_route_callback(
    &mut self,
    callback: fn(&TcpStream),
  ) -> &mut Self {
    self.pre_route_callback = callback;

    self
  }

  /// Set a callback to run after a client response is delivered
  ///
  /// # Examples
  ///
  /// ```rust
  /// use log::info;
  ///
  /// windmark::Router::new().set_post_route_callback(|stream| {
  ///   info!(
  ///     "closed connection from {}",
  ///     stream.peer_addr().unwrap().ip(),
  ///   )
  /// });
  /// ```
  pub fn set_post_route_callback(
    &mut self,
    callback: fn(&TcpStream),
  ) -> &mut Self {
    self.post_route_callback = callback;

    self
  }
}
impl Default for Router {
  fn default() -> Self {
    Self {
      routes: HashMap::default(),
      error_handler: |_| {
        "This capsule has not implemented an error handler...".to_string()
      },
      private_key_file_name: "".to_string(),
      certificate_chain_file_name: "".to_string(),
      header: |_| "".to_string(),
      footer: |_| "".to_string(),
      ssl_acceptor: Arc::new(
        SslAcceptor::mozilla_intermediate(SslMethod::tls())
          .unwrap()
          .build(),
      ),
      default_logger: false,
      pre_route_callback: |_| {},
      post_route_callback: |_| {},
    }
  }
}
