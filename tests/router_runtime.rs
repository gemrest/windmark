use openssl::{
  asn1::Asn1Time,
  ec::{EcGroup, EcKey},
  hash::MessageDigest,
  nid::Nid,
  pkey::PKey,
  ssl::{SslAcceptor, SslConnector, SslMethod, SslVerifyMode},
  x509::{X509NameBuilder, X509},
};
use std::{
  io::{Read, Write},
  net::{SocketAddr, TcpListener, TcpStream},
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};
use windmark::{
  context::{HookContext, RouteContext},
  module::{AsyncModule, Module},
  response::Response,
  router::{Router, Server},
  router_option::RouterOption,
};

type Events = Arc<Mutex<Vec<&'static str>>>;

struct Observer(Events);

impl Module for Observer {
  fn on_attach(&mut self, _: &mut Router) {
    self.0.lock().unwrap().push("attach");
  }

  fn on_pre_route(&self, context: &HookContext) {
    if context.url.path() == "/Users/Alice" {
      assert_eq!(
        context.parameters.as_ref().unwrap().get("Name"),
        Some("Alice")
      );
    }

    self.0.lock().unwrap().push("module-pre");
  }

  fn on_post_route(&self, context: &HookContext) {
    if context.url.path() == "/Users/Alice" {
      assert_eq!(
        context.parameters.as_ref().unwrap().get("Name"),
        Some("Alice")
      );
    }

    self.0.lock().unwrap().push("module-post");
  }
}

struct AsyncObserver {
  events:      Events,
  initialized: bool,
}

#[async_trait::async_trait]
impl AsyncModule for AsyncObserver {
  async fn on_attach(&mut self, _: &mut Router) {
    self.initialized = true;

    self.events.lock().unwrap().push("async-attach");
  }

  async fn on_pre_route(&self, _: &HookContext) {
    assert!(self.initialized);
    self.events.lock().unwrap().push("async-pre");
  }

  async fn on_post_route(&self, _: &HookContext) {
    self.events.lock().unwrap().push("async-post");
  }
}

fn acceptor() -> SslAcceptor {
  let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
  let key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
  let mut name = X509NameBuilder::new().unwrap();

  name.append_entry_by_text("CN", "localhost").unwrap();

  let name = name.build();
  let mut certificate = X509::builder().unwrap();

  certificate.set_version(2).unwrap();
  certificate.set_subject_name(&name).unwrap();
  certificate.set_issuer_name(&name).unwrap();
  certificate.set_pubkey(&key).unwrap();
  certificate
    .set_not_before(&Asn1Time::days_from_now(0).unwrap())
    .unwrap();
  certificate
    .set_not_after(&Asn1Time::days_from_now(1).unwrap())
    .unwrap();
  certificate.sign(&key, MessageDigest::sha256()).unwrap();

  let mut builder =
    SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

  builder.set_certificate(&certificate.build()).unwrap();
  builder.set_private_key(&key).unwrap();
  builder.build()
}

fn request(address: SocketAddr, path: &str, expected: &[u8]) {
  request_chunks(
    address,
    &[format!("gemini://localhost{path}\r\n").as_bytes()],
    expected,
  );
}

fn connect_tcp(address: SocketAddr) -> TcpStream {
  let started = Instant::now();
  let stream = loop {
    match TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
      Ok(stream) => break stream,

      Err(error) => {
        assert!(started.elapsed() < Duration::from_secs(5), "{error}");
        std::thread::sleep(Duration::from_millis(10));
      }
    }
  };

  stream
    .set_read_timeout(Some(Duration::from_secs(5)))
    .unwrap();
  stream
    .set_write_timeout(Some(Duration::from_secs(5)))
    .unwrap();

  stream
}

fn connect(address: SocketAddr) -> openssl::ssl::SslStream<TcpStream> {
  let stream = connect_tcp(address);
  let mut connector = SslConnector::builder(SslMethod::tls()).unwrap();

  connector.set_verify(SslVerifyMode::NONE);
  connector.build().connect("localhost", stream).unwrap()
}

fn request_chunks(address: SocketAddr, chunks: &[&[u8]], expected: &[u8]) {
  exchange(connect(address), chunks, expected);
}

fn exchange(
  mut stream: openssl::ssl::SslStream<TcpStream>,
  chunks: &[&[u8]],
  expected: &[u8],
) {
  let mut output = vec![0; expected.len()];

  for chunk in chunks {
    stream.write_all(chunk).unwrap();
  }

  stream.read_exact(&mut output).unwrap();
  assert_eq!(output, expected);
  assert_eq!(stream.read(&mut [0]).unwrap(), 0);
  assert!(stream
    .get_shutdown()
    .contains(openssl::ssl::ShutdownState::RECEIVED));
}

async fn serve_requests(
  router: Router,
  client: impl FnOnce(SocketAddr) + Send + 'static,
) {
  serve_requests_until(router, client, std::future::pending()).await;
}

async fn serve_requests_until(
  mut router: Router,
  client: impl FnOnce(SocketAddr) + Send + 'static,
  shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) {
  router.set_listener_address("127.0.0.1");
  router.set_port(0);
  router.set_ssl_acceptor(acceptor());

  let bound = router.bind().await.unwrap();

  serve_bound_requests(bound, client, shutdown).await;
}

async fn serve_bound_requests(
  bound: Server,
  client: impl FnOnce(SocketAddr) + Send + 'static,
  shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) {
  let address = bound.local_addr().unwrap();
  #[cfg(feature = "tokio")]
  let server = tokio::spawn(async move { bound.run_until(shutdown).await });
  #[cfg(feature = "async-std")]
  let server =
    async_std::task::spawn(async move { bound.run_until(shutdown).await });
  #[cfg(feature = "tokio")]
  let run_client = tokio::task::spawn_blocking;
  #[cfg(feature = "async-std")]
  let run_client = async_std::task::spawn_blocking;
  let client = run_client(move || client(address));
  #[cfg(feature = "tokio")]
  let result = client.await;

  #[cfg(feature = "async-std")]
  client.await;

  #[cfg(feature = "tokio")]
  server.abort();
  #[cfg(feature = "async-std")]
  server.cancel().await;
  #[cfg(feature = "tokio")]
  result.unwrap();
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn callbacks_partials_and_case_folding_keep_their_existing_order() {
  let events: Events = Arc::default();
  let mut router = Router::new();

  router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  router.attach(Observer(events.clone()));
  router
    .attach_async(AsyncObserver {
      events:      events.clone(),
      initialized: false,
    })
    .await;

  let recorded = events.clone();

  router.set_pre_route_callback(move |_: &HookContext| {
    recorded.lock().unwrap().push("callback-pre");
  });

  let recorded = events.clone();

  router.add_header(move |_: &RouteContext| {
    recorded.lock().unwrap().push("header");
    "HEADER".to_owned()
  });

  let recorded = events.clone();

  router.add_footer(move |_: &RouteContext| {
    recorded.lock().unwrap().push("footer");
    "FOOTER".to_owned()
  });

  let recorded = events.clone();

  router.mount("/Users/:Name", move |context: RouteContext| {
    recorded.lock().unwrap().push("route");
    Response::success(context.parameters.get("Name").unwrap())
  });

  let recorded = events.clone();

  router.mount("/binary", move |_| {
    recorded.lock().unwrap().push("route");
    Response::binary_success([0, 0xff], "application/octet-stream")
  });

  let recorded = events.clone();

  router.set_post_route_callback(
    move |_: &HookContext, response: &mut Response| {
      recorded.lock().unwrap().push("callback-post");

      if let Some(content) = response.content_mut() {
        content.push('!');
      }
    },
  );
  serve_requests(router, move |address| {
    request(
      address,
      "/Users/Alice",
      b"20 text/gemini; charset=utf-8; lang=en\r\nHEADER\nAlice!\nFOOTER\n",
    );
    request(
      address,
      "/binary",
      b"20 application/octet-stream\r\n\x00\xff",
    );
  })
  .await;
  assert_eq!(
    *events.lock().unwrap(),
    [
      "attach",
      "async-attach",
      "async-pre",
      "module-pre",
      "callback-pre",
      "header",
      "footer",
      "route",
      "async-post",
      "module-post",
      "callback-post",
      "async-pre",
      "module-pre",
      "callback-pre",
      "header",
      "footer",
      "route",
      "async-post",
      "module-post",
      "callback-post",
    ]
  );
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn request_framing_handles_split_characters_and_terminates_errors() {
  let mut router = Router::new();

  router.mount("/caf%C3%A9", |_| Response::success("accepted"));
  router.set_error_handler(|_| Response::success("accepted"));
  serve_requests(router, |address| {
    let success = b"20 text/gemini; charset=utf-8; lang=en\r\naccepted\n";
    let invalid_utf8 = b"gemini://localhost/\xff";
    let invalid_url = "relative";
    let utf8_error = format!(
      "59 The server (Windmark) received a bad request: {}\r\n",
      std::str::from_utf8(invalid_utf8.as_slice()).unwrap_err()
    );
    let url_error = format!(
      "59 The server (Windmark) received a bad request: {}\r\n",
      url::Url::parse(invalid_url).unwrap_err()
    );
    let prefix = "gemini://localhost/";
    let maximum_url = format!("{prefix}{}", "a".repeat(1024 - prefix.len()));

    assert_eq!(maximum_url.len(), 1024);
    request_chunks(
      address,
      &[b"gemini://localhost/caf%C", b"3%A9\r", b"\n"],
      success,
    );
    request_chunks(address, &[invalid_utf8, b"\r\n"], utf8_error.as_bytes());
    request_chunks(
      address,
      &[invalid_url.as_bytes(), b"\r\n"],
      url_error.as_bytes(),
    );
    request_chunks(address, &[maximum_url.as_bytes(), b"\r", b"\n"], success);

    let mut followed_by_extra_bytes = b"gemini://localhost/\r\n".to_vec();

    followed_by_extra_bytes.extend_from_slice(&[b'x'; 1024]);
    request_chunks(address, &[&followed_by_extra_bytes], success);
    request_chunks(
      address,
      &[maximum_url.as_bytes(), b"a\r\n"],
      b"59 The server (Windmark) received a bad request: request URI exceeds 1024 bytes\r\n",
    );
  })
  .await;
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn cloned_routers_retain_partials_and_running_servers_keep_their_snapshot(
) {
  let mut router = Router::new();

  router.mount("/", |_| Response::success("BODY"));
  router.add_header(|_: &windmark::context::RouteContext| "HEADER".to_owned());
  router.add_footer(|_: &windmark::context::RouteContext| "FOOTER".to_owned());

  let mut registration = router.clone();

  serve_requests(router.clone(), move |address| {
    let expected =
      b"20 text/gemini; charset=utf-8; lang=en\r\nHEADER\nBODY\nFOOTER\n";

    request(address, "/", expected);
    registration.add_header(|_: &windmark::context::RouteContext| {
      "SECOND HEADER".to_owned()
    });
    registration.add_footer(|_: &windmark::context::RouteContext| {
      "SECOND FOOTER".to_owned()
    });
    request(address, "/", expected);
  })
  .await;

  for configured in [router.clone(), router] {
    serve_requests(configured, |address| {
      request(
        address,
        "/",
        b"20 text/gemini; charset=utf-8; lang=en\r\nHEADER\nSECOND HEADER\nBODY\nFOOTER\nSECOND FOOTER\n",
      );
    })
    .await;
  }
}

#[derive(Default)]
struct HookGate {
  state: std::sync::Mutex<(bool, Option<std::task::Waker>)>,
}

impl HookGate {
  fn release(&self) {
    let mut state = self.state.lock().unwrap();

    state.0 = true;

    if let Some(waker) = state.1.take() {
      waker.wake();
    }
  }

  async fn wait_async(&self) {
    std::future::poll_fn(|context| {
      let mut state = self.state.lock().unwrap();

      if state.0 {
        std::task::Poll::Ready(())
      } else {
        state.1 = Some(context.waker().clone());

        std::task::Poll::Pending
      }
    })
    .await;
  }
}

struct GatedModule {
  second:     bool,
  post_route: bool,
  gate:       Arc<HookGate>,
  route_gate: Arc<HookGate>,
  entered:    std::sync::mpsc::Sender<()>,
  progressed: std::sync::mpsc::Sender<()>,
  events:     Arc<Mutex<Vec<(bool, String)>>>,
}

impl GatedModule {
  fn enter(&self, context: &HookContext, post_route: bool) -> bool {
    if self.post_route != post_route {
      return false;
    }

    self
      .events
      .lock()
      .unwrap()
      .push((self.second, context.url.path().to_owned()));

    if self.second && context.url.path() == "/first" {
      self.entered.send(()).unwrap();
      self.route_gate.release();

      return true;
    }

    if !self.second && context.url.path() == "/second" {
      self.progressed.send(()).unwrap();
    }

    false
  }
}

#[async_trait::async_trait]
impl AsyncModule for GatedModule {
  async fn on_pre_route(&self, context: &HookContext) {
    if self.enter(context, false) {
      self.gate.wait_async().await;
    }
  }

  async fn on_post_route(&self, context: &HookContext) {
    if self.enter(context, true) {
      self.gate.wait_async().await;
    }
  }
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn shared_async_modules_allow_progress_in_both_hook_phases() {
  for post_route in [false, true] {
    let mut router = Router::new();
    let gate = Arc::new(HookGate::default());
    let route_gate = Arc::new(HookGate::default());
    let (route_entered, route_entry) = std::sync::mpsc::channel();
    let events = Arc::new(Mutex::new(Vec::new()));
    let (entered, entry) = std::sync::mpsc::channel();
    let (progressed, progress) = std::sync::mpsc::channel();

    router.add_options(&[RouterOption::AllowConcurrentModules]);
    router.set_error_handler(|_| Response::success("BODY"));

    let route_wait = route_gate.clone();

    router.mount("/second", move |_| {
      let route_wait = route_wait.clone();
      let route_entered = route_entered.clone();

      async move {
        if post_route {
          route_entered.send(()).unwrap();
          route_wait.wait_async().await;
        }

        Response::success("BODY")
      }
    });

    for second in [false, true] {
      let module = GatedModule {
        second,
        post_route,
        gate: gate.clone(),
        route_gate: route_gate.clone(),
        entered: entered.clone(),
        progressed: progressed.clone(),
        events: events.clone(),
      };

      router.attach_async(module).await;
    }

    serve_requests(router, move |address| {
      let expected = b"20 text/gemini; charset=utf-8; lang=en\r\nBODY\n";
      let first_connection = connect(address);
      let second_connection = connect(address);
      let (start_second, second_start) = std::sync::mpsc::channel();
      let second = std::thread::spawn(move || {
        second_start.recv_timeout(Duration::from_secs(5)).unwrap();
        exchange(
          second_connection,
          &[b"gemini://localhost/second\r\n"],
          expected,
        );
      });

      if post_route {
        start_second.send(()).unwrap();
        route_entry.recv_timeout(Duration::from_secs(5)).unwrap();
      }

      let first = std::thread::spawn(move || {
        exchange(
          first_connection,
          &[b"gemini://localhost/first\r\n"],
          expected,
        );
      });

      entry.recv_timeout(Duration::from_secs(5)).unwrap();

      if !post_route {
        start_second.send(()).unwrap();
      }

      let progressed = progress.recv_timeout(Duration::from_secs(5));

      second.join().unwrap();
      gate.release();
      first.join().unwrap();
      progressed.expect("the second request did not reach the first module");
      assert_eq!(
        *events.lock().unwrap(),
        [
          (false, "/first".to_owned()),
          (true, "/first".to_owned()),
          (false, "/second".to_owned()),
          (true, "/second".to_owned())
        ]
      );
    })
    .await;
  }
}

struct PanickingModule {
  post_route: bool,
}

impl Module for PanickingModule {
  fn on_pre_route(&self, context: &HookContext) {
    assert!(
      self.post_route || context.url.path() != "/panic",
      "pre-route panic"
    );
  }

  fn on_post_route(&self, context: &HookContext) {
    assert!(
      !self.post_route || context.url.path() != "/panic",
      "post-route panic"
    );
  }
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn poisoned_modules_do_not_disable_other_modules() {
  for post_route in [false, true] {
    let mut router = Router::new();
    let events = Arc::new(Mutex::new(Vec::new()));

    router.set_error_handler(|_| Response::success("BODY"));
    router.attach(PanickingModule {
      post_route,
    });
    router.attach(Observer(events.clone()));
    events.lock().unwrap().clear();
    serve_requests(router, move |address| {
      let expected = b"20 text/gemini; charset=utf-8; lang=en\r\nBODY\n";

      assert!(std::panic::catch_unwind(|| {
        request(address, "/panic", expected)
      })
      .is_err());
      events.lock().unwrap().clear();
      request(address, "/healthy", expected);
      assert_eq!(*events.lock().unwrap(), ["module-pre", "module-post"]);
    })
    .await;
  }
}

mod named_routes {
  use rossweisse::route;
  use windmark::{context::RouteContext, response::Response};

  #[rossweisse::router]
  pub struct Capsule;

  #[rossweisse::router]
  impl Capsule {
    #[rossweisse::route(index)]
    pub fn index(_: RouteContext) -> Response { Response::success("INDEX") }

    #[route]
    pub fn about(_: RouteContext) -> Response { Response::success("ABOUT") }

    #[route]
    pub fn __router_index(_: RouteContext) -> Response {
      Response::success("ORDINARY")
    }
  }
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn index_route_attributes_preserve_names_and_select_the_root_path() {
  let mut capsule = named_routes::Capsule::new();

  serve_requests(capsule.router().clone(), |address| {
    for (path, body) in [
      ("/", "INDEX"),
      ("/about", "ABOUT"),
      ("/__router_index", "ORDINARY"),
    ] {
      request(
        address,
        path,
        format!("20 text/gemini; charset=utf-8; lang=en\r\n{body}\n")
          .as_bytes(),
      );
    }
  })
  .await;
}

async fn pause(duration: Duration) {
  #[cfg(feature = "tokio")]
  tokio::time::sleep(duration).await;
  #[cfg(feature = "async-std")]
  async_std::task::sleep(duration).await;
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn connection_deadlines_release_stalled_handshakes_and_requests() {
  let mut router = Router::new();
  let mut limits = windmark::router::ConnectionLimits::default();
  let timeout = Duration::from_secs(1);

  limits.max_connections = std::num::NonZeroUsize::new(1);
  limits.handshake_timeout = Some(timeout);
  limits.request_timeout = Some(timeout);
  limits.shutdown_timeout = Some(timeout);

  router.set_connection_limits(limits);
  router.mount("/", |_| Response::success("ready"));
  serve_requests(router, |address| {
    let mut stalled = connect_tcp(address);

    stalled
      .set_read_timeout(Some(Duration::from_secs(5)))
      .unwrap();

    let result = stalled.read(&mut [0]);

    assert!(
      matches!(result, Ok(0))
        || result.is_err_and(|error| {
          error.kind() == std::io::ErrorKind::ConnectionReset
        })
    );

    let mut stalled = connect(address);

    stalled.write_all(b"gemini://localhost/").unwrap();

    let started = Instant::now();
    let result = stalled.read(&mut [0]);

    assert!(result.is_err() || matches!(result, Ok(0)));
    assert!(started.elapsed() < Duration::from_secs(4));
    request(
      address,
      "/",
      b"20 text/gemini; charset=utf-8; lang=en\r\nready\n",
    );
  })
  .await;
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn connection_admission_caps_active_handlers() {
  use std::sync::atomic::{AtomicUsize, Ordering};

  let active = Arc::new(AtomicUsize::new(0));
  let peak = Arc::new(AtomicUsize::new(0));
  let observed = peak.clone();
  let mut router = Router::new();
  let mut limits = windmark::router::ConnectionLimits::default();

  limits.max_connections = std::num::NonZeroUsize::new(1);

  router.set_connection_limits(limits);
  router.mount("/", move |_| {
    let active = active.clone();
    let peak = peak.clone();

    async move {
      let count = active.fetch_add(1, Ordering::SeqCst) + 1;

      peak.fetch_max(count, Ordering::SeqCst);
      pause(Duration::from_millis(40)).await;
      active.fetch_sub(1, Ordering::SeqCst);

      Response::success("ready")
    }
  });
  serve_requests(router, move |address| {
    let start = Arc::new(std::sync::Barrier::new(3));
    let clients: Vec<_> = (0..2)
      .map(|_| {
        let start = start.clone();

        std::thread::spawn(move || {
          start.wait();
          request(
            address,
            "/",
            b"20 text/gemini; charset=utf-8; lang=en\r\nready\n",
          );
        })
      })
      .collect();

    start.wait();

    for client in clients {
      client.join().unwrap();
    }

    assert_eq!(observed.load(Ordering::SeqCst), 1);
  })
  .await;
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn shutdown_drains_completed_handlers_and_cancels_unfinished_handlers() {
  use std::sync::atomic::{AtomicBool, Ordering};

  struct Completion(Arc<AtomicBool>);

  impl Drop for Completion {
    fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); }
  }

  for finish in [true, false] {
    let started = Arc::new(AtomicBool::new(false));
    let stopping = started.clone();
    let dropped = Arc::new(AtomicBool::new(false));
    let observed = dropped.clone();
    let mut router = Router::new();
    let mut limits = windmark::router::ConnectionLimits::default();

    limits.drain_timeout = Duration::from_millis(100);

    router.set_connection_limits(limits);
    router.mount("/", move |_| {
      let completion = Completion(dropped.clone());
      let started = started.clone();

      async move {
        started.store(true, Ordering::SeqCst);

        if finish {
          pause(Duration::from_millis(10)).await;
        } else {
          std::future::pending::<()>().await;
        }

        drop(completion);

        Response::success("finished")
      }
    });
    serve_requests_until(
      router,
      move |address| {
        let mut stream = connect(address);
        let started = Instant::now();

        if finish {
          exchange(
            stream,
            &[b"gemini://localhost/\r\n"],
            b"20 text/gemini; charset=utf-8; lang=en\r\nfinished\n",
          );
        } else {
          stream.write_all(b"gemini://localhost/\r\n").unwrap();

          let result = stream.read(&mut [0]);

          assert!(result.is_err() || matches!(result, Ok(0)));
          assert!(started.elapsed() < Duration::from_secs(2));
        }

        assert!(observed.load(Ordering::SeqCst));
      },
      async move {
        while !stopping.load(Ordering::SeqCst) {
          pause(Duration::from_millis(1)).await;
        }
      },
    )
    .await;
  }
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn invalid_response_headers_become_complete_failure_responses() {
  let mut router = Router::new();

  router.mount("/status", |_| Response::new(99, "invalid"));
  router.mount("/metadata", |_| {
    let mut response = Response::success("must not escape");

    response.with_mime("text/gemini\r\nINJECTED");

    response
  });
  serve_requests(router, |address| {
    for path in ["/status", "/metadata"] {
      request(
        address,
        path,
        b"40 The server could not encode the response\r\n",
      );
    }
  })
  .await;
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn startup_preserves_an_existing_logger() {
  struct Logger;

  impl log::Log for Logger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool { false }

    fn log(&self, _: &log::Record<'_>) {}

    fn flush(&self) {}
  }

  static LOGGER: Logger = Logger;

  log::set_logger(&LOGGER).unwrap();

  for _ in 0..2 {
    let mut router = Router::new();

    router.mount("/", |_| Response::success("ready"));
    serve_requests(router, |address| {
      request(
        address,
        "/",
        b"20 text/gemini; charset=utf-8; lang=en\r\nready\n",
      );
    })
    .await;
  }
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn write_deadlines_release_connections_when_clients_stop_reading() {
  use std::num::NonZeroUsize;
  use windmark::router::ConnectionLimits;

  let mut limits = ConnectionLimits::default();

  limits.max_connections = NonZeroUsize::new(1);
  limits.write_timeout = Some(Duration::from_millis(50));

  let mut router = Router::new();

  router.set_connection_limits(limits);
  router.mount("/large", |_| {
    Response::binary_success(
      vec![0; 32 * 1024 * 1024],
      "application/octet-stream",
    )
  });
  router.mount("/", |_| Response::success("ready"));
  serve_requests(router, |address| {
    let mut stalled = connect(address);

    stalled.write_all(b"gemini://localhost/large\r\n").unwrap();

    let started = Instant::now();

    request(
      address,
      "/",
      b"20 text/gemini; charset=utf-8; lang=en\r\nready\n",
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    drop(stalled);
  })
  .await;
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn bound_servers_own_their_ports_and_report_binding_errors() {
  let mut router = Router::new();

  router.set_listener_address("127.0.0.1");
  router.set_port(0);
  router.set_ssl_acceptor(acceptor());

  let server = router.bind().await.unwrap();
  let address = server.local_addr().unwrap();

  assert_ne!(address.port(), 0);
  router.set_port(address.port());
  assert!(router.bind().await.is_err());
  drop(server);

  let rebound = router.bind().await.unwrap();

  assert_eq!(rebound.local_addr().unwrap(), address);
  rebound.run_until(std::future::ready(())).await;

  let listener = TcpListener::bind(address).unwrap();

  assert_eq!(listener.local_addr().unwrap(), address);
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn binding_captures_routes_but_preserves_shared_module_registration() {
  let mut router = Router::new();
  let events: Events = Arc::default();

  router.set_listener_address("127.0.0.1");
  router.set_port(0);
  router.set_ssl_acceptor(acceptor());
  router.set_error_handler(|_| Response::success("before"));

  let first = router.bind().await.unwrap();

  router.mount("/", |_| Response::success("after"));

  let second = router.bind().await.unwrap();

  assert_ne!(first.local_addr().unwrap(), second.local_addr().unwrap());
  router.attach(Observer(events.clone()));
  drop(router);
  serve_bound_requests(
    first,
    |address| {
      request(
        address,
        "/",
        b"20 text/gemini; charset=utf-8; lang=en\r\nbefore\n",
      );
    },
    std::future::pending(),
  )
  .await;
  serve_bound_requests(
    second,
    |address| {
      request(
        address,
        "/",
        b"20 text/gemini; charset=utf-8; lang=en\r\nafter\n",
      );
    },
    std::future::pending(),
  )
  .await;
  assert_eq!(
    *events.lock().unwrap(),
    [
      "attach",
      "module-pre",
      "module-post",
      "module-pre",
      "module-post"
    ]
  );
}
