use super::{
  call_async_modules,
  call_modules,
  resolve_lookup_path,
  status_line,
  HookPhase,
  ModuleScheduling,
  Router,
};
use crate::{
  context::{HookContext, Parameters, RouteContext},
  handler::Partial,
  module::{AsyncModule, Module},
  response::Response,
  router_option::RouterOption::{
    self,
    AddMissingTrailingSlash,
    AllowCaseInsensitiveLookup,
    AllowConcurrentModules,
    RemoveExtraTrailingSlash,
  },
};
use matchit::Router as MatchRouter;
use std::{
  collections::HashSet,
  sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
    Arc,
    Mutex,
  },
  time::Duration,
};

const HOOK_TIMEOUT: Duration = Duration::from_secs(5);

fn hook_context() -> HookContext {
  HookContext {
    peer_address: None,
    url:          "gemini://localhost/".parse().unwrap(),
    parameters:   None,
    certificate:  None,
  }
}

fn resolve(
  options: &[RouterOption],
  request_path: &str,
  routes: &[&str],
) -> String {
  let mut matcher = MatchRouter::new();

  for route in routes {
    matcher.insert(*route, ()).unwrap();
  }

  let option_set = options.iter().copied().collect::<HashSet<_>>();

  resolve_lookup_path(&option_set, request_path, |candidate| {
    let lookup = if options.contains(&AllowCaseInsensitiveLookup) {
      candidate.to_ascii_lowercase()
    } else {
      candidate.to_owned()
    };

    matcher.at(&lookup).is_ok()
  })
}

#[test]
fn lookup_paths_without_fixes_are_returned_unchanged() {
  assert_eq!(resolve(&[], "/foo", &["/foo"]), "/foo");
  assert_eq!(resolve(&[], "/missing", &["/foo"]), "/missing");
  assert_eq!(resolve(&[], "/posts/42", &["/posts/{id}"]), "/posts/42");
  assert_eq!(
    resolve(&[AllowCaseInsensitiveLookup], "/FoO", &["/foo"]),
    "/FoO"
  );
}

#[test]
fn trailing_slash_fixes_apply_only_when_their_target_route_exists() {
  let remove_extra_trailing_slash = [RemoveExtraTrailingSlash];

  assert_eq!(
    resolve(&remove_extra_trailing_slash, "/foo/", &["/foo"]),
    "/foo"
  );
  assert_eq!(resolve(&remove_extra_trailing_slash, "/", &["/"]), "/");
  assert_eq!(
    resolve(&remove_extra_trailing_slash, "/foo/", &["/bar"]),
    "/foo/"
  );
  assert_eq!(
    resolve(&remove_extra_trailing_slash, "/foo/", &["/foo", "/foo/"]),
    "/foo/"
  );
  assert_eq!(
    resolve(&[AddMissingTrailingSlash], "/foo", &["/foo/"]),
    "/foo/"
  );
  assert_eq!(
    resolve(
      &[AllowCaseInsensitiveLookup, RemoveExtraTrailingSlash],
      "/Foo/",
      &["/foo"],
    ),
    "/Foo",
  );
}

fn default_status_line(response: &Response) -> Result<String, &'static str> {
  let router = Router::default();

  status_line(response, &router.character_set, &router.languages.join(","))
}

#[test]
fn status_lines_combine_response_metadata_with_router_defaults() {
  for (response, expected) in [
    (
      Response::success("hi"),
      "20 text/gemini; charset=utf-8; lang=en",
    ),
    (
      Response::success("hi").with_mime("text/plain").clone(),
      "20 text/plain; charset=utf-8; lang=en",
    ),
    (
      Response::success("hi").with_character_set("UTF-8").clone(),
      "20 text/gemini; charset=UTF-8; lang=en",
    ),
    (
      Response::success("hi").with_languages(["en", "fr"]).clone(),
      "20 text/gemini; charset=utf-8; lang=\"en,fr\"",
    ),
    (
      Response::binary_success([0xFFu8], "image/png"),
      "20 image/png",
    ),
    (Response::new(21, ""), "20 application/octet-stream"),
    (Response::temporary_redirect("/elsewhere"), "30 /elsewhere"),
    (Response::new(40, "down\nfor maintenance"), "40 down"),
    (Response::input("Your name?"), "10 Your name?"),
  ] {
    assert_eq!(default_status_line(&response).as_deref(), Ok(expected));
  }
}

#[test]
fn footer_rendering_preserves_empty_values_and_callback_order() {
  let context = RouteContext {
    peer_address: None,
    url:          "gemini://localhost/footer".parse().unwrap(),
    parameters:   Parameters::default(),
    certificate:  None,
  };

  for values in [vec![], vec![""], vec!["", ""], vec!["a", "", "b\n"]] {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let partials: Vec<Arc<dyn Partial>> = values
      .iter()
      .enumerate()
      .map(|(index, value)| {
        let calls = calls.clone();
        let value = (*value).to_owned();

        Arc::new(move |context: &RouteContext| {
          assert_eq!(context.url.path(), "/footer");
          calls.lock().unwrap().push(index);
          value.clone()
        }) as Arc<dyn Partial>
      })
      .collect();

    assert_eq!(super::render_footer(&partials, &context), values.join("\n"));
    assert_eq!(
      *calls.lock().unwrap(),
      (0..values.len()).collect::<Vec<_>>()
    );
  }
}

#[test]
fn path_resolution_preserves_unicode_and_probe_order() {
  let options = [AllowCaseInsensitiveLookup, AddMissingTrailingSlash]
    .into_iter()
    .collect();
  let probes = std::cell::RefCell::new(Vec::new());
  let path = resolve_lookup_path(&options, "/Users/ÄLICE", |candidate| {
    probes.borrow_mut().push(candidate.to_owned());

    candidate == "/Users/ÄLICE/"
  });

  assert_eq!(path, "/Users/ÄLICE/");
  assert_eq!(*probes.borrow(), ["/Users/ÄLICE", "/Users/ÄLICE/"]);
}

#[test]
fn response_assembly_preserves_publicly_constructible_states() {
  for (status, expected_line, expected_body) in [
    (
      20,
      "20 text/gemini; charset=utf-8; lang=en",
      b"HEADERbody\nFOOTER\n".as_slice(),
    ),
    (21, "20 application/octet-stream", b"\x00\xff".as_slice()),
    (22, "20 application/octet-stream", b"\x00\xff".as_slice()),
    (40, "40 body", b"".as_slice()),
  ] {
    let mut response = Response::new(status, "body");

    if let Some(bytes) = response.binary_content_mut() {
      *bytes = vec![0, 0xff];
    }

    let status_line = default_status_line(&response).unwrap();
    let expected = [expected_line.as_bytes(), b"\r\n", expected_body].concat();

    assert_eq!(status_line, expected_line);
    assert_eq!(
      response.clone().serialize_body("HEADER", "FOOTER"),
      expected_body
    );
    assert_eq!(
      super::serialize_response(response, &status_line, "HEADER", "FOOTER"),
      expected
    );
  }

  for status in [21, 22] {
    let response = Response::new(status, "ignored");

    assert!(response
      .clone()
      .serialize_body("HEADER", "FOOTER")
      .is_empty());
    assert_eq!(
      super::serialize_response(
        response,
        "20 application/octet-stream",
        "HEADER",
        "FOOTER"
      ),
      b"20 application/octet-stream\r\n"
    );
  }
}

#[test]
fn case_insensitive_routes_preserve_parameter_names_and_values() {
  let mut router = Router::new();

  router.mount("/Users/{Name}/Files/{*FilePath}", |_| {
    Response::success("file")
  });
  router.add_options(&[AllowCaseInsensitiveLookup]);

  let matched = router
    .routes
    .at("/uSeRs/ÄLICE/fIlEs/Photo%2FOne.PNG")
    .unwrap();

  assert_eq!(matched.parameters.get("Name"), Some("ÄLICE"));
  assert_eq!(matched.parameters.get("FilePath"), Some("Photo%2FOne.PNG"));
  assert_eq!(matched.parameters.get("name"), None);
}

#[test]
fn case_insensitive_routes_preserve_static_priority_and_inline_parameters() {
  let mut router = Router::new();

  router.add_options(&[AllowCaseInsensitiveLookup]);
  router.mount("/Users/{Name}", |_| Response::success("user"));
  router.mount("/Users/Me", |_| Response::success("profile"));
  router.mount("/Files/Prefix{Name}", |_| Response::success("file"));

  let profile = router.routes.at("/users/ME").unwrap();
  let file = router.routes.at("/FILES/prefixREPORT.PDF").unwrap();

  assert!(profile.parameters.is_empty());
  assert_eq!(file.parameters.get("Name"), Some("REPORT.PDF"));
}

#[test]
fn legacy_parameter_syntax_is_rejected_with_the_braced_form() {
  for (route, braced_route) in [
    ("/users/:name", "/users/{name}"),
    ("/files/*path", "/files/{*path}"),
    ("/:first/static/:second/", "/{first}/static/{second}/"),
  ] {
    let panic = std::panic::catch_unwind(|| {
      Router::new().mount(route, |_| Response::success("legacy"));
    })
    .unwrap_err();
    let message = panic.downcast_ref::<String>().unwrap();

    assert!(
      message.contains(&format!("write `{braced_route}`")),
      "{message}"
    );
  }
}

#[test]
fn literal_colons_and_escaped_braces_mount_as_static_text() {
  let mut router = Router::new();

  router.mount("/time/12:30", |_| Response::success("time"));
  router.mount("/Literal/{{Braces}}/{Name}.GMI", |_| {
    Response::success("braces")
  });
  router.add_options(&[AllowCaseInsensitiveLookup]);

  let matched = router.routes.at("/literal/{braces}/ÄLICE.gmi").unwrap();

  assert!(router.routes.contains("/TIME/12:30"));
  assert_eq!(matched.parameters.get("Name"), Some("ÄLICE"));
}

#[test]
fn case_insensitive_routes_restore_repeated_values_from_their_own_positions() {
  let mut router = Router::new();

  router.mount("/Same/{First}/{Second}", |_| Response::success("pair"));
  router.add_options(&[AllowCaseInsensitiveLookup]);

  let matched = router.routes.at("/same/SAME/SaMe").unwrap();

  assert_eq!(matched.parameters.get("First"), Some("SAME"));
  assert_eq!(matched.parameters.get("Second"), Some("SaMe"));
}

#[test]
fn case_insensitive_matching_can_be_disabled_reenabled_and_cloned() {
  let mut router = Router::new();

  router.mount("/Mixed/{Name}", |_| Response::success("mixed"));
  assert!(!router.routes.contains("/mixed/Alice"));
  router.add_options(&[AllowCaseInsensitiveLookup]);
  assert!(router.routes.contains("/mixed/Alice"));

  let cloned = router.clone();

  router.toggle_options(&[AllowCaseInsensitiveLookup]);
  assert!(!router.routes.contains("/mixed/Alice"));
  assert!(cloned.routes.contains("/mixed/Alice"));
  router.toggle_options(&[AllowCaseInsensitiveLookup]);
  assert!(router.routes.contains("/mixed/Alice"));
  router.remove_options(&[AllowCaseInsensitiveLookup]);
  assert!(!router.routes.contains("/mixed/Alice"));
  assert!(router.routes.contains("/Mixed/Alice"));
}

#[test]
fn enabling_case_insensitive_matching_rejects_existing_conflicts() {
  let mut router = Router::new();

  router.mount("/Foo", |_| Response::success("upper"));
  router.mount("/foo", |_| Response::success("lower"));

  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    router.add_options(&[AllowCaseInsensitiveLookup]);
  }));

  assert!(result.is_err());
  assert!(!router.options.contains(&AllowCaseInsensitiveLookup));
  assert!(router.routes.contains("/Foo"));
  assert!(router.routes.contains("/foo"));
  assert!(!router.routes.contains("/FOO"));
}

#[test]
fn mounting_case_conflicts_preserves_existing_routes() {
  let mut router = Router::new();

  router.add_options(&[AllowCaseInsensitiveLookup]);
  router.mount("/Users/{Name}", |_| Response::success("user"));

  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    router.mount("/users/{Other}", |_| Response::success("conflict"));
  }));

  assert!(result.is_err());

  let matched = router.routes.at("/USERS/Alice").unwrap();

  assert_eq!(matched.parameters.get("Name"), Some("Alice"));
  assert_eq!(matched.parameters.get("Other"), None);
  router.mount("/Other", |_| Response::success("other"));
  assert!(router.routes.contains("/OTHER"));
}

#[test]
fn trailing_slash_fixes_preserve_case_insensitive_parameters() {
  let mut router = Router::new();

  router.mount("/Users/{Name}/", |_| Response::success("user"));
  router.add_options(&[AllowCaseInsensitiveLookup, AddMissingTrailingSlash]);

  let path =
    resolve_lookup_path(&router.options, "/uSeRs/Alice", |candidate| {
      router.routes.contains(candidate)
    });
  let matched = router.routes.at(&path).unwrap();

  assert_eq!(path, "/uSeRs/Alice/");
  assert_eq!(matched.parameters.get("Name"), Some("Alice"));
}

#[test]
fn shared_synchronous_modules_allow_overlapping_invocations() {
  struct SignalingModule(mpsc::Sender<()>);

  impl Module for SignalingModule {
    fn on_pre_route(&self, _: &HookContext) { self.0.send(()).unwrap(); }
  }

  struct BlockingModule {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
    blocked: AtomicBool,
  }

  impl Module for BlockingModule {
    fn on_pre_route(&self, _: &HookContext) {
      if !self.blocked.swap(true, Ordering::Relaxed) {
        self.entered.send(()).unwrap();
        self
          .release
          .lock()
          .unwrap()
          .recv_timeout(HOOK_TIMEOUT)
          .unwrap();
      }
    }
  }

  let (signal, progress) = mpsc::channel();
  let (entered, entry) = mpsc::channel();
  let (release, resume) = mpsc::channel();
  let mut router = Router::new();

  router.attach(SignalingModule(signal));
  router.attach(BlockingModule {
    entered,
    release: Mutex::new(resume),
    blocked: AtomicBool::new(false),
  });

  let modules = router.modules.clone();
  let scheduling = Arc::new(ModuleScheduling::default());
  let first_scheduling = scheduling.clone();
  let first_modules = modules.clone();

  scheduling.concurrent.store(true, Ordering::Relaxed);

  let first = std::thread::spawn(move || {
    call_modules(&first_modules, &first_scheduling, |module| {
      module.on_pre_route(&hook_context());
    });
  });

  progress.recv_timeout(HOOK_TIMEOUT).unwrap();
  entry.recv_timeout(HOOK_TIMEOUT).unwrap();

  let second = std::thread::spawn(move || {
    call_modules(&modules, &scheduling, |module| {
      module.on_pre_route(&hook_context());
    });
  });
  let advanced = progress.recv_timeout(HOOK_TIMEOUT);

  second.join().unwrap();
  release.send(()).unwrap();
  first.join().unwrap();
  advanced.expect("the first module was blocked by the second module");
}

#[test]
fn module_scheduling_options_are_shared_across_router_clones() {
  let mut router = Router::new();
  let mut cloned = router.clone();
  let enabled = || router.module_scheduling.concurrent.load(Ordering::Relaxed);

  assert!(!enabled());
  cloned.add_options(&[AllowConcurrentModules]);
  assert!(enabled());
  cloned.toggle_options(&[AllowConcurrentModules]);
  assert!(!enabled());
  cloned.toggle_options(&[AllowConcurrentModules]);
  assert!(enabled());
  router.remove_options(&[AllowConcurrentModules]);
  assert!(!cloned.module_scheduling.concurrent.load(Ordering::Relaxed));
}

#[test]
fn synchronous_hook_phases_take_exclusive_or_shared_permits() {
  let mut router = Router::new();

  router.attach(SchedulingProbe);
  call_modules(&router.modules, &router.module_scheduling, |_| {
    assert!(router.module_scheduling.synchronous.try_read().is_err());
  });
  router.add_options(&[AllowConcurrentModules]);
  call_modules(&router.modules, &router.module_scheduling, |_| {
    assert!(router.module_scheduling.synchronous.try_read().is_ok());
    assert!(router.module_scheduling.synchronous.try_write().is_err());
  });
}

struct SchedulingProbe;

impl Module for SchedulingProbe {}

#[async_trait::async_trait]
impl AsyncModule for SchedulingProbe {}

#[cfg_attr(feature = "tokio", tokio::test(flavor = "current_thread"))]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn exclusive_async_phases_wait_for_existing_concurrent_phases() {
  use std::{future::Future, pin::pin, task::Poll};

  let mut router = Router::new();
  let context = hook_context();

  router.attach_async(SchedulingProbe).await;

  for phase in [HookPhase::PreRoute, HookPhase::PostRoute] {
    let shared = router.module_scheduling.asynchronous.read().await;
    let mut dispatch = pin!(call_async_modules(
      &router.async_modules,
      &router.module_scheduling,
      &context,
      phase,
    ));

    std::future::poll_fn(|context| {
      assert!(dispatch.as_mut().poll(context).is_pending());
      Poll::Ready(())
    })
    .await;
    drop(shared);

    dispatch.await;
  }

  router.add_options(&[AllowConcurrentModules]);

  let scheduling = router.module_scheduling.clone();
  let modules = router.async_modules.clone();
  let shared = scheduling.asynchronous.read().await;
  let mut concurrent = pin!(call_async_modules(
    &modules,
    &scheduling,
    &context,
    HookPhase::PreRoute,
  ));

  std::future::poll_fn(|context| {
    assert!(concurrent.as_mut().poll(context).is_ready());
    Poll::Ready(())
  })
  .await;
  router.remove_options(&[AllowConcurrentModules]);

  let mut exclusive = pin!(call_async_modules(
    &modules,
    &scheduling,
    &context,
    HookPhase::PostRoute,
  ));

  std::future::poll_fn(|context| {
    assert!(exclusive.as_mut().poll(context).is_pending());
    Poll::Ready(())
  })
  .await;
  drop(shared);

  exclusive.await;
}

#[cfg_attr(feature = "tokio", tokio::test(flavor = "current_thread"))]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn attached_modules_record_their_type_names() {
  struct Named;

  impl Module for Named {}

  #[async_trait::async_trait]
  impl AsyncModule for Named {}

  let mut router = Router::new();

  router.attach(Named);
  router.attach_async(Named).await;

  let async_name = router.async_modules.lock().await[0].name;

  assert!(router.modules.lock().unwrap()[0].name.ends_with("::Named"));
  assert!(async_name.ends_with("::Named"));
}

#[test]
fn request_uris_reject_forbidden_syntax_before_normalization() {
  for request in [
    "gemini://user@localhost/",
    "gemini://@localhost/",
    "gemini://localhost/#",
    "gemini://localhost/#fragment",
    "gemini://localhost/\t",
    "gemini://localhost/?hello world",
    "gemini://localhost/?%ZZ",
    "gemini://localhost/?%0",
    "gemini://localhost/?%",
    "gemini://localhost/[path]",
    "gemini://localhost/café",
    "gemini:///",
    "/relative",
  ] {
    assert!(super::request::parse_uri(request).is_err(), "{request}");
  }

  for request in [
    "gemini://localhost/",
    "gemini://localhost",
    "gemini://[::1]/",
    "gemini://localhost/?hello%20world",
    "gemini://localhost/caf%C3%A9",
    "gemini://localhost/%23%40%5B",
    "gopher://localhost/1/example",
    "https://localhost/",
    "mailto:user@example.com",
  ] {
    assert!(super::request::parse_uri(request).is_ok(), "{request}");
  }
}

#[test]
fn response_headers_reject_invalid_statuses_and_metadata() {
  for response in [
    Response::new(-1, "message"),
    Response::new(23, "message"),
    Response::new(99, "message"),
    Response::input(""),
    Response::temporary_redirect(""),
    Response::temporary_redirect("/hello world"),
    Response::temporary_redirect("/%XX"),
    Response::temporary_redirect("/[path]"),
    Response::temporary_redirect("/#one#two"),
    Response::bad_request("one\rtwo"),
    Response::bad_request("one\u{0085}two"),
    Response::binary_success([], "text/plain\r\ninjected"),
    Response::binary_success([], "invalid media type"),
  ] {
    assert!(default_status_line(&response).is_err());
  }

  assert_eq!(
    default_status_line(&Response::temporary_failure("")).unwrap(),
    "40"
  );
  assert_eq!(
    default_status_line(&Response::temporary_redirect("../page#part")).unwrap(),
    "30 ../page#part"
  );
  assert_eq!(
    default_status_line(&Response::temporary_redirect("https://example.com/"))
      .unwrap(),
    "30 https://example.com/"
  );
}

#[test]
fn response_languages_support_private_tags_and_omission() {
  let mut response = Response::success("hello");

  response.with_languages(["x-private", "zh-Hans-CN"]);
  assert_eq!(
    default_status_line(&response).unwrap(),
    "20 text/gemini; charset=utf-8; lang=\"x-private,zh-Hans-CN\""
  );
  response.with_languages(Vec::<String>::new());
  assert_eq!(
    default_status_line(&response).unwrap(),
    "20 text/gemini; charset=utf-8"
  );
  response.with_languages(["en;injected=value"]);
  assert!(default_status_line(&response).is_err());
}

#[test]
fn credentials_load_complete_chains_and_use_the_last_setter() {
  use openssl::{
    asn1::Asn1Time,
    ec::{EcGroup, EcKey},
    hash::MessageDigest,
    nid::Nid,
    pkey::PKey,
    x509::{X509NameBuilder, X509},
  };

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

  let certificate = certificate.build();
  let certificate_pem =
    String::from_utf8(certificate.to_pem().unwrap()).unwrap();
  let key_pem =
    String::from_utf8(key.private_key_to_pem_pkcs8().unwrap()).unwrap();
  let chain = certificate_pem.repeat(2);
  let directory = std::env::temp_dir().join(format!(
    "windmark-credentials-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos(),
  ));

  std::fs::create_dir(&directory).unwrap();
  std::fs::write(directory.join("chain.pem"), &chain).unwrap();
  std::fs::write(directory.join("key.pem"), &key_pem).unwrap();

  let mut router = Router::new();

  router.set_certificate("invalid");
  router.set_private_key("invalid");
  router.set_certificate_file(directory.join("chain.pem").to_str().unwrap());
  router.set_private_key_file(directory.join("key.pem").to_str().unwrap());

  let acceptor = router.create_acceptor().unwrap();

  assert_eq!(acceptor.context().extra_chain_certs().len(), 1);
  router.set_certificate_file("missing-certificate.pem");
  router.set_private_key_file("missing-key.pem");
  router.set_certificate(chain);
  router.set_private_key(key_pem);

  let acceptor = router.create_acceptor().unwrap();

  assert_eq!(acceptor.context().extra_chain_certs().len(), 1);
  assert_eq!(
    acceptor.context().certificate().unwrap().to_der().unwrap(),
    certificate.to_der().unwrap(),
  );
  router.set_certificate("");
  assert!(router.create_acceptor().is_err());
  std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn redirects_validate_the_first_path_segment_without_restricting_schemes() {
  for target in ["1:foo", ":foo", "a_b:foo", "1:foo/bar"] {
    assert!(default_status_line(&Response::temporary_redirect(target)).is_err());
  }

  for target in [
    "./1:foo",
    "../page",
    "/1:foo",
    "?query:part",
    "#part",
    "https://example.com/",
    "mailto:user@example.com",
  ] {
    assert!(default_status_line(&Response::temporary_redirect(target)).is_ok());
  }

  assert!(super::request::parse_uri("http:/user@example.org/").is_err());
}

#[test]
fn redirects_validate_authorities_without_normalizing_the_target() {
  for target in [
    "gemini://a@b@c/",
    "//a@b@c/",
    "//user[name]@example.com/",
    "//example.com:port/",
    "//example.com:12:34/",
    "//example[host]/",
    "//[::1]suffix/",
    "//[::1]:port/",
    "//[invalid]/",
    "//[v1.]/",
    "//[v1.a@b]/",
  ] {
    assert!(
      default_status_line(&Response::temporary_redirect(target)).is_err(),
      "{target}"
    );
  }

  for target in [
    "https://user:password@example.com:443/path",
    "//user%40name@example.com/",
    "//[::1]:1965/",
    "//[v1.example:address]/",
    "//example.com:/",
    "//example.com:99999/",
    "file:///path",
    "//example.com/path@part?query@part#fragment@part",
  ] {
    assert_eq!(
      default_status_line(&Response::temporary_redirect(target)).unwrap(),
      format!("30 {target}")
    );
  }
}

#[test]
fn response_metadata_resolves_overrides_and_rejects_invalid_parameters() {
  let mut response = Response::success("body");

  response.with_mime("text/gemini; charset=iso-8859-1; lang=fr; note=\"a b\"");
  response.with_character_set("utf-8");
  response.with_languages(["en", "de"]);

  let metadata = default_status_line(&response)
    .unwrap()
    .strip_prefix("20 ")
    .unwrap()
    .parse::<mime::Mime>()
    .unwrap();

  assert_eq!(
    metadata
      .params()
      .filter(|(name, _)| *name == mime::CHARSET)
      .count(),
    1
  );
  assert_eq!(metadata.get_param("charset").unwrap(), "utf-8");
  assert_eq!(metadata.get_param("lang").unwrap(), "en,de");
  assert_eq!(metadata.get_param("note").unwrap(), "a b");
  response.with_languages(Vec::<String>::new());
  assert!(!default_status_line(&response).unwrap().contains("lang="));

  for media_type in [
    "text/*",
    "*/*",
    "text/gemini; lang=bogus_tag",
    "text/gemini; lang=\"en,\"",
    "text/plain; charset=utf-8; CHARSET=utf-8",
    "text/plain; note=\"café\"",
  ] {
    assert!(
      default_status_line(&Response::binary_success([], media_type)).is_err(),
      "{media_type}"
    );
  }
}

#[test]
fn character_sets_describe_bytes_without_transcoding_them() {
  let mut response = Response::success("café");

  response.with_character_set("iso-8859-1");
  assert!(default_status_line(&response).is_err());
  response.with_character_set("utf-16");
  assert!(default_status_line(&response).is_err());

  let mut response =
    Response::binary_success([0x63, 0x61, 0x66, 0xe9], "text/plain");

  response.with_character_set("iso-8859-1");
  response.with_languages(["fr"]);
  assert_eq!(
    default_status_line(&response).unwrap(),
    "20 text/plain; charset=iso-8859-1; lang=fr"
  );
  assert_eq!(response.serialize_body("", ""), [0x63, 0x61, 0x66, 0xe9]);
}

#[test]
fn percent_equivalent_routes_preserve_original_captures_and_reserved_escapes() {
  let mut router = Router::new();

  router.mount("/caf%C3%A9/~user/{Name}/{*Rest}", |_| {
    Response::success("body")
  });

  for path in [
    "/caf%c3%a9/%7euser/%41lice/Photo%2fone.PNG",
    "/caf%C3%A9/~user/%41lice/Photo%2fone.PNG",
  ] {
    let matched = router.routes.at(path).unwrap();

    assert_eq!(matched.parameters.get("Name"), Some("%41lice"));
    assert_eq!(matched.parameters.get("Rest"), Some("Photo%2fone.PNG"));
  }

  router.add_options(&[AllowCaseInsensitiveLookup]);

  let matched = router
    .routes
    .at("/CAF%c3%a9/%7EUSER/%41LiCe/Ä%2fOne")
    .unwrap();

  assert_eq!(matched.parameters.get("Name"), Some("%41LiCe"));
  assert_eq!(matched.parameters.get("Rest"), Some("Ä%2fOne"));
  router.remove_options(&[AllowCaseInsensitiveLookup]);
  assert!(router.routes.at("/CAF%C3%A9/~USER/Alice/file").is_err());
}

#[test]
fn equivalent_route_collisions_leave_registered_handlers_available() {
  let mut router = Router::new();

  router.mount("/%7euser", |_| Response::success("original"));

  let collision =
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      router.mount("/~user", |_| Response::success("replacement"));
    }));

  assert!(collision.is_err());
  assert!(router.routes.contains("/%7Euser"));
  router.mount("/encoded%2Fslash", |_| Response::success("encoded"));
  assert!(router.routes.contains("/encoded%2fslash"));
  assert!(!router.routes.contains("/encoded/slash"));
}
