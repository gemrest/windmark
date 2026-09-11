#![allow(clippy::significant_drop_tightening)]

use crate::{
  context::{ErrorContext, HookContext, RouteContext},
  handler::{
    ErrorResponse,
    Partial,
    PostRouteHook,
    PreRouteHook,
    RouteResponse,
  },
  module::{AsyncModule, Module},
  response::Response,
  router_option::RouterOption,
};
#[cfg(feature = "async-std")]
use async_std::{
  io::{ReadExt, WriteExt},
  sync::Mutex as AsyncMutex,
};
use openssl::ssl::{self, SslAcceptor, SslMethod};
use std::{
  collections::HashSet,
  error::Error,
  fmt::Write,
  future::IntoFuture,
  sync::{Arc, Mutex},
  time,
};
#[cfg(feature = "tokio")]
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  sync::Mutex as AsyncMutex,
};
use url::Url;

macro_rules! block {
  ($body:expr) => {
    #[cfg(feature = "tokio")]
    ::tokio::task::block_in_place(|| {
      ::tokio::runtime::Handle::current().block_on(async { $body });
    });
    #[cfg(feature = "async-std")]
    ::async_std::task::block_on(async { $body });
  };
}

macro_rules! or_error {
  ($stream:ident, $operation:expr, $error_format:literal) => {
    match $operation {
      Ok(u) => u,
      Err(e) => {
        $stream
          .write_all(format!($error_format, e).as_bytes())
          .await?;

        return Ok(());
      }
    }
  };
}

#[cfg(feature = "tokio")]
type TcpStream = tokio::net::TcpStream;
#[cfg(feature = "async-std")]
type TcpStream = async_std::net::TcpStream;

#[cfg(feature = "tokio")]
type Stream = tokio_openssl::SslStream<TcpStream>;
#[cfg(feature = "async-std")]
type Stream = async_std_openssl::SslStream<TcpStream>;

/// A router which takes care of all tasks a Windmark server should handle:
/// response generation, panics, logging, and more.
#[derive(Clone)]
pub struct Router {
  routes:                matchit::Router<Arc<dyn RouteResponse>>,
  error_handler:         Arc<dyn ErrorResponse>,
  private_key_file_name: String,
  private_key_content:   Option<String>,
  certificate_file_name: String,
  certificate_content:   Option<String>,
  headers:               Arc<Mutex<Vec<Box<dyn Partial>>>>,
  footers:               Arc<Mutex<Vec<Box<dyn Partial>>>>,
  ssl_acceptor:          Arc<SslAcceptor>,
  manual_acceptor_set:   bool,
  #[cfg(feature = "logger")]
  default_logger:        bool,
  #[cfg(feature = "logger")]
  log_filter:            String,
  pre_route_callback:    Arc<dyn PreRouteHook>,
  post_route_callback:   Arc<dyn PostRouteHook>,
  character_set:         String,
  languages:             Vec<String>,
  port:                  u16,
  async_modules:         Arc<AsyncMutex<Vec<Box<dyn AsyncModule + Send>>>>,
  modules:               Arc<Mutex<Vec<Box<dyn Module + Send>>>>,
  options:               HashSet<RouterOption>,
  listener_address:      String,
}

struct RequestHandler {
  routes:              matchit::Router<Arc<dyn RouteResponse>>,
  error_handler:       Arc<dyn ErrorResponse>,
  headers:             Arc<[Box<dyn Partial>]>,
  footers:             Arc<[Box<dyn Partial>]>,
  pre_route_callback:  Arc<dyn PreRouteHook>,
  post_route_callback: Arc<dyn PostRouteHook>,
  character_set:       String,
  languages_joined:    String,
  async_modules:       Arc<AsyncMutex<Vec<Box<dyn AsyncModule + Send>>>>,
  modules:             Arc<Mutex<Vec<Box<dyn Module + Send>>>>,
  options:             HashSet<RouterOption>,
}

impl RequestHandler {
  #[allow(
    clippy::too_many_lines,
    clippy::significant_drop_in_scrutinee,
    clippy::cognitive_complexity
  )]
  async fn handle(&self, stream: &mut Stream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0u8; 1024];
    let mut footer = String::new();
    let mut header = String::new();
    let mut request = String::new();
    let mut url = loop {
      let size = match stream.read(&mut buffer).await {
        Ok(0) | Err(_) => return Ok(()),
        Ok(size) => size,
      };

      request.push_str(or_error!(
        stream,
        std::str::from_utf8(&buffer[0..size]),
        "59 The server (Windmark) received a bad request: {}"
      ));

      if request.len() > 1024 {
        stream
          .write_all(
            b"59 The server (Windmark) received a request exceeding 1024 \
              bytes\r\n",
          )
          .await?;

        return Ok(());
      }

      if let Some(position) = request.find("\r\n") {
        break or_error!(
          stream,
          Url::parse(&request[..position]),
          "59 The server (Windmark) received a bad request: {}"
        );
      }
    };

    if url.path().is_empty() {
      url.set_path("/");
    }

    let route_path =
      resolve_lookup_path(&self.options, url.path(), |candidate| {
        self.routes.at(candidate).is_ok()
      });
    let route = self.routes.at(&route_path);
    let peer_certificate = stream.ssl().peer_certificate();
    let hook_context = HookContext::new(
      stream.get_ref().peer_addr(),
      url.clone(),
      route.as_ref().ok().map(|route| route.params.clone()),
      peer_certificate.clone(),
    );

    for module in &mut *self.async_modules.lock().await {
      module.on_pre_route(&hook_context).await;
    }

    if let Ok(mut modules) = self.modules.lock() {
      for module in &mut *modules {
        module.on_pre_route(&hook_context);
      }
    }

    self.pre_route_callback.call(&hook_context);

    let mut content = if let Ok(ref route) = route {
      let route_context = RouteContext::new(
        stream.get_ref().peer_addr(),
        url,
        &route.params,
        peer_certificate,
      );

      for partial_header in self.headers.iter() {
        writeln!(&mut header, "{}", partial_header.call(&route_context))
          .expect("failed to write header");
      }

      footer = render_footer(&self.footers, &route_context);

      route.value.call(route_context).await
    } else {
      self
        .error_handler
        .call(ErrorContext::new(
          stream.get_ref().peer_addr(),
          url,
          peer_certificate,
        ))
        .await
    };

    for module in &mut *self.async_modules.lock().await {
      module.on_post_route(&hook_context).await;
    }

    if let Ok(mut modules) = self.modules.lock() {
      for module in &mut *modules {
        module.on_post_route(&hook_context);
      }
    }

    self.post_route_callback.call(&hook_context, &mut content);

    let status_line =
      status_line(&content, &self.character_set, &self.languages_joined);
    let response = serialize_response(content, &status_line, &header, &footer);

    stream.write_all(&response).await?;
    #[cfg(feature = "tokio")]
    stream.shutdown().await?;
    #[cfg(feature = "async-std")]
    stream.get_mut().shutdown(std::net::Shutdown::Both)?;
    Ok(())
  }
}

fn serialize_response(
  content: Response,
  status_line: &str,
  header: &str,
  footer: &str,
) -> Vec<u8> {
  let mut response = Vec::with_capacity(
    status_line.len() + content.body_length(header, footer) + 2,
  );

  response.extend_from_slice(status_line.as_bytes());
  response.extend_from_slice(b"\r\n");
  content.append_body(&mut response, header, footer);
  drop(content);

  response
}

fn render_footer(
  partials: &[Box<dyn Partial>],
  context: &RouteContext,
) -> String {
  let mut footer = String::new();

  for (index, partial) in partials.iter().enumerate() {
    if index != 0 {
      footer.push('\n');
    }

    footer.push_str(&partial.call(context));
  }

  footer
}

/// Spawn a task to complete the TLS handshake on an accepted connection and
/// dispatch it through `handler`.
fn spawn_connection(
  handler: Arc<RequestHandler>,
  acceptor: Arc<SslAcceptor>,
  stream: TcpStream,
) {
  #[cfg(feature = "tokio")]
  let spawner = tokio::spawn;
  #[cfg(feature = "async-std")]
  let spawner = async_std::task::spawn;

  spawner(async move {
    let ssl = match ssl::Ssl::new(acceptor.context()) {
      Ok(ssl) => ssl,
      Err(e) => {
        error!("ssl context error: {e:?}");

        return;
      }
    };
    #[cfg(feature = "tokio")]
    let quick_stream = tokio_openssl::SslStream::new(ssl, stream);
    #[cfg(feature = "async-std")]
    let quick_stream = async_std_openssl::SslStream::new(ssl, stream);

    match quick_stream {
      Ok(mut stream) => {
        if let Err(e) = std::pin::Pin::new(&mut stream).accept().await {
          warn!("stream accept error: {e:?}");

          return;
        }

        if let Err(e) = handler.handle(&mut stream).await {
          error!("handle error: {e}");
        }
      }
      Err(e) => error!("ssl stream error: {e:?}"),
    }
  });
}

/// Build the Gemini response status line for `content`, falling back to the
/// router's `default_character_set` and `default_languages` for `20` responses.
///
/// Statuses `21`/`22` (binary success) are sent to clients as a plain `20` with
/// the response's MIME as the meta; every other status uses the first line of
/// its `content` as the meta.
fn status_line(
  content: &Response,
  default_character_set: &str,
  default_languages: &str,
) -> String {
  match content.status {
    20 => {
      let mime = content.mime.as_deref().unwrap_or("text/gemini");
      let character_set = content
        .character_set
        .as_deref()
        .unwrap_or(default_character_set);
      let languages = content.languages.as_ref().map_or_else(
        || std::borrow::Cow::Borrowed(default_languages),
        |languages| std::borrow::Cow::Owned(languages.join(",")),
      );

      format!("20 {mime}; charset={character_set}; lang={languages}")
    }
    21 | 22 => format!("20 {}", content.mime.as_deref().unwrap_or_default()),
    status =>
      format!(
        "{status} {}",
        content.content.lines().next().unwrap_or_default()
      ),
  }
}

/// Resolve which path an incoming request should be matched against, applying
/// the configured path-fixing `options`.
///
/// `route_exists` probes whether a candidate path resolves to a mounted route,
/// letting the trailing-slash fixes fall back gracefully when their target
/// route is absent. An exact match always takes precedence over any fix.
fn resolve_lookup_path(
  options: &HashSet<RouterOption>,
  request_path: &str,
  route_exists: impl Fn(&str) -> bool,
) -> String {
  let path = if options.contains(&RouterOption::AllowCaseInsensitiveLookup) {
    request_path.to_lowercase()
  } else {
    request_path.to_owned()
  };

  if route_exists(&path) {
    return path;
  }

  if options.contains(&RouterOption::RemoveExtraTrailingSlash)
    && path.ends_with('/')
    && path != "/"
  {
    let trimmed = path.trim_end_matches('/');

    if route_exists(trimmed) {
      return trimmed.to_string();
    }
  } else if options.contains(&RouterOption::AddMissingTrailingSlash)
    && !path.ends_with('/')
  {
    let path_with_slash = format!("{path}/");

    if route_exists(&path_with_slash) {
      return path_with_slash;
    }
  }

  path
}

impl Router {
  /// Create a new `Router`
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new(); 
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
  /// windmark::router::Router::new().set_private_key_file("windmark_private.pem");
  /// ```
  pub fn set_private_key_file(
    &mut self,
    private_key_file_name: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.private_key_file_name = private_key_file_name.into();

    self
  }

  /// Set the content of the private key
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().set_private_key("..."); 
  /// ```
  pub fn set_private_key(
    &mut self,
    private_key_content: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.private_key_content = Some(private_key_content.into());

    self
  }

  /// Set the filename of the certificate chain file.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().set_certificate_file("windmark_public.pem");
  /// ```
  pub fn set_certificate_file(
    &mut self,
    certificate_name: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.certificate_file_name = certificate_name.into();

    self
  }

  /// Set the content of the certificate chain file.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().set_certificate("..."); 
  /// ```
  pub fn set_certificate(
    &mut self,
    certificate_content: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.certificate_content = Some(certificate_content.into());

    self
  }

  /// Map routes to URL paths
  ///
  /// Supports both synchronous and asynchronous handlers
  ///
  /// # Examples
  ///
  /// ```rust
  /// use windmark::response::Response;
  ///
  /// windmark::router::Router::new()
  ///   .mount("/", |_| {
  ///     async { Response::success("This is the index page!") }
  ///   })
  ///   .mount("/about", |_| async { Response::success("About that...") });
  /// ```
  ///
  /// # Panics
  ///
  /// May panic if the route cannot be mounted.
  pub fn mount<R>(
    &mut self,
    route: impl Into<String> + AsRef<str>,
    handler: impl Fn(RouteContext) -> R + Send + Sync + 'static,
  ) -> &mut Self
  where
    R: IntoFuture<Output = Response> + Send + 'static,
    <R as IntoFuture>::IntoFuture: Send,
  {
    self
      .routes
      .insert(
        route.into(),
        Arc::new(move |context: RouteContext| handler(context).into_future()),
      )
      .expect("failed to mount route");

    self
  }

  /// Create an error handler which will be displayed on any error.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().set_error_handler(|_| {
  ///   windmark::response::Response::success("You have encountered an error!")
  /// });
  /// ```
  pub fn set_error_handler<R>(
    &mut self,
    handler: impl Fn(ErrorContext) -> R + Send + Sync + 'static,
  ) -> &mut Self
  where
    R: IntoFuture<Output = Response> + Send + 'static,
    <R as IntoFuture>::IntoFuture: Send,
  {
    self.error_handler =
      Arc::new(move |context| handler(context).into_future());

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
  /// windmark::router::Router::new().add_header(
  ///   |context: &windmark::context::RouteContext| {
  ///     format!("This is displayed at the top of {}!", context.url.path())
  ///   },
  /// );
  /// ```
  pub fn add_header(&mut self, handler: impl Partial + 'static) -> &mut Self {
    (*self.headers.lock().expect("headers lock poisoned"))
      .push(Box::new(handler));

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
  /// windmark::router::Router::new().add_footer(
  ///   |context: &windmark::context::RouteContext| {
  ///     format!("This is displayed at the bottom of {}!", context.url.path())
  ///   },
  /// );
  /// ```
  pub fn add_footer(&mut self, handler: impl Partial + 'static) -> &mut Self {
    (*self.footers.lock().expect("footers lock poisoned"))
      .push(Box::new(handler));

    self
  }

  /// Run the `Router` and wait for requests
  ///
  /// Under the default Tokio runtime, the server runs until it receives an
  /// interrupt (Ctrl+C / SIGINT), at which point it stops accepting new
  /// connections and returns. The `async-std` runtime runs until the process
  /// is terminated.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().run(); 
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
    if !self.manual_acceptor_set {
      self.create_acceptor()?;
    }

    #[cfg(feature = "logger")]
    if self.default_logger {
      pretty_env_logger::formatted_builder()
        .parse_filters(&self.log_filter)
        .init();
    }

    #[cfg(feature = "tokio")]
    let listener = tokio::net::TcpListener::bind(format!(
      "{}:{}",
      self.listener_address, self.port
    ))
    .await?;
    #[cfg(feature = "async-std")]
    let listener = async_std::net::TcpListener::bind(format!(
      "{}:{}",
      self.listener_address, self.port
    ))
    .await?;

    #[cfg(feature = "logger")]
    info!("windmark is listening for connections");

    let handler = Arc::new(RequestHandler {
      routes:              self.routes.clone(),
      error_handler:       self.error_handler.clone(),
      headers:             self
        .headers
        .lock()
        .expect("headers lock poisoned")
        .drain(..)
        .collect::<Vec<_>>()
        .into(),
      footers:             self
        .footers
        .lock()
        .expect("footers lock poisoned")
        .drain(..)
        .collect::<Vec<_>>()
        .into(),
      pre_route_callback:  self.pre_route_callback.clone(),
      post_route_callback: self.post_route_callback.clone(),
      character_set:       self.character_set.clone(),
      languages_joined:    self.languages.join(","),
      async_modules:       self.async_modules.clone(),
      modules:             self.modules.clone(),
      options:             self.options.clone(),
    });
    let acceptor = self.ssl_acceptor.clone();

    // Under Tokio, accept connections until interrupted (Ctrl+C / SIGINT) so
    // the server can shut down gracefully; `async-std` has no built-in signal
    // handling, so it keeps the original unconditional accept loop.
    #[cfg(feature = "tokio")]
    {
      loop {
        tokio::select! {
          connection = listener.accept() => match connection {
            Ok((stream, _)) =>
              spawn_connection(handler.clone(), acceptor.clone(), stream),
            Err(e) => error!("tcp stream error: {e:?}"),
          },
          _ = tokio::signal::ctrl_c() => {
            #[cfg(feature = "logger")]
            info!("windmark received an interrupt; shutting down");

            break;
          }
        }
      }

      Ok(())
    }

    #[cfg(feature = "async-std")]
    loop {
      match listener.accept().await {
        Ok((stream, _)) =>
          spawn_connection(handler.clone(), acceptor.clone(), stream),
        Err(e) => error!("tcp stream error: {e:?}"),
      }
    }
  }

  fn create_acceptor(&mut self) -> Result<(), Box<dyn Error>> {
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;

    if let Some(ref cert_content) = self.certificate_content {
      builder.set_certificate(
        openssl::x509::X509::from_pem(cert_content.as_bytes())?.as_ref(),
      )?;
    } else {
      builder.set_certificate_file(
        &self.certificate_file_name,
        ssl::SslFiletype::PEM,
      )?;
    }

    if let Some(ref key_content) = self.private_key_content {
      builder.set_private_key(
        openssl::pkey::PKey::private_key_from_pem(key_content.as_bytes())?
          .as_ref(),
      )?;
    } else {
      builder.set_private_key_file(
        &self.private_key_file_name,
        ssl::SslFiletype::PEM,
      )?;
    }

    builder.check_private_key()?;
    builder.set_verify_callback(ssl::SslVerifyMode::PEER, |_, _| true);
    builder.set_session_id_context(
      time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)?
        .as_secs()
        .to_string()
        .as_bytes(),
    )?;

    self.ssl_acceptor = Arc::new(builder.build());

    Ok(())
  }

  /// Use a self-made `SslAcceptor`
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// use openssl::ssl;
  ///
  /// windmark::router::Router::new().set_ssl_acceptor({
  ///   let mut builder =
  ///     ssl::SslAcceptor::mozilla_intermediate(ssl::SslMethod::tls()).unwrap();
  ///
  ///   builder
  ///     .set_private_key_file("windmark_private.pem", ssl::SslFiletype::PEM)
  ///     .unwrap();
  ///   builder
  ///     .set_certificate_file("windmark_public.pem", ssl::SslFiletype::PEM)
  ///     .unwrap();
  ///   builder.check_private_key().unwrap();
  ///
  ///   builder.build()
  /// });
  /// ```
  pub fn set_ssl_acceptor(&mut self, ssl_acceptor: SslAcceptor) -> &mut Self {
    self.ssl_acceptor = Arc::new(ssl_acceptor);
    self.manual_acceptor_set = true;

    self
  }

  /// Enabled the default logger (the
  /// [`pretty_env_logger`](https://crates.io/crates/pretty_env_logger) and
  /// [`log`](https://crates.io/crates/log) crates).
  #[cfg(feature = "logger")]
  pub fn enable_default_logger(&mut self, enable: bool) -> &mut Self {
    self.default_logger = enable;

    if self.log_filter.is_empty() {
      self.log_filter = "windmark=trace".to_string();
    }

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
  /// windmark::router::Router::new()
  ///   .enable_default_logger(true)
  ///   .set_log_level("your_crate_name=trace", true);
  /// // If you would only like to log, logs from your crate:
  /// // .set_log_level("your_crate_name=trace", false);
  /// ```
  #[cfg(feature = "logger")]
  pub fn set_log_level(
    &mut self,
    log_level: impl Into<String> + AsRef<str>,
    log_windmark: bool,
  ) -> &mut Self {
    self.log_filter = format!(
      "{}{}",
      if log_windmark { "windmark," } else { "" },
      log_level.into()
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
  /// windmark::router::Router::new().set_pre_route_callback(
  ///   |context: &windmark::context::HookContext| {
  ///     info!(
  ///       "accepted connection from {}",
  ///       context.peer_address.unwrap().ip(),
  ///     )
  ///   },
  /// );
  /// ```
  pub fn set_pre_route_callback(
    &mut self,
    callback: impl PreRouteHook + 'static,
  ) -> &mut Self {
    self.pre_route_callback = Arc::new(callback);

    self
  }

  /// Set a callback to run after a client response is delivered
  ///
  /// # Examples
  ///
  /// ```rust
  /// use log::info;
  ///
  /// windmark::router::Router::new().set_post_route_callback(
  ///   |context: &windmark::context::HookContext,
  ///    _content: &mut windmark::response::Response| {
  ///     info!(
  ///       "closed connection from {}",
  ///       context.peer_address.unwrap().ip(),
  ///     )
  ///   },
  /// );
  /// ```
  pub fn set_post_route_callback(
    &mut self,
    callback: impl PostRouteHook + 'static,
  ) -> &mut Self {
    self.post_route_callback = Arc::new(callback);

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
  /// use windmark::response::Response;
  ///
  /// windmark::router::Router::new().attach_stateless(|r| {
  ///   r.mount(
  ///     "/module",
  ///     Box::new(|_| Response::success("This is a module!")),
  ///   );
  ///   r.set_error_handler(Box::new(|_| {
  ///     Response::not_found(
  ///       "This error handler has been implemented by a module!",
  ///     )
  ///   }));
  /// });
  /// ```
  ///
  /// ## External Module
  ///
  /// ```rust
  /// use windmark::response::Response;
  ///
  /// mod windmark_example {
  ///   pub fn module(router: &mut windmark::router::Router) {
  ///     router.mount(
  ///       "/module",
  ///       Box::new(|_| {
  ///         windmark::response::Response::success("This is a module!")
  ///       }),
  ///     );
  ///   }
  /// }
  ///
  /// windmark::router::Router::new().attach_stateless(windmark_example::module);
  /// ```
  pub fn attach_stateless<F>(&mut self, mut module: F) -> &mut Self
  where F: FnMut(&mut Self) {
    module(self);

    self
  }

  /// Attach a stateful module to a `Router`; with async support
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
  /// use windmark::{context::HookContext, router::Router};
  ///
  /// #[derive(Default)]
  /// struct Clicker {
  ///   clicks: isize,
  /// }
  ///
  /// #[async_trait::async_trait]
  /// impl windmark::module::AsyncModule for Clicker {
  ///   async fn on_attach(&mut self, _: &mut Router) {
  ///     info!("clicker has been attached!");
  ///   }
  ///
  ///   async fn on_pre_route(&mut self, context: &HookContext) {
  ///     self.clicks += 1;
  ///
  ///     info!(
  ///       "clicker has been called pre-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       self.clicks
  ///     );
  ///   }
  ///
  ///   async fn on_post_route(&mut self, context: &HookContext) {
  ///     info!(
  ///       "clicker has been called post-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       self.clicks
  ///     );
  ///   }
  /// }
  ///
  /// // Router::new().attach_async(Clicker::default());
  /// ```
  pub fn attach_async(
    &mut self,
    mut module: impl AsyncModule + 'static,
  ) -> &mut Self {
    block!({
      module.on_attach(self).await;

      (*self.async_modules.lock().await).push(Box::new(module));
    });

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
  /// use windmark::{context::HookContext, response::Response, router::Router};
  ///
  /// #[derive(Default)]
  /// struct Clicker {
  ///   clicks: isize,
  /// }
  ///
  /// impl windmark::module::Module for Clicker {
  ///   fn on_attach(&mut self, _: &mut Router) {
  ///     info!("clicker has been attached!");
  ///   }
  ///
  ///   fn on_pre_route(&mut self, context: &HookContext) {
  ///     self.clicks += 1;
  ///
  ///     info!(
  ///       "clicker has been called pre-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       self.clicks
  ///     );
  ///   }
  ///
  ///   fn on_post_route(&mut self, context: &HookContext) {
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
    (*self.modules.lock().expect("modules lock poisoned"))
      .push(Box::new(module));

    self
  }

  /// Specify a custom character set.
  ///
  /// Will be over-ridden if a character set is specified in a [`Response`].
  ///
  /// Defaults to `"utf-8"`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().set_character_set("utf-8"); 
  /// ```
  pub fn set_character_set(
    &mut self,
    character_set: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.character_set = character_set.into();

    self
  }

  /// Specify a custom language.
  ///
  /// Will be over-ridden if a language is specified in a [`Response`].
  ///
  /// Defaults to `"en"`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().set_languages(["en"]); 
  /// ```
  pub fn set_languages<S>(&mut self, language: impl AsRef<[S]>) -> &mut Self
  where S: Into<String> + AsRef<str> {
    self.languages = language
      .as_ref()
      .iter()
      .map(|s| s.as_ref().to_string())
      .collect::<Vec<String>>();

    self
  }

  /// Specify a custom port.
  ///
  /// Defaults to `1965`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().set_port(1965); 
  /// ```
  pub const fn set_port(&mut self, port: u16) -> &mut Self {
    self.port = port;

    self
  }

  /// Add optional features to the router
  ///
  /// # Examples
  ///
  /// ```rust
  /// use windmark::router_option::RouterOption;
  ///
  /// windmark::router::Router::new()
  ///   .add_options(&[RouterOption::RemoveExtraTrailingSlash]);
  /// ```
  pub fn add_options(&mut self, options: &[RouterOption]) -> &mut Self {
    for option in options {
      self.options.insert(*option);
    }

    self
  }

  /// Toggle optional features for the router
  ///
  /// # Examples
  ///
  /// ```rust
  /// use windmark::router_option::RouterOption;
  ///
  /// windmark::router::Router::new()
  ///   .toggle_options(&[RouterOption::RemoveExtraTrailingSlash]);
  /// ```
  pub fn toggle_options(&mut self, options: &[RouterOption]) -> &mut Self {
    for option in options {
      if self.options.contains(option) {
        self.options.remove(option);
      } else {
        self.options.insert(*option);
      }
    }

    self
  }

  /// Remove optional features from the router
  ///
  /// # Examples
  ///
  /// ```rust
  /// use windmark::router_option::RouterOption;
  ///
  /// windmark::router::Router::new()
  ///   .remove_options(&[RouterOption::RemoveExtraTrailingSlash]);
  /// ```
  pub fn remove_options(&mut self, options: &[RouterOption]) -> &mut Self {
    for option in options {
      self.options.remove(option);
    }

    self
  }

  /// Specify a custom listener address.
  ///
  /// Defaults to `"0.0.0.0"`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().set_listener_address("[::]"); 
  /// ```
  pub fn set_listener_address(
    &mut self,
    address: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.listener_address = address.into();

    self
  }
}

impl Default for Router {
  fn default() -> Self {
    Self {
      routes: matchit::Router::new(),
      error_handler: Arc::new(|_| {
        async {
          Response::not_found(
            "This capsule has not implemented an error handler...",
          )
        }
      }),
      private_key_file_name: String::new(),
      certificate_file_name: String::new(),
      headers: Arc::new(Mutex::new(vec![])),
      footers: Arc::new(Mutex::new(vec![])),
      ssl_acceptor: Arc::new(
        SslAcceptor::mozilla_intermediate(SslMethod::tls())
          .expect("failed to create default SSL acceptor")
          .build(),
      ),
      manual_acceptor_set: false,
      #[cfg(feature = "logger")]
      default_logger: false,
      #[cfg(feature = "logger")]
      log_filter: String::new(),
      pre_route_callback: Arc::new((|_| {}) as fn(&HookContext)),
      post_route_callback: Arc::new(
        (|_, _: &mut Response| {}) as fn(&HookContext, &mut Response),
      ),
      character_set: "utf-8".to_string(),
      languages: vec!["en".to_string()],
      port: 1965,
      modules: Arc::new(Mutex::new(vec![])),
      async_modules: Arc::new(AsyncMutex::new(vec![])),
      options: HashSet::new(),
      private_key_content: None,
      certificate_content: None,
      listener_address: "0.0.0.0".to_string(),
    }
  }
}

#[cfg(test)]
#[path = "../tests/router.rs"]
mod tests;
