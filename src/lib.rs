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

mod handler;
pub mod response;
pub(crate) mod returnable;
pub mod utilities;

#[macro_use]
extern crate log;

use std::{error::Error, sync::Arc};

use openssl::ssl::{self, SslAcceptor, SslMethod};
pub use response::Response;
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

#[derive(Clone)]
pub struct Router {
  routes: matchit::Router<RouteResponse>,
  error_handler: ErrorResponse,
  private_key_file_name: String,
  certificate_chain_file_name: String,
  header: Partial,
  footer: Partial,
  ssl_acceptor: Arc<SslAcceptor>,
  #[cfg(feature = "logger")]
  default_logger: bool,
  pre_route_callback: Callback,
  post_route_callback: Callback,
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
  /// use windmark::Response;
  ///
  /// windmark::Router::new()
  ///   .mount("/", |_| Response::Success("This is the index page!".into()))
  ///   .mount("/test", |_| {
  ///     Response::Success("This is a test page!".into())
  ///   });
  /// ```
  ///
  /// # Panics
  ///
  /// if the route cannot be mounted.
  pub fn mount(&mut self, route: &str, handler: RouteResponse) -> &mut Self {
    self.routes.insert(route, handler).unwrap();

    self
  }

  /// Create an error handler which will be displayed on any error.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_error_handler(|_| {
  ///   windmark::Response::Success("You have encountered an error!".into())
  /// });
  /// ```
  pub fn set_error_handler(&mut self, handler: ErrorResponse) -> &mut Self {
    self.error_handler = handler;

    self
  }

  /// Set a header for the `Router` which should be displayed on every route.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_header(|context| {
  ///   format!("This is displayed at the top of {}!", context.url.path())
  /// });
  /// ```
  pub fn set_header(&mut self, handler: Partial) -> &mut Self {
    self.header = handler;

    self
  }

  /// Set a footer for the `Router` which should be displayed on every route.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::Router::new().set_footer(|context| {
  ///   format!("This is displayed at the bottom of {}!", context.url.path())
  /// });
  /// ```
  pub fn set_footer(&mut self, handler: Partial) -> &mut Self {
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
  pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
    self.create_acceptor()?;

    #[cfg(feature = "logger")]
    if self.default_logger {
      pretty_env_logger::init();
    }

    let acceptor = self.ssl_acceptor.clone();
    let mut listener = tokio::net::TcpListener::bind("0.0.0.0:1965").await?;

    #[cfg(feature = "logger")]
    info!("windmark is listening for connections");

    while let Some(stream) = listener.incoming().next().await {
      match stream {
        Ok(stream) => {
          let acceptor = acceptor.clone();
          let self_clone = self.clone();

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

  async fn handle(
    &self,
    stream: &mut tokio_openssl::SslStream<tokio::net::TcpStream>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 1024];
    let mut url = Url::parse("gemini://fuwn.me/")?;
    let mut response_status = 0;
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

    (self.pre_route_callback)(stream.get_ref(), &url, {
      if let Ok(route) = &route {
        Some(&route.params)
      } else {
        None
      }
    });

    if let Ok(ref route) = route {
      header = {
        let header = (self.header)(RouteContext::new(
          stream.get_ref(),
          &url,
          &route.params,
        ));

        if header.is_empty() {
          "".to_string()
        } else {
          format!("{}\n", header)
        }
      };
      footer = {
        let footer = (self.footer)(RouteContext::new(
          stream.get_ref(),
          &url,
          &route.params,
        ));

        if footer.is_empty() {
          "".to_string()
        } else {
          format!("\n{}", footer)
        }
      };
      content = {
        to_value_set_status(
          (route.value)(RouteContext::new(
            stream.get_ref(),
            &url,
            &route.params,
          )),
          &mut response_status,
        )
      };
    } else {
      content = to_value_set_status(
        (self.error_handler)(ErrorContext::new(stream.get_ref(), &url)),
        &mut response_status,
      );
    }

    stream
      .write_all(
        format!(
          "{}{}\r\n{}",
          response_status,
          match response_status {
            20 => " text/gemini; charset=utf-8",
            _ => &*content,
          },
          match response_status {
            20 => format!("{}{}{}", header, content, footer),
            _ => "".to_string(),
          }
        )
        .as_bytes(),
      )
      .await?;

    (self.post_route_callback)(stream.get_ref(), &url, {
      if let Ok(route) = &route {
        Some(&route.params)
      } else {
        None
      }
    });

    stream.shutdown().await?;

    Ok(())
  }

  fn create_acceptor(&mut self) -> Result<(), Box<dyn Error>> {
    let mut builder = SslAcceptor::mozilla_intermediate(ssl::SslMethod::tls())?;

    builder.set_private_key_file(
      &self.private_key_file_name,
      ssl::SslFiletype::PEM,
    )?;
    builder.set_certificate_chain_file(&self.certificate_chain_file_name)?;
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
  /// windmark::Router::new().set_pre_route_callback(|stream, _url, _| {
  ///   info!(
  ///     "accepted connection from {}",
  ///     stream.peer_addr().unwrap().ip(),
  ///   )
  /// });
  /// ```
  pub fn set_pre_route_callback(&mut self, callback: Callback) -> &mut Self {
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
  /// windmark::Router::new().set_post_route_callback(|stream, _url, _| {
  ///   info!(
  ///     "closed connection from {}",
  ///     stream.peer_addr().unwrap().ip(),
  ///   )
  /// });
  /// ```
  pub fn set_post_route_callback(&mut self, callback: Callback) -> &mut Self {
    self.post_route_callback = callback;

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
  ///   r.mount("/module", |_| Response::Success("This is a module!".into()));
  ///   r.set_error_handler(|_| {
  ///     Response::NotFound(
  ///       "This error handler has been implemented by a module!".into(),
  ///     )
  ///   });
  /// });
  /// ```
  pub fn attach<F>(&mut self, mut module: F) -> &mut Self
  where F: FnMut(&mut Self) {
    module(self);

    self
  }
}
impl Default for Router {
  fn default() -> Self {
    Self {
      routes: matchit::Router::new(),
      error_handler: |_| {
        Response::NotFound(
          "This capsule has not implemented an error handler...".to_string(),
        )
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
      #[cfg(feature = "logger")]
      default_logger: false,
      pre_route_callback: |_, _, _| {},
      post_route_callback: |_, _, _| {},
    }
  }
}
