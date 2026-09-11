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
  module::Module,
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

  fn on_pre_route(&mut self, _: &HookContext) {
    self.0.lock().unwrap().push("module-pre");
  }

  fn on_post_route(&mut self, _: &HookContext) {
    self.0.lock().unwrap().push("module-post");
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

  stream
    .write_all(format!("gemini://localhost{path}\r\n").as_bytes())
    .unwrap();
  stream.read_exact(&mut output).unwrap();
  assert_eq!(output, expected);
}

#[cfg_attr(
  feature = "tokio",
  tokio::test(flavor = "multi_thread", worker_threads = 2)
)]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn callbacks_partials_and_case_folding_keep_their_existing_order() {
  let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
  let address = reservation.local_addr().unwrap();
  let events: Events = Arc::default();
  let mut router = Router::new();

  router.set_listener_address("127.0.0.1");
  router.set_port(address.port());
  router.set_ssl_acceptor(acceptor());
  router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  router.attach(Observer(events.clone()));

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

  router.mount("/users/:name", move |context: RouteContext| {
    recorded.lock().unwrap().push("route");
    Response::success(context.parameters.get("name").unwrap())
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
  let client = run_client(move || {
    request(
      address,
      "/Users/Alice",
      b"20 text/gemini; charset=utf-8; lang=en\r\nHEADER\nalice!\nFOOTER",
    );
    request(
      address,
      "/binary",
      b"20 application/octet-stream\r\n\x00\xff",
    );
  });

  #[cfg(feature = "tokio")]
  client.await.unwrap();

  #[cfg(feature = "async-std")]
  client.await;

  #[cfg(feature = "tokio")]
  server.abort();
  #[cfg(feature = "async-std")]
  server.cancel().await;
  assert_eq!(
    *events.lock().unwrap(),
    [
      "attach",
      "module-pre",
      "callback-pre",
      "header",
      "footer",
      "route",
      "module-post",
      "callback-post",
      "module-pre",
      "callback-pre",
      "header",
      "footer",
      "route",
      "module-post",
      "callback-post",
    ]
  );
}
