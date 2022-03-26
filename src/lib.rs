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

#![feature(once_cell)]
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
pub mod utilities;

#[cfg(feature = "logger")]
#[macro_use]
extern crate log;

use std::{collections::HashMap, lazy::SyncLazy, net::TcpStream, sync::Arc};

use openssl::ssl::{self, SslAcceptor, SslMethod};
use regex::Regex;
use url::Url;

static DYNAMIC_PARAMETER_REGEX: SyncLazy<Regex> =
  SyncLazy::new(|| Regex::new(r":[a-zA-Z][0-9a-zA-Z_-]*").unwrap());

type RouteResponseHandler = fn(&TcpStream, &Url, Option<String>) -> String;
type CallbackHandler = fn(&TcpStream, &Url);

#[allow(unused)]
#[derive(Clone)]
struct RouteResponse {
  is_dynamic:          bool,
  dynamics_parameters: Vec<String>,
  handler:             RouteResponseHandler,
}
impl RouteResponse {
  pub fn new(
    is_dynamic: bool,
    handler: RouteResponseHandler,
    dynamics_parameters: Vec<String>,
  ) -> Self {
    Self {
      is_dynamic,
      dynamics_parameters,
      handler,
    }
  }
}

#[derive(Clone)]
pub struct Router {
  routes: HashMap<String, RouteResponse>,
  error_handler: RouteResponseHandler,
  private_key_file_name: String,
  certificate_chain_file_name: String,
  header: RouteResponseHandler,
  footer: RouteResponseHandler,
  ssl_acceptor: Arc<SslAcceptor>,
  #[cfg(feature = "logger")]
  default_logger: bool,
  pre_route_callback: CallbackHandler,
  post_route_callback: CallbackHandler,
  drop_trailing_slash: bool,
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
  ///   .mount("/", |_, _, _| "This is the index page!".into())
  ///   .mount("/test", |_, _, _| "This is a test page!".into());
  /// ```
  pub fn mount(
    &mut self,
    route: &str,
    handler: RouteResponseHandler,
  ) -> &mut Self {
    let mut fixed_route = route.to_string();
    let mut is_dynamic = false;
    let dynamic_parameters = DYNAMIC_PARAMETER_REGEX
      .find_iter(route)
      .map(|m| m.as_str().to_string())
      .collect::<Vec<String>>();

    if let Some(dynamic_parameter) = dynamic_parameters.get(0) {
      fixed_route = route.replace(dynamic_parameter, "");
      is_dynamic = true;
    }

    if !fixed_route.is_empty() {
      self.routes.insert(
        fixed_route,
        RouteResponse::new(is_dynamic, handler, dynamic_parameters),
      );
    }

    self
  }

  /// Create an error handler which will be displayed on any error.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new()
  ///   .set_error_handler(|_, _, _| "You have encountered an error!".into());
  /// ```
  pub fn set_error_handler(
    &mut self,
    handler: RouteResponseHandler,
  ) -> &mut Self {
    self.error_handler = handler;

    self
  }

  /// Set a header for the `Router` which should be displayed on every route.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_header(|_, _, _| {
  ///   "This will be displayed on every route! (at the top)".into()
  /// });
  /// ```
  pub fn set_header(&mut self, handler: RouteResponseHandler) -> &mut Self {
    self.header = handler;

    self
  }

  /// Set a footer for the `Router` which should be displayed on every route.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_footer(|_, _, _| {
  ///   "This will be displayed on every route! (at the bottom)".into()
  /// });
  /// ```
  pub fn set_footer(&mut self, handler: RouteResponseHandler) -> &mut Self {
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
    let fixed_url_path;

    while let Ok(size) = stream.ssl_read(&mut buffer) {
      let content = String::from_utf8(buffer[0..size].to_vec()).unwrap();

      url = url::Url::parse(&content.replace("\r\n", "")).unwrap();

      if content.contains("\r\n") {
        break;
      }
    }

    (self.pre_route_callback)(stream.get_ref(), &url);

    stream
      .ssl_write(
        format!(
          "20 text/gemini; charset=utf-8\r\n{}{}{}",
          {
            let header = (self.header)(stream.get_ref(), &url, None);

            if header.is_empty() {
              "".to_string()
            } else {
              format!("{}\n", header)
            }
          },
          {
            if self.drop_trailing_slash
              && url.path().ends_with('/')
              && url.path() != "/"
            {
              fixed_url_path = url.path().trim_end_matches('/');
            } else {
              fixed_url_path = url.path();
            }

            #[allow(clippy::option_if_let_else)]
            if let Some(route) = self.routes.get(fixed_url_path) {
              println!("non dynamic");

              (route.handler)(stream.get_ref(), &url, None)
            } else {
              let matched_dynamics = self
                .routes
                .iter()
                .filter(|(path, _)| url.path().contains(&(*path).clone()))
                .map(|(path, _)| path.clone())
                .filter(|path| path.matches('/').count() == 2)
                .collect::<Vec<String>>();

              if matched_dynamics.is_empty() {
                (self.error_handler)(stream.get_ref(), &url, None)
              } else {
                (self
                  .routes
                  .get(matched_dynamics[0].as_str())
                  .unwrap()
                  .handler)(stream.get_ref(), &url, {
                  let raw_dynamic =
                    url.path().replace(&matched_dynamics[0], "");

                  if raw_dynamic.is_empty() {
                    None
                  } else {
                    Some(raw_dynamic)
                  }
                })
              }
            }
          },
          {
            let footer = (self.footer)(stream.get_ref(), &url, None);

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

    (self.post_route_callback)(stream.get_ref(), &url);

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
  #[cfg(feature = "logger")]
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
  /// windmark::Router::new().set_pre_route_callback(|stream, _url| {
  ///   info!(
  ///     "accepted connection from {}",
  ///     stream.peer_addr().unwrap().ip(),
  ///   )
  /// });
  /// ```
  pub fn set_pre_route_callback(
    &mut self,
    callback: CallbackHandler,
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
  /// windmark::Router::new().set_post_route_callback(|stream, _url| {
  ///   info!(
  ///     "closed connection from {}",
  ///     stream.peer_addr().unwrap().ip(),
  ///   )
  /// });
  /// ```
  pub fn set_post_route_callback(
    &mut self,
    callback: CallbackHandler,
  ) -> &mut Self {
    self.post_route_callback = callback;

    self
  }

  /// Drop the trailing slash on requests.
  ///
  /// Defaults to `true`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// // A request to `gemini://fuwn.me/test/` will be interpreted as
  /// // `gemini://fuwn.me/test`. This is the recommended behaviour.
  ///
  /// windmark::Router::new().set_drop_trailing_slash(true);
  /// ```
  pub fn set_drop_trailing_slash(&mut self, drop: bool) -> &mut Self {
    self.drop_trailing_slash = drop;

    self
  }
}
impl Default for Router {
  fn default() -> Self {
    Self {
      routes: HashMap::default(),
      error_handler: |_, _, _| {
        "This capsule has not implemented an error handler...".to_string()
      },
      private_key_file_name: "".to_string(),
      certificate_chain_file_name: "".to_string(),
      header: |_, _, _| "".to_string(),
      footer: |_, _, _| "".to_string(),
      ssl_acceptor: Arc::new(
        SslAcceptor::mozilla_intermediate(SslMethod::tls())
          .unwrap()
          .build(),
      ),
      #[cfg(feature = "logger")]
      default_logger: false,
      pre_route_callback: |_, _| {},
      post_route_callback: |_, _| {},
      drop_trailing_slash: true,
    }
  }
}
