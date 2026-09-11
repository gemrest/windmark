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
  router::Router,
  router_option::RouterOption,
};

type Events = Arc<Mutex<Vec<&'static str>>>;

struct Observer(Events);

impl Module for Observer {
  fn on_attach(&mut self, _: &mut Router) {
    self.0.lock().unwrap().push("attach");
  }

  fn on_pre_route(&mut self, context: &HookContext) {
    if context.url.path() == "/Users/Alice" {
      assert_eq!(
        context.parameters.as_ref().unwrap().get("Name"),
        Some("Alice")
      );
    }

    self.0.lock().unwrap().push("module-pre");
  }

  fn on_post_route(&mut self, context: &HookContext) {
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

  async fn on_pre_route(&mut self, _: &HookContext) {
    assert!(self.initialized);
    self.events.lock().unwrap().push("async-pre");
  }

  async fn on_post_route(&mut self, _: &HookContext) {
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

fn request_chunks(address: SocketAddr, chunks: &[&[u8]], expected: &[u8]) {
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

  let mut connector = SslConnector::builder(SslMethod::tls()).unwrap();

  connector.set_verify(SslVerifyMode::NONE);

  let mut stream = connector.build().connect("localhost", stream).unwrap();
  let mut output = vec![0; expected.len()];

  for chunk in chunks {
    stream.write_all(chunk).unwrap();
  }

  stream.read_exact(&mut output).unwrap();
  assert_eq!(output, expected);
}

async fn serve_requests(
  mut router: Router,
  client: impl FnOnce(SocketAddr) + Send + 'static,
) {
  let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = reservation.local_addr().unwrap();

  router.set_listener_address("127.0.0.1");
  router.set_port(address.port());
  router.set_ssl_acceptor(acceptor());
  drop(reservation);

  #[cfg(feature = "tokio")]
  let server = tokio::spawn(async move { router.run().await.unwrap() });
  #[cfg(feature = "async-std")]
  let server =
    async_std::task::spawn(async move { router.run().await.unwrap() });
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
      response.content.push('!');
    },
  );
  serve_requests(router, move |address| {
    request(
      address,
      "/Users/Alice",
      b"20 text/gemini; charset=utf-8; lang=en\r\nHEADER\nAlice!\nFOOTER",
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
    let invalid_url = "not a url";
    let utf8_error = format!(
      "59 The server (Windmark) received a bad request: {}\r\n",
      std::str::from_utf8(invalid_utf8.as_slice()).unwrap_err()
    );
    let url_error = format!(
      "59 The server (Windmark) received a bad request: {}\r\n",
      url::Url::parse(invalid_url).unwrap_err()
    );
    let prefix = "gemini://localhost/";
    let maximum_url = format!("{prefix}{}", "a".repeat(1022 - prefix.len()));

    assert_eq!(maximum_url.len(), 1022);
    request_chunks(
      address,
      &[b"gemini://localhost/caf\xc3", b"\xa9\r", b"\n"],
      success,
    );
    request_chunks(address, &[invalid_utf8, b"\r\n"], utf8_error.as_bytes());
    request_chunks(
      address,
      &[invalid_url.as_bytes(), b"\r\n"],
      url_error.as_bytes(),
    );
    request_chunks(address, &[maximum_url.as_bytes(), b"\r\n"], success);
    request_chunks(
      address,
      &[maximum_url.as_bytes(), b"a\r\n"],
      b"59 The server (Windmark) received a request exceeding 1024 bytes\r\n",
    );
  })
  .await;
}
