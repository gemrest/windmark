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
//! Windmark is an elegant and highly performant, async Gemini server framework
//! for the modern age!
//!
//! ## Usage
//!
//! ### Add Windmark as a dependency
//!
//! ```toml
//! # Cargo.toml
//!
//! [dependencies]
//! windmark = "0.1.12"
//! tokio = { version = "0.2.4", features = ["full"] }
//!
//! # If you would like to use the built-in logger (recommended)
//! # windmark = { version = "0.1.12", features = ["logger"] }
//!
//! # If you would like to use the built-in MIME dedection when `Success`-ing a file
//! # (recommended)
//! # windmark = { version = "0.1.12", features = ["auto-deduce-mime"] }
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
//!     .set_certificate_chain_file("windmark_public.pem")
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
//! An example of a fully featured Gemini capsule written using Windmark can be
//! found [here](https://github.com/gemrest/locus). This example Gemini capsule also
//! happens to be the source code for [Fuwn's](https://github.com/Fuwn) (this
//! library's author) personal Gemini capsule!
//!
//! ## Modules
//!
//! Modules are reusable extensions which can be procedurally mounted onto
//! Windmark routers.
//!
//! [Add yours!](https://github.com/gemrest/windmark/edit/main/README.md)
//!
//! - [Windmark Comments](https://github.com/gemrest/windmark-comments)
//!
//! ## Capsules using Windmark
//!
//! [Add yours!](https://github.com/gemrest/windmark/edit/main/README.md)
//!
//! - <https://fuwn.me/>
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
pub mod module;
pub mod response;
pub mod returnable;
pub mod utilities;

#[macro_use]
extern crate log;

use std::{
  error::Error,
  sync::{Arc, Mutex},
};

pub use module::Module;
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
  returnable::{CallbackContext, ErrorContext, RouteContext},
};

/// A router which takes care of all tasks a Windmark server should handle:
/// response generation, panics, logging, and more.
#[derive(Clone)]
pub struct Router {
  routes:                matchit::Router<Arc<Mutex<RouteResponse>>>,
  error_handler:         Arc<Mutex<ErrorResponse>>,
  private_key_file_name: String,
  ca_file_name:          String,
  headers:               Arc<Mutex<Vec<Partial>>>,
  footers:               Arc<Mutex<Vec<Partial>>>,
  ssl_acceptor:          Arc<SslAcceptor>,
  #[cfg(feature = "logger")]
  default_logger:        bool,
  pre_route_callback:    Arc<Mutex<Callback>>,
  post_route_callback:   Arc<Mutex<Callback>>,
  charset:               String,
  language:              String,
  port:                  i32,
  modules:               Arc<Mutex<Vec<Box<dyn Module + Send>>>>,
  fix_path:              bool,
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

  /// Add a header for the `Router` which should be displayed on every route.
  ///
  /// # Panics
  ///
  /// May panic if the header cannot be added.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().add_header(Box::new(|context| {
  ///   format!("This is displayed at the top of {}!", context.url.path())
  /// }));
  /// ```
  pub fn add_header(&mut self, handler: Partial) -> &mut Self {
    (*self.headers.lock().unwrap()).push(handler);

    self
  }

  /// Add a footer for the `Router` which should be displayed on every route.
  ///
  /// # Panics
  ///
  /// May panic if the header cannot be added.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().add_footer(Box::new(|context| {
  ///   format!("This is displayed at the bottom of {}!", context.url.path())
  /// }));
  /// ```
  pub fn add_footer(&mut self, handler: Partial) -> &mut Self {
    (*self.footers.lock().unwrap()).push(handler);

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
    #[cfg(not(feature = "auto-deduce-mime"))]
    let mut response_mime_type = "".to_string();
    let mut footer = String::new();
    let mut header = String::new();

    while let Ok(size) = stream.read(&mut buffer).await {
      let content = String::from_utf8(buffer[0..size].to_vec())?;

      url = url::Url::parse(&content.replace("\r\n", ""))?;

      if content.contains("\r\n") {
        break;
      }
    }

    let fixed_path = if self.fix_path {
      self
        .routes
        .fix_path(if url.path().is_empty() {
          "/"
        } else {
          url.path()
        })
        .unwrap_or_else(|| url.path().to_string())
    } else {
      url.path().to_string()
    };
    let route = &mut self.routes.at(&fixed_path);

    for module in &mut *self.modules.lock().unwrap() {
      module.on_pre_route(CallbackContext::new(stream.get_ref(), &url, {
        if let Ok(route) = &route {
          Some(&route.params)
        } else {
          None
        }
      }));
    }

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

    let content = if let Ok(ref route) = route {
      let footers_length = (*self.footers.lock().unwrap()).len();

      for partial_header in &mut *self.headers.lock().unwrap() {
        header.push_str(&format!(
          "{}\n",
          partial_header(RouteContext::new(
            stream.get_ref(),
            &url,
            &route.params,
          )),
        ));
      }
      for (i, partial_footer) in
        (&mut *self.footers.lock().unwrap()).iter_mut().enumerate()
      {
        footer.push_str(&format!(
          "{}{}",
          partial_footer(RouteContext::new(
            stream.get_ref(),
            &url,
            &route.params,
          )),
          if footers_length > 1 && i != footers_length - 1 {
            "\n"
          } else {
            ""
          },
        ));
      }
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
    } else {
      to_value_set_status(
        (*self.error_handler)
          .lock()
          .unwrap()
          .call_mut((ErrorContext::new(stream.get_ref(), &url),)),
        &mut response_status,
        #[cfg(not(feature = "auto-deduce-mime"))]
        &mut response_mime_type,
      )
    };

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
            20 => format!("{}{}\n{}", header, content, footer),
            21 => (&*content).to_string(),
            _ => "".to_string(),
          }
        )
        .as_bytes(),
      )
      .await?;

    for module in &mut *self.modules.lock().unwrap() {
      module.on_post_route(CallbackContext::new(stream.get_ref(), &url, {
        if let Ok(route) = &route {
          Some(&route.params)
        } else {
          None
        }
      }));
    }

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

  /// Attach a stateless module to a `Router`.
  ///
  /// A module is an extension or middleware to a `Router`. Modules get full
  /// access to the `Router`, but can be extended by a third party.
  ///
  /// # Examples
  ///
  /// ## Integrated Module
  ///
  /// ```rust
  /// use windmark::Response;
  ///
  /// windmark::Router::new().attach_stateless(|r| {
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
  ///
  /// ## External Module
  ///
  /// ```rust
  /// use windmark::Response;
  ///
  /// mod windmark_example {
  ///   pub fn module(router: &mut windmark::Router) {
  ///     router.mount(
  ///       "/module",
  ///       Box::new(|_| windmark::Response::Success("This is a module!".into())),
  ///     );
  ///   }
  /// }
  ///
  /// windmark::Router::new().attach_stateless(windmark_example::module);
  /// ```
  pub fn attach_stateless<F>(&mut self, mut module: F) -> &mut Self
  where F: FnMut(&mut Self) {
    module(self);

    self
  }

  /// Attach a stateful module to a `Router`.
  ///
  /// Like a stateless module is an extension or middleware to a `Router`.
  /// Modules get full access to the `Router` and can be extended by a third
  /// party, but also, can create hooks will be executed through various parts
  /// of a routes' lifecycle. Stateful modules also have state, so variables can
  /// be stored for further access.
  ///
  /// # Panics
  ///
  /// May panic if the stateful module cannot be attached.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use log::info;
  /// use windmark::{returnable::CallbackContext, Response, Router};
  ///
  /// #[derive(Default)]
  /// struct Clicker {
  ///   clicks: isize,
  /// }
  /// impl windmark::Module for Clicker {
  ///   fn on_attach(&mut self, _: &mut Router) {
  ///     info!("clicker has been attached!");
  ///   }
  ///
  ///   fn on_pre_route(&mut self, context: CallbackContext<'_>) {
  ///     self.clicks += 1;
  ///
  ///     info!(
  ///       "clicker has been called pre-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       self.clicks
  ///     );
  ///   }
  ///
  ///   fn on_post_route(&mut self, context: CallbackContext<'_>) {
  ///     info!(
  ///       "clicker has been called post-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       self.clicks
  ///     );
  ///   }
  /// }
  ///
  /// Router::new().attach(Clicker::default());
  /// ```
  pub fn attach(
    &mut self,
    mut module: impl Module + 'static + Send,
  ) -> &mut Self {
    module.on_attach(self);

    (*self.modules.lock().unwrap()).push(Box::new(module));

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

  /// Performs a case-insensitive lookup of routes, using the case corrected
  /// path if successful. Missing/ extra trailing slashes are also corrected.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_fix_path(true); 
  /// ```
  pub fn set_fix_path(&mut self, fix_path: bool) -> &mut Self {
    self.fix_path = fix_path;

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
      headers: Arc::new(Mutex::new(vec![])),
      footers: Arc::new(Mutex::new(vec![])),
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
      modules: Arc::new(Mutex::new(vec![])),
      fix_path: false,
    }
  }
}
