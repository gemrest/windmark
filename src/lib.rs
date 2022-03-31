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

//! # Windmark
//!
//! [![crates.io](https://img.shields.io/crates/v/windmark.svg)](https://crates.io/crates/windmark)
//! [![docs.rs](https://docs.rs/windmark/badge.svg)](https://docs.rs/windmark)
//! [![github.com](https://github.com/gemrest/windmark/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/windmark/actions/workflows/check.yaml)
//!
//! Windmark is an elegant and highly performant, async Gemini server framework.
//!
//! ## Usage
//!
//! ### Add Windmark as a dependency
//!
//! ```toml
//! # Cargo.toml
//!
//! [dependencies]
//! windmark = "0.1.7"
//! tokio = { version = "0.2.4", features = ["full"] }
//!
//! # If you would like to use the built-in logger (recommended)
//! # windmark = { version = "0.1.7", features = ["logger"] }
//! ```
//!
//! ### Implement a Windmark server
//!
//! ```rust
//! // src/main.rs
//!
//! use windmark::Response;
//!
//! #[windmark::main]
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!   windmark::Router::new()
//!     .set_private_key_file("windmark_private.pem")
//!     .set_certificate_file("windmark_public.pem")
//!     .mount("/", Box::new(|_| Response::Success("Hello, World!".into())))
//!     .set_error_handler(Box::new(|_| {
//!       Response::PermanentFailure("This route does not exist!".into())
//!     }))
//!     .run()
//!     .await
//! }
//! ```
//!
//! ## Examples
//!
//! Examples can be found within the
//! [`examples/`](https://github.com/gemrest/windmark/tree/main/examples) directory.
//!
//! ## License
//!
//! This project is licensed with the
//! [GNU General Public License v3.0](https://github.com/gemrest/windmark/blob/main/LICENSE).

#![feature(once_cell, fn_traits)]
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

mod handler;
pub mod response;
pub(crate) mod returnable;
pub mod utilities;

#[macro_use]
extern crate log;

use std::{
  error::Error,
  sync::{Arc, Mutex},
};

use openssl::ssl::{self, SslAcceptor, SslMethod};
pub use response::Response;
pub use tokio::main;
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  stream::StreamExt,
};
use url::Url;

use crate::{
  handler::{Callback, ErrorResponse, Partial, RouteResponse},
  response::to_value_set_status,
  returnable::{ErrorContext, RouteContext},
};

/// A router which takes care of all tasks a Windmark server should handle:
/// response generation, panics, logging, and more.
#[derive(Clone)]
pub struct Router {
  routes:                matchit::Router<Arc<Mutex<RouteResponse>>>,
  error_handler:         Arc<Mutex<ErrorResponse>>,
  private_key_file_name: String,
  ca_file_name:          String,
  header:                Arc<Mutex<Partial>>,
  footer:                Arc<Mutex<Partial>>,
  ssl_acceptor:          Arc<SslAcceptor>,
  #[cfg(feature = "logger")]
  default_logger:        bool,
  pre_route_callback:    Arc<Mutex<Callback>>,
  post_route_callback:   Arc<Mutex<Callback>>,
  charset:               String,
  language:              String,
  port:                  i32,
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
  /// windmark::Router::new().set_certificate_file("windmark_public.pem");
  /// ```
  pub fn set_certificate_file(&mut self, certificate_name: &str) -> &mut Self {
    self.ca_file_name = certificate_name.to_string();

    self
  }

  /// Map routes to URL paths
  ///
  /// # Examples
  ///
  /// ```rust
  /// use windmark::Response;
  ///
  /// windmark::Router::new()
  ///   .mount(
  ///     "/",
  ///     Box::new(|_| Response::Success("This is the index page!".into())),
  ///   )
  ///   .mount(
  ///     "/test",
  ///     Box::new(|_| Response::Success("This is a test page!".into())),
  ///   );
  /// ```
  ///
  /// # Panics
  ///
  /// if the route cannot be mounted.
  pub fn mount(&mut self, route: &str, handler: RouteResponse) -> &mut Self {
    self
      .routes
      .insert(route, Arc::new(Mutex::new(handler)))
      .unwrap();

    self
  }

  /// Create an error handler which will be displayed on any error.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_error_handler(Box::new(|_| {
  ///   windmark::Response::Success("You have encountered an error!".into())
  /// }));
  /// ```
  pub fn set_error_handler(&mut self, handler: ErrorResponse) -> &mut Self {
    self.error_handler = Arc::new(Mutex::new(handler));

    self
  }

  /// Set a header for the `Router` which should be displayed on every route.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_header(Box::new(|context| {
  ///   format!("This is displayed at the top of {}!", context.url.path())
  /// }));
  /// ```
  pub fn set_header(&mut self, handler: Partial) -> &mut Self {
    self.header = Arc::new(Mutex::new(handler));

    self
  }

  /// Set a footer for the `Router` which should be displayed on every route.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_footer(Box::new(|context| {
  ///   format!("This is displayed at the bottom of {}!", context.url.path())
  /// }));
  /// ```
  pub fn set_footer(&mut self, handler: Partial) -> &mut Self {
    self.footer = Arc::new(Mutex::new(handler));

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
  pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
    self.create_acceptor()?;

    #[cfg(feature = "logger")]
    if self.default_logger {
      pretty_env_logger::init();
    }

    let acceptor = self.ssl_acceptor.clone();
    let mut listener =
      tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;

    #[cfg(feature = "logger")]
    info!("windmark is listening for connections");

    while let Some(stream) = listener.incoming().next().await {
      match stream {
        Ok(stream) => {
          let acceptor = acceptor.clone();
          let mut self_clone = self.clone();

          tokio::spawn(async move {
            match tokio_openssl::accept(&acceptor, stream).await {
              Ok(mut stream) => {
                if let Err(e) = self_clone.handle(&mut stream).await {
                  error!("handle error: {}", e);
                }
              }
              Err(e) => error!("ssl error: {:?}", e),
            }
          });
        }
        Err(e) => error!("tcp error: {:?}", e),
      }
    }

    Ok(())
  }

  #[allow(clippy::too_many_lines)]
  async fn handle(
    &mut self,
    stream: &mut tokio_openssl::SslStream<tokio::net::TcpStream>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 1024];
    let mut url = Url::parse("gemini://fuwn.me/")?;
    let mut response_status = 0;
    let mut response_mime_type = "".to_string();
    let mut footer = String::new();
    let mut header = String::new();
    let content;

    while let Ok(size) = stream.read(&mut buffer).await {
      let content = String::from_utf8(buffer[0..size].to_vec())?;

      url = url::Url::parse(&content.replace("\r\n", ""))?;

      if content.contains("\r\n") {
        break;
      }
    }

    let route = self.routes.at(url.path());

    (*self.pre_route_callback).lock().unwrap().call_mut((
      stream.get_ref(),
      &url,
      {
        if let Ok(route) = &route {
          Some(&route.params)
        } else {
          None
        }
      },
    ));

    if let Ok(ref route) = route {
      header = {
        let header = (*self.header).lock().unwrap().call_mut((
          RouteContext::new(stream.get_ref(), &url, &route.params),
        ));

        if header.is_empty() {
          "".to_string()
        } else {
          format!("{}\n", header)
        }
      };
      footer = {
        let footer = (*self.footer).lock().unwrap().call_mut((
          RouteContext::new(stream.get_ref(), &url, &route.params),
        ));

        if footer.is_empty() {
          "".to_string()
        } else {
          format!("\n{}", footer)
        }
      };
      content = {
        to_value_set_status(
          (*route.value).lock().unwrap().call_mut((RouteContext::new(
            stream.get_ref(),
            &url,
            &route.params,
          ),)),
          &mut response_status,
          #[cfg(not(feature = "auto-deduce-mime"))]
          &mut response_mime_type,
        )
      };
    } else {
      content = to_value_set_status(
        (*self.error_handler)
          .lock()
          .unwrap()
          .call_mut((ErrorContext::new(stream.get_ref(), &url),)),
        &mut response_status,
        #[cfg(not(feature = "auto-deduce-mime"))]
        &mut response_mime_type,
      );
    }

    stream
      .write_all(
        format!(
          "{}{}\r\n{}",
          if response_status == 21 {
            20
          } else {
            response_status
          },
          match response_status {
            20 =>
              format!(
                " text/gemini; charset={}; lang={}",
                self.charset, self.language
              ),
            #[cfg(feature = "auto-deduce-mime")]
            21 => tree_magic::from_u8(&*content.as_bytes()),
            #[cfg(not(feature = "auto-deduce-mime"))]
            21 => response_mime_type,
            _ => (&*content).to_string(),
          },
          match response_status {
            20 => format!("{}{}{}", header, content, footer),
            21 => (&*content).to_string(),
            _ => "".to_string(),
          }
        )
        .as_bytes(),
      )
      .await?;

    (*self.post_route_callback).lock().unwrap().call_mut((
      stream.get_ref(),
      &url,
      {
        if let Ok(route) = &route {
          Some(&route.params)
        } else {
          None
        }
      },
    ));

    stream.shutdown().await?;

    Ok(())
  }

  fn create_acceptor(&mut self) -> Result<(), Box<dyn Error>> {
    let mut builder = SslAcceptor::mozilla_intermediate(ssl::SslMethod::tls())?;

    builder.set_private_key_file(
      &self.private_key_file_name,
      ssl::SslFiletype::PEM,
    )?;
    builder.set_certificate_file(
      &self.ca_file_name,
      openssl::ssl::SslFiletype::PEM,
    )?;
    builder.check_private_key()?;

    self.ssl_acceptor = Arc::new(builder.build());

    Ok(())
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
  ///     .set_certificate_file(
  ///       "windmark_public.pem",
  ///       openssl::ssl::SslFiletype::PEM,
  ///     )
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
    std::env::set_var("RUST_LOG", "windmark=trace");

    self
  }

  /// Set the default logger's log level.
  ///
  /// If you enable Windmark's default logger with `enable_default_logger`,
  /// Windmark will only log, logs from itself. By setting a log level with
  /// this method, you are overriding the default log level, so you must choose
  /// to enable logs from Windmark with the `log_windmark` parameter.
  ///
  /// Log level "language" is detailed
  /// [here](https://docs.rs/env_logger/0.9.0/env_logger/#enabling-logging).
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new()
  ///   .enable_default_logger(true)
  ///   .set_log_level("your_crate_name=trace", true);
  /// // If you would only like to log, logs from your crate:
  /// // .set_log_level("your_crate_name=trace", false);
  /// ```
  #[cfg(feature = "logger")]
  pub fn set_log_level(
    &mut self,
    log_level: &str,
    log_windmark: bool,
  ) -> &mut Self {
    std::env::set_var(
      "RUST_LOG",
      format!(
        "{}{}",
        if log_windmark { "windmark," } else { "" },
        log_level
      ),
    );

    self
  }

  /// Set a callback to run before a client response is delivered
  ///
  /// # Examples
  ///
  /// ```rust
  /// use log::info;
  ///
  /// windmark::Router::new().set_pre_route_callback(Box::new(
  ///   |stream, _url, _| {
  ///     info!(
  ///       "accepted connection from {}",
  ///       stream.peer_addr().unwrap().ip(),
  ///     )
  ///   },
  /// ));
  /// ```
  pub fn set_pre_route_callback(&mut self, callback: Callback) -> &mut Self {
    self.pre_route_callback = Arc::new(Mutex::new(callback));

    self
  }

  /// Set a callback to run after a client response is delivered
  ///
  /// # Examples
  ///
  /// ```rust
  /// use log::info;
  ///
  /// windmark::Router::new().set_post_route_callback(Box::new(
  ///   |stream, _url, _| {
  ///     info!(
  ///       "closed connection from {}",
  ///       stream.peer_addr().unwrap().ip(),
  ///     )
  ///   },
  /// ));
  /// ```
  pub fn set_post_route_callback(&mut self, callback: Callback) -> &mut Self {
    self.post_route_callback = Arc::new(Mutex::new(callback));

    self
  }

  /// Attach a module to a `Router`.
  ///
  /// A module is an extension or middleware to a `Router`. Modules get full
  /// access to the `Router`, but can be extended by a third party.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use windmark::Response;
  ///
  /// windmark::Router::new().attach(|r| {
  ///   r.mount(
  ///     "/module",
  ///     Box::new(|_| Response::Success("This is a module!".into())),
  ///   );
  ///   r.set_error_handler(Box::new(|_| {
  ///     Response::NotFound(
  ///       "This error handler has been implemented by a module!".into(),
  ///     )
  ///   }));
  /// });
  /// ```
  pub fn attach<F>(&mut self, mut module: F) -> &mut Self
  where F: FnMut(&mut Self) {
    module(self);

    self
  }

  /// Specify a custom character set.
  ///
  /// Defaults to `"utf-8"`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_charset("utf-8"); 
  /// ```
  pub fn set_charset(&mut self, charset: &str) -> &mut Self {
    self.charset = charset.to_string();

    self
  }

  /// Specify a custom language.
  ///
  /// Defaults to `"en"`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_language("en"); 
  /// ```
  pub fn set_language(&mut self, language: &str) -> &mut Self {
    self.language = language.to_string();

    self
  }

  /// Specify a custom port.
  ///
  /// Defaults to `1965`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_port(1965); 
  /// ```
  pub fn set_port(&mut self, port: i32) -> &mut Self {
    self.port = port;

    self
  }
}
impl Default for Router {
  fn default() -> Self {
    Self {
      routes: matchit::Router::new(),
      error_handler: Arc::new(Mutex::new(Box::new(|_| {
        Response::NotFound(
          "This capsule has not implemented an error handler...".to_string(),
        )
      }))),
      private_key_file_name: "".to_string(),
      ca_file_name: "".to_string(),
      header: Arc::new(Mutex::new(Box::new(|_| "".to_string()))),
      footer: Arc::new(Mutex::new(Box::new(|_| "".to_string()))),
      ssl_acceptor: Arc::new(
        SslAcceptor::mozilla_intermediate(SslMethod::tls())
          .unwrap()
          .build(),
      ),
      #[cfg(feature = "logger")]
      default_logger: false,
      pre_route_callback: Arc::new(Mutex::new(Box::new(|_, _, _| {}))),
      post_route_callback: Arc::new(Mutex::new(Box::new(|_, _, _| {}))),
      charset: "utf-8".to_string(),
      language: "en".to_string(),
      port: 1965,
    }
  }
}
