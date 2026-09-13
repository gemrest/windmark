#![allow(clippy::significant_drop_tightening)]

use crate::{
  context::{ErrorContext, HookContext, RouteContext},
  handler::{ErrorResponse, Partial, PostRouteHook, PreRouteHook},
  module::{AsyncModule, Module},
  response::Response,
  router_option::RouterOption,
};
#[cfg(feature = "async-std")]
use async_std::{
  io::{ReadExt, WriteExt},
  sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock},
};
use openssl::ssl::{self, SslAcceptor, SslMethod};
use std::{
  collections::HashSet,
  error::Error,
  fmt::Write,
  future::IntoFuture,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
    Mutex,
    RwLock,
  },
  time,
};
#[cfg(feature = "tokio")]
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock},
};

mod connections;
mod request;
mod routes;
mod server;

pub use connections::ConnectionLimits;
use connections::{with_timeout, TcpListener};
use routes::Routes;
pub use server::Server;

const MAX_REQUEST_URI_BYTES: usize = 1024;

#[cfg(feature = "tokio")]
type TcpStream = tokio::net::TcpStream;
#[cfg(feature = "async-std")]
type TcpStream = async_std::net::TcpStream;

#[cfg(feature = "tokio")]
type Stream = tokio_openssl::SslStream<TcpStream>;
#[cfg(feature = "async-std")]
type Stream = async_std_openssl::SslStream<TcpStream>;

struct ModuleEntry {
  module:   Box<dyn Module>,
  poisoned: AtomicBool,
}

type SharedModule = Arc<ModuleEntry>;
type SharedAsyncModule = Arc<dyn AsyncModule>;

#[derive(Default)]
struct ModuleScheduling {
  concurrent:   AtomicBool,
  synchronous:  RwLock<()>,
  asynchronous: AsyncRwLock<()>,
}

#[derive(Clone, Copy)]
enum HookPhase {
  PreRoute,
  PostRoute,
}

/// A router dispatches requests through its handlers, partials, and modules.
///
/// Requests must contain an absolute URI of at most 1024 bytes, excluding
/// CRLF, without userinfo or a fragment. Non-ASCII characters must be
/// percent-encoded. Other schemes remain available to proxy handlers.
#[derive(Clone)]
pub struct Router {
  routes:                Routes,
  error_handler:         Arc<dyn ErrorResponse>,
  private_key_file_name: String,
  private_key_content:   Option<String>,
  certificate_file_name: String,
  certificate_content:   Option<String>,
  headers:               Arc<Mutex<Vec<Arc<dyn Partial>>>>,
  footers:               Arc<Mutex<Vec<Arc<dyn Partial>>>>,
  ssl_acceptor:          Option<Arc<SslAcceptor>>,
  pre_route_callback:    Arc<dyn PreRouteHook>,
  post_route_callback:   Arc<dyn PostRouteHook>,
  character_set:         String,
  languages:             Vec<String>,
  port:                  u16,
  async_modules:         Arc<AsyncMutex<Vec<SharedAsyncModule>>>,
  modules:               Arc<Mutex<Vec<SharedModule>>>,
  module_scheduling:     Arc<ModuleScheduling>,
  options:               HashSet<RouterOption>,
  listener_address:      String,
  connection_limits:     ConnectionLimits,
}

struct RequestHandler {
  routes:              Routes,
  error_handler:       Arc<dyn ErrorResponse>,
  headers:             Arc<[Arc<dyn Partial>]>,
  footers:             Arc<[Arc<dyn Partial>]>,
  pre_route_callback:  Arc<dyn PreRouteHook>,
  post_route_callback: Arc<dyn PostRouteHook>,
  character_set:       String,
  languages_joined:    String,
  async_modules:       Arc<AsyncMutex<Vec<SharedAsyncModule>>>,
  modules:             Arc<Mutex<Vec<SharedModule>>>,
  module_scheduling:   Arc<ModuleScheduling>,
  options:             HashSet<RouterOption>,
  connection_limits:   ConnectionLimits,
}

impl RequestHandler {
  #[allow(
    clippy::too_many_lines,
    clippy::significant_drop_in_scrutinee,
    clippy::cognitive_complexity
  )]
  async fn handle(&self, stream: &mut Stream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0u8; MAX_REQUEST_URI_BYTES];
    let mut footer = String::new();
    let mut header = String::new();
    let mut request = Vec::new();
    let request = with_timeout(self.connection_limits.request_timeout, async {
      let url = loop {
        let size = match stream.read(&mut buffer).await {
          Ok(0) => return Ok(None),
          Err(error) => return Err(error),
          Ok(size) => size,
        };

        request.extend_from_slice(&buffer[..size]);

        let request_end = request.windows(2).position(|pair| pair == b"\r\n");
        let uri_length = request_end.unwrap_or_else(|| {
          request.len() - usize::from(request.last() == Some(&b'\r'))
        });

        if uri_length > MAX_REQUEST_URI_BYTES {
          return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request URI exceeds 1024 bytes",
          ));
        }

        if let Some(position) = request_end {
          let request =
            std::str::from_utf8(&request[..position]).map_err(|error| {
              std::io::Error::new(std::io::ErrorKind::InvalidData, error)
            })?;

          break request::parse_uri(request).map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, error)
          })?;
        }
      };

      Ok(Some(url))
    })
    .await?;
    let mut url = match request {
      Ok(Some(url)) => url,
      Ok(None) => return Ok(()),
      Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
        let response = format!(
          "59 The server (Windmark) received a bad request: {error}\r\n"
        );

        with_timeout(
          self.connection_limits.write_timeout,
          stream.write_all(response.as_bytes()),
        )
        .await??;

        return Ok(());
      }
      Err(error) => return Err(error.into()),
    };

    if url.path().is_empty() {
      url.set_path("/");
    }

    let route_path =
      resolve_lookup_path(&self.options, url.path(), |candidate| {
        self.routes.contains(candidate)
      });
    let route = self.routes.at(&route_path);
    let peer_certificate = stream.ssl().peer_certificate();
    let hook_context = HookContext {
      peer_address: stream.get_ref().peer_addr().ok(),
      url:          url.clone(),
      parameters:   route.as_ref().ok().map(|route| route.parameters.clone()),
      certificate:  peer_certificate.clone(),
    };

    call_async_modules(
      &self.async_modules,
      &self.module_scheduling,
      &hook_context,
      HookPhase::PreRoute,
    )
    .await;
    call_modules(&self.modules, &self.module_scheduling, |module| {
      module.on_pre_route(&hook_context);
    });
    self.pre_route_callback.call(&hook_context);

    let mut content = if let Ok(ref route) = route {
      let route_context = RouteContext {
        peer_address: stream.get_ref().peer_addr().ok(),
        url,
        parameters: route.parameters.clone(),
        certificate: peer_certificate,
      };

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

    call_async_modules(
      &self.async_modules,
      &self.module_scheduling,
      &hook_context,
      HookPhase::PostRoute,
    )
    .await;
    call_modules(&self.modules, &self.module_scheduling, |module| {
      module.on_post_route(&hook_context);
    });
    self.post_route_callback.call(&hook_context, &mut content);

    let status_line = match status_line(
      &content,
      &self.character_set,
      &self.languages_joined,
    ) {
      Ok(line) => line,
      Err(error) => {
        error!("response encoding error: {error}");

        content = Response::temporary_failure(
          "The server could not encode the response",
        );

        "40 The server could not encode the response".to_owned()
      }
    };
    let response = serialize_response(content, &status_line, &header, &footer);

    with_timeout(
      self.connection_limits.write_timeout,
      stream.write_all(&response),
    )
    .await??;

    Ok(())
  }
}

fn call_modules(
  modules: &Mutex<Vec<SharedModule>>,
  scheduling: &ModuleScheduling,
  hook: impl Fn(&dyn Module),
) {
  let invoke = || {
    let modules = modules.lock().ok().map(|modules| modules.clone());

    if let Some(modules) = modules {
      for module in modules {
        if !module.poisoned.load(Ordering::Acquire) {
          let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
              hook(module.module.as_ref());
            }));

          if let Err(panic) = result {
            module.poisoned.store(true, Ordering::Release);
            std::panic::resume_unwind(panic);
          }
        }
      }
    }
  };

  if scheduling.concurrent.load(Ordering::Relaxed) {
    let _permit = scheduling
      .synchronous
      .read()
      .unwrap_or_else(std::sync::PoisonError::into_inner);

    invoke();
  } else {
    let _permit = scheduling
      .synchronous
      .write()
      .unwrap_or_else(std::sync::PoisonError::into_inner);

    invoke();
  }
}

async fn call_async_modules(
  modules: &AsyncMutex<Vec<SharedAsyncModule>>,
  scheduling: &ModuleScheduling,
  context: &HookContext,
  phase: HookPhase,
) {
  let invoke = async {
    let modules = modules.lock().await.clone();

    for module in modules {
      match phase {
        HookPhase::PreRoute => module.on_pre_route(context).await,
        HookPhase::PostRoute => module.on_post_route(context).await,
      }
    }
  };

  if scheduling.concurrent.load(Ordering::Relaxed) {
    let _permit = scheduling.asynchronous.read().await;

    invoke.await;
  } else {
    let _permit = scheduling.asynchronous.write().await;

    invoke.await;
  }
}

fn serialize_response(
  content: Response,
  status_line: &str,
  header: &str,
  footer: &str,
) -> Vec<u8> {
  let mut response = Vec::with_capacity(
    status_line.len() + content.body_capacity(header, footer) + 2,
  );

  response.extend_from_slice(status_line.as_bytes());
  response.extend_from_slice(b"\r\n");
  content.append_body(&mut response, header, footer);
  drop(content);

  response
}

fn render_footer(
  partials: &[Arc<dyn Partial>],
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

/// Build a protocol-valid response header from the handler's response.
fn status_line(
  content: &Response,
  default_character_set: &str,
  default_languages: &str,
) -> Result<String, &'static str> {
  let status = content.status();

  if matches!(status, 20..=22) {
    let mime = content.mime().unwrap_or(if status == 20 {
      "text/gemini"
    } else {
      "application/octet-stream"
    });
    let metadata = if status == 20 {
      let character_set =
        content.character_set().unwrap_or(default_character_set);
      let languages = content.languages().map_or_else(
        || default_languages.to_owned(),
        |languages| languages.join(","),
      );
      let mut metadata =
        format!("{mime}; charset={}", mime_parameter(character_set));

      if !languages.is_empty() {
        if languages
          .split(',')
          .any(|language| language_tags::LanguageTag::parse(language).is_err())
        {
          return Err("invalid response language tag");
        }

        write!(&mut metadata, "; lang={}", mime_parameter(&languages))
          .expect("writing to a string cannot fail");
      }

      metadata
    } else {
      mime.to_owned()
    };

    if metadata.chars().any(char::is_control)
      || metadata.parse::<mime::Mime>().is_err()
    {
      return Err("invalid response media type or parameters");
    }

    return Ok(format!("20 {metadata}"));
  }

  if !matches!(status, 10 | 11 | 30 | 31 | 40..=44 | 50..=53 | 59..=62) {
    return Err("undefined response status");
  }

  let metadata = content
    .content()
    .unwrap_or_default()
    .lines()
    .next()
    .unwrap_or_default();

  if metadata.chars().any(char::is_control) {
    return Err("response metadata contains a control character");
  }

  if matches!(status, 10 | 11 | 30 | 31) && metadata.is_empty() {
    return Err("input prompts and redirect targets must not be empty");
  }

  if matches!(status, 30 | 31) && !request::valid_uri_reference(metadata) {
    return Err("invalid redirect URI reference");
  }

  if metadata.is_empty() {
    Ok(status.to_string())
  } else {
    Ok(format!("{status} {metadata}"))
  }
}

fn mime_parameter(value: &str) -> String {
  if !value.is_empty()
    && value.bytes().all(|byte| {
      byte.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?=".contains(&byte)
    })
  {
    value.to_owned()
  } else {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
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
  let path = request_path.to_owned();

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
  /// Create a new `Router`.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new(); 
  /// ```
  #[must_use]
  pub fn new() -> Self { Self::default() }

  /// Set the filename of the private key file, replacing any inline key.
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
    self.private_key_content = None;

    self
  }

  /// Set the content of the private key.
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

  /// Set the filename of the certificate chain file, replacing any inline
  /// chain.
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
    self.certificate_content = None;

    self
  }

  /// Set a PEM certificate chain, with the server certificate first.
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

  /// Map routes to URL paths.
  ///
  /// This method supports both synchronous and asynchronous handlers.
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
  /// This method panics if the route is invalid or conflicts with a mounted
  /// route, including conflicts under case-insensitive matching when enabled.
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

  /// Set the handler for unmatched routes. Transport errors and route panics
  /// do not invoke this handler.
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
      .push(Arc::new(handler));

    self
  }

  /// Add a footer for the `Router` which should be displayed on every route.
  ///
  /// # Panics
  ///
  /// This method panics if the footer registry lock has been poisoned.
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
      .push(Arc::new(handler));

    self
  }

  /// Run the `Router` and wait for requests.
  ///
  /// Under the default Tokio runtime, the server runs until it receives an
  /// interrupt (Ctrl+C / SIGINT), at which point it stops accepting new
  /// connections and drains for the configured period before returning. The
  /// default drain period is zero. The `async-std` runtime runs until the
  /// process is terminated; use `run_until` for application-controlled
  /// shutdown.
  ///
  /// # Examples
  ///
  /// ```rust
  /// windmark::router::Router::new().run(); 
  /// ```
  ///
  /// # Panics
  ///
  /// This method panics if a partial registry lock has been poisoned.
  ///
  /// # Errors
  ///
  /// This method returns an error if TLS configuration or listener binding
  /// fails.
  pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
    let server = self.bind().await?;

    server.run().await;

    Ok(())
  }

  /// Set connection limits for subsequent server starts.
  ///
  /// # Examples
  ///
  /// ```rust
  /// let mut limits = windmark::router::ConnectionLimits::default();
  ///
  /// limits.max_connections = std::num::NonZeroUsize::new(256);
  /// limits.request_timeout = Some(std::time::Duration::from_secs(30));
  /// limits.drain_timeout = std::time::Duration::from_secs(5);
  ///
  /// windmark::router::Router::new().set_connection_limits(limits);
  /// ```
  pub const fn set_connection_limits(
    &mut self,
    limits: ConnectionLimits,
  ) -> &mut Self {
    self.connection_limits = limits;

    self
  }

  /// Serve requests until `shutdown` completes, then drain active connections
  /// for the configured period. Remaining handler futures are cancelled.
  /// Dropping this future also cancels the server's connection tasks.
  ///
  /// # Errors
  ///
  /// This method returns an error if TLS configuration or listener binding
  /// fails.
  ///
  /// # Panics
  ///
  /// This method panics if a partial registry lock has been poisoned.
  pub async fn run_until(
    &mut self,
    shutdown: impl std::future::Future<Output = ()>,
  ) -> Result<(), Box<dyn Error>> {
    let server = self.bind().await?;

    server.run_until(shutdown).await;

    Ok(())
  }

  /// Bind a server using the current configuration.
  ///
  /// Routes, callbacks, partials, TLS settings, and connection limits are
  /// captured at binding. Module registrations and module scheduling remain
  /// shared with this router and its clones. State owned by handlers and
  /// partials also retains its existing sharing semantics.
  ///
  /// The returned server owns the listener independently of this router.
  /// Set the port to zero and call `Server::local_addr` to discover an
  /// operating-system-assigned port before serving requests.
  ///
  /// # Errors
  ///
  /// This method returns an error if TLS configuration or listener binding
  /// fails.
  ///
  /// # Panics
  ///
  /// This method panics if a partial registry lock has been poisoned.
  pub async fn bind(&self) -> Result<Server, Box<dyn Error>> {
    let acceptor = match &self.ssl_acceptor {
      Some(acceptor) => acceptor.clone(),
      None => Arc::new(self.create_acceptor()?),
    };
    let listener =
      TcpListener::bind(format!("{}:{}", self.listener_address, self.port))
        .await?;
    let handler = Arc::new(RequestHandler {
      routes:              self.routes.clone(),
      error_handler:       self.error_handler.clone(),
      headers:             self
        .headers
        .lock()
        .expect("headers lock poisoned")
        .clone()
        .into(),
      footers:             self
        .footers
        .lock()
        .expect("footers lock poisoned")
        .clone()
        .into(),
      pre_route_callback:  self.pre_route_callback.clone(),
      post_route_callback: self.post_route_callback.clone(),
      character_set:       self.character_set.clone(),
      languages_joined:    self.languages.join(","),
      async_modules:       self.async_modules.clone(),
      modules:             self.modules.clone(),
      module_scheduling:   self.module_scheduling.clone(),
      options:             self.options.clone(),
      connection_limits:   self.connection_limits,
    });

    Ok(Server {
      listener,
      handler,
      acceptor,
    })
  }

  fn create_acceptor(&self) -> Result<SslAcceptor, Box<dyn Error>> {
    let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;

    builder.set_min_proto_version(Some(ssl::SslVersion::TLS1_2))?;

    if let Some(ref content) = self.certificate_content {
      let mut certificates =
        openssl::x509::X509::stack_from_pem(content.as_bytes())?.into_iter();
      let leaf = certificates.next().ok_or_else(|| {
        std::io::Error::new(
          std::io::ErrorKind::InvalidInput,
          "the certificate chain is empty",
        )
      })?;

      builder.set_certificate(&leaf)?;

      for certificate in certificates {
        builder.add_extra_chain_cert(certificate)?;
      }
    } else {
      builder.set_certificate_chain_file(&self.certificate_file_name)?;
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

    Ok(builder.build())
  }

  /// Use a custom `SslAcceptor` instead of building one from the credentials.
  ///
  /// # Examples
  ///
  /// ```rust,no_run
  /// use openssl::ssl;
  ///
  /// windmark::router::Router::new().set_ssl_acceptor({
  ///   let mut builder =
  ///     ssl::SslAcceptor::mozilla_intermediate_v5(ssl::SslMethod::tls()).unwrap();
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
    self.ssl_acceptor = Some(Arc::new(ssl_acceptor));

    self
  }

  /// Set a callback to run before a request is dispatched, even if the request
  /// does not match a route.
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

  /// Set a callback to modify the response after the handler, before
  /// serialisation and delivery.
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

  /// Attach a stateful module and await its initialisation before registering
  /// its request hooks.
  ///
  /// Like a stateless module, a stateful module extends a `Router` and gets
  /// full access to it during attachment. Third parties can provide modules
  /// with hooks that run during a request's lifecycle. Stateful modules retain
  /// state between hook calls.
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
  ///   clicks: std::sync::atomic::AtomicUsize,
  /// }
  ///
  /// #[async_trait::async_trait]
  /// impl windmark::module::AsyncModule for Clicker {
  ///   async fn on_attach(&mut self, _: &mut Router) {
  ///     info!("clicker has been attached!");
  ///   }
  ///
  ///   async fn on_pre_route(&self, context: &HookContext) {
  ///     let clicks = self
  ///       .clicks
  ///       .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
  ///       + 1;
  ///
  ///     info!(
  ///       "clicker has been called pre-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       clicks
  ///     );
  ///   }
  ///
  ///   async fn on_post_route(&self, context: &HookContext) {
  ///     info!(
  ///       "clicker has been called post-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       self.clicks.load(std::sync::atomic::Ordering::Relaxed)
  ///     );
  ///   }
  /// }
  ///
  /// # #[windmark::main]
  /// # async fn main() {
  /// Router::new().attach_async(Clicker::default()).await;
  /// # }
  /// ```
  pub async fn attach_async(
    &mut self,
    mut module: impl AsyncModule + 'static,
  ) -> &mut Self {
    module.on_attach(self).await;
    (*self.async_modules.lock().await).push(Arc::new(module));

    self
  }

  /// Attach a stateful module to a `Router`.
  ///
  /// Like a stateless module, a stateful module extends a `Router` and gets
  /// full access to it during attachment. Third parties can provide modules
  /// with hooks that run during a request's lifecycle. Stateful modules retain
  /// state between hook calls.
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
  ///   clicks: std::sync::atomic::AtomicUsize,
  /// }
  ///
  /// impl windmark::module::Module for Clicker {
  ///   fn on_attach(&mut self, _: &mut Router) {
  ///     info!("clicker has been attached!");
  ///   }
  ///
  ///   fn on_pre_route(&self, context: &HookContext) {
  ///     let clicks = self
  ///       .clicks
  ///       .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
  ///       + 1;
  ///
  ///     info!(
  ///       "clicker has been called pre-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       clicks
  ///     );
  ///   }
  ///
  ///   fn on_post_route(&self, context: &HookContext) {
  ///     info!(
  ///       "clicker has been called post-route on {} with {} clicks!",
  ///       context.url.path(),
  ///       self.clicks.load(std::sync::atomic::Ordering::Relaxed)
  ///     );
  ///   }
  /// }
  ///
  /// Router::new().attach(Clicker::default());
  /// ```
  pub fn attach(&mut self, mut module: impl Module + 'static) -> &mut Self {
    module.on_attach(self);
    (*self.modules.lock().expect("modules lock poisoned")).push(Arc::new(
      ModuleEntry {
        module:   Box::new(module),
        poisoned: AtomicBool::new(false),
      },
    ));

    self
  }

  /// Specify a custom character set.
  ///
  /// A character set specified in a [`Response`] overrides this setting.
  ///
  /// The default character set is `"utf-8"`.
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
  /// Languages specified in a [`Response`] override this setting.
  ///
  /// The default language is `"en"`.
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
  /// The default port is `1965`.
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

  /// Add optional features to the router.
  ///
  /// # Panics
  ///
  /// This method panics if enabling case-insensitive matching would make
  /// existing routes conflict. Case-insensitive matching remains disabled
  /// when validation fails.
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
      if *option == RouterOption::AllowCaseInsensitiveLookup {
        self
          .routes
          .enable_case_insensitive()
          .expect("routes conflict under case-insensitive matching");
      }

      if *option == RouterOption::AllowConcurrentModules {
        self
          .module_scheduling
          .concurrent
          .store(true, Ordering::Relaxed);
      } else {
        self.options.insert(*option);
      }
    }

    self
  }

  /// Toggle optional features for the router.
  ///
  /// # Panics
  ///
  /// This method panics if enabling case-insensitive matching would make
  /// existing routes conflict. Case-insensitive matching remains disabled
  /// when validation fails.
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
      if *option == RouterOption::AllowConcurrentModules {
        self
          .module_scheduling
          .concurrent
          .fetch_xor(true, Ordering::Relaxed);
      } else if self.options.contains(option) {
        self.remove_options(std::slice::from_ref(option));
      } else {
        self.add_options(std::slice::from_ref(option));
      }
    }

    self
  }

  /// Remove optional features from the router.
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
      if *option == RouterOption::AllowCaseInsensitiveLookup {
        self.routes.disable_case_insensitive();
      }

      if *option == RouterOption::AllowConcurrentModules {
        self
          .module_scheduling
          .concurrent
          .store(false, Ordering::Relaxed);
      } else {
        self.options.remove(option);
      }
    }

    self
  }

  /// Specify a custom listener address.
  ///
  /// The default listener address is `"0.0.0.0"`.
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
      routes:                Routes::default(),
      error_handler:         Arc::new(|_| {
        async {
          Response::not_found(
            "This capsule has not implemented an error handler...",
          )
        }
      }),
      private_key_file_name: String::new(),
      certificate_file_name: String::new(),
      headers:               Arc::new(Mutex::new(vec![])),
      footers:               Arc::new(Mutex::new(vec![])),
      ssl_acceptor:          None,
      pre_route_callback:    Arc::new((|_| {}) as fn(&HookContext)),
      post_route_callback:   Arc::new(
        (|_, _: &mut Response| {}) as fn(&HookContext, &mut Response),
      ),
      character_set:         "utf-8".to_string(),
      languages:             vec!["en".to_string()],
      port:                  1965,
      modules:               Arc::new(Mutex::new(vec![])),
      async_modules:         Arc::new(AsyncMutex::new(vec![])),
      module_scheduling:     Arc::new(ModuleScheduling::default()),
      options:               HashSet::new(),
      private_key_content:   None,
      certificate_content:   None,
      listener_address:      "0.0.0.0".to_string(),
      connection_limits:     ConnectionLimits::default(),
    }
  }
}

#[cfg(test)]
#[path = "../tests/router.rs"]
mod tests;
