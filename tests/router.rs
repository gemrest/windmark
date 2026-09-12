use super::{resolve_lookup_path, status_line};
use crate::{response::Response, router_option::RouterOption};
use matchit::Router as MatchRouter;
use std::collections::HashSet;

/// Resolve `request_path` against a router holding `routes`, under `options`.
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
    let lookup = if options.contains(&RouterOption::AllowCaseInsensitiveLookup)
    {
      candidate.to_ascii_lowercase()
    } else {
      candidate.to_owned()
    };

    matcher.at(&lookup).is_ok()
  })
}

#[test]
fn exact_match_is_returned_unchanged() {
  assert_eq!(resolve(&[], "/foo", &["/foo"]), "/foo");
}

#[test]
fn unmatched_path_without_options_is_returned_unchanged() {
  assert_eq!(resolve(&[], "/missing", &["/foo"]), "/missing");
}

#[test]
fn dynamic_route_is_matched_without_modification() {
  assert_eq!(resolve(&[], "/posts/42", &["/posts/:id"]), "/posts/42");
}

#[test]
fn case_insensitive_lookup_preserves_the_path() {
  assert_eq!(
    resolve(
      &[RouterOption::AllowCaseInsensitiveLookup],
      "/FoO",
      &["/foo"]
    ),
    "/FoO",
  );
}

#[test]
fn extra_trailing_slash_is_removed_when_unslashed_route_exists() {
  assert_eq!(
    resolve(
      &[RouterOption::RemoveExtraTrailingSlash],
      "/foo/",
      &["/foo"]
    ),
    "/foo",
  );
}

#[test]
fn root_path_is_left_untouched_by_trailing_slash_removal() {
  assert_eq!(
    resolve(&[RouterOption::RemoveExtraTrailingSlash], "/", &["/"]),
    "/",
  );
}

#[test]
fn missing_trailing_slash_is_added_when_slashed_route_exists() {
  assert_eq!(
    resolve(&[RouterOption::AddMissingTrailingSlash], "/foo", &["/foo/"]),
    "/foo/",
  );
}

#[test]
fn trailing_slash_fix_is_skipped_when_the_target_route_is_absent() {
  assert_eq!(
    resolve(
      &[RouterOption::RemoveExtraTrailingSlash],
      "/foo/",
      &["/bar"]
    ),
    "/foo/",
  );
}

#[test]
fn exact_match_takes_precedence_over_trailing_slash_fix() {
  assert_eq!(
    resolve(
      &[RouterOption::RemoveExtraTrailingSlash],
      "/foo/",
      &["/foo", "/foo/"],
    ),
    "/foo/",
  );
}

#[test]
fn case_insensitive_lookup_combines_with_trailing_slash_removal() {
  assert_eq!(
    resolve(
      &[
        RouterOption::AllowCaseInsensitiveLookup,
        RouterOption::RemoveExtraTrailingSlash,
      ],
      "/Foo/",
      &["/foo"],
    ),
    "/Foo",
  );
}

/// Format `response`'s status line with the router's default charset and
/// language (matching `Router::default`).
fn line(response: &Response) -> String { status_line(response, "utf-8", "en") }

#[test]
fn success_status_line_falls_back_to_router_defaults() {
  assert_eq!(
    line(&Response::success("hi")),
    "20 text/gemini; charset=utf-8; lang=en",
  );
}

#[test]
fn success_status_line_uses_the_response_mime() {
  let mut response = Response::success("hi");

  response.with_mime("text/plain");
  assert_eq!(line(&response), "20 text/plain; charset=utf-8; lang=en");
}

#[test]
fn success_status_line_uses_the_response_character_set() {
  let mut response = Response::success("hi");

  response.with_character_set("iso-8859-1");
  assert_eq!(
    line(&response),
    "20 text/gemini; charset=iso-8859-1; lang=en"
  );
}

#[test]
fn success_status_line_joins_the_response_languages() {
  let mut response = Response::success("hi");

  response.with_languages(["en", "fr"]);
  assert_eq!(line(&response), "20 text/gemini; charset=utf-8; lang=en,fr");
}

#[test]
fn binary_success_is_sent_as_a_plain_success_with_its_mime() {
  assert_eq!(
    line(&Response::binary_success([0xFFu8], "image/png")),
    "20 image/png"
  );
}

#[test]
fn status_21_without_a_mime_emits_an_empty_meta() {
  assert_eq!(line(&Response::new(21, "")), "20 ");
}

#[test]
fn redirect_status_line_uses_its_content_as_the_meta() {
  assert_eq!(
    line(&Response::temporary_redirect("/elsewhere")),
    "30 /elsewhere"
  );
}

#[test]
fn failure_status_line_uses_only_the_first_line_of_content() {
  assert_eq!(line(&Response::new(40, "down\nfor maintenance")), "40 down");
}

#[test]
fn input_status_line_uses_its_prompt_as_the_meta() {
  assert_eq!(line(&Response::input("Your name?")), "10 Your name?");
}

#[test]
fn footer_rendering_preserves_empty_values_and_callback_order() {
  let context = crate::context::RouteContext {
    peer_address: None,
    url:          "gemini://localhost/footer".parse().unwrap(),
    parameters:   crate::context::Parameters::default(),
    certificate:  None,
  };

  for values in [vec![], vec![""], vec!["", ""], vec!["a", "", "b\n"]] {
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let partials: Vec<std::sync::Arc<dyn crate::handler::Partial>> = values
      .iter()
      .enumerate()
      .map(|(index, value)| {
        let calls = calls.clone();
        let value = (*value).to_owned();

        std::sync::Arc::new(move |context: &crate::context::RouteContext| {
          assert_eq!(context.url.path(), "/footer");
          calls.lock().unwrap().push(index);
          value.clone()
        }) as std::sync::Arc<dyn crate::handler::Partial>
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
  let options = [
    RouterOption::AllowCaseInsensitiveLookup,
    RouterOption::AddMissingTrailingSlash,
  ]
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
    (21, "20 ", b"\x00\xff".as_slice()),
    (22, "20 ", b"\x00\xff".as_slice()),
    (23, "23 body", b"".as_slice()),
    (40, "40 body", b"".as_slice()),
    (79, "79 body", b"".as_slice()),
    (-1, "-1 body", b"".as_slice()),
  ] {
    let mut response = Response::new(status, "body");

    response.binary_content = Some(vec![0, 0xff]);

    let status_line = line(&response);
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
      super::serialize_response(response, "20 ", "HEADER", "FOOTER"),
      b"20 \r\n"
    );
  }
}

#[test]
fn case_insensitive_routes_preserve_parameter_names_and_values() {
  let mut router = super::Router::new();

  router.mount("/Users/:Name/Files/*FilePath", |_| {
    Response::success("file")
  });
  router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);

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
  let mut router = super::Router::new();

  router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  router.mount("/Users/:Name", |_| Response::success("user"));
  router.mount("/Users/Me", |_| Response::success("profile"));
  router.mount("/Files/Prefix:Name", |_| Response::success("file"));

  let profile = router.routes.at("/users/ME").unwrap();
  let file = router.routes.at("/FILES/prefixREPORT.PDF").unwrap();

  assert!(profile.parameters.is_empty());
  assert_eq!(file.parameters.get("Name"), Some("REPORT.PDF"));
}

#[test]
fn case_insensitive_routes_restore_repeated_values_from_their_own_positions() {
  let mut router = super::Router::new();

  router.mount("/Same/:First/:Second", |_| Response::success("pair"));
  router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);

  let matched = router.routes.at("/same/SAME/SaMe").unwrap();

  assert_eq!(matched.parameters.get("First"), Some("SAME"));
  assert_eq!(matched.parameters.get("Second"), Some("SaMe"));
}

#[test]
fn case_insensitive_matching_can_be_disabled_reenabled_and_cloned() {
  let mut router = super::Router::new();

  router.mount("/Mixed/:Name", |_| Response::success("mixed"));
  assert!(!router.routes.contains("/mixed/Alice"));
  router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  assert!(router.routes.contains("/mixed/Alice"));

  let cloned = router.clone();

  router.toggle_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  assert!(!router.routes.contains("/mixed/Alice"));
  assert!(cloned.routes.contains("/mixed/Alice"));
  router.toggle_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  assert!(router.routes.contains("/mixed/Alice"));
  router.remove_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  assert!(!router.routes.contains("/mixed/Alice"));
  assert!(router.routes.contains("/Mixed/Alice"));
}

#[test]
fn enabling_case_insensitive_matching_rejects_existing_conflicts() {
  let mut router = super::Router::new();

  router.mount("/Foo", |_| Response::success("upper"));
  router.mount("/foo", |_| Response::success("lower"));

  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  }));

  assert!(result.is_err());
  assert!(!router
    .options
    .contains(&RouterOption::AllowCaseInsensitiveLookup));
  assert!(router.routes.contains("/Foo"));
  assert!(router.routes.contains("/foo"));
  assert!(!router.routes.contains("/FOO"));
}

#[test]
fn mounting_case_conflicts_preserves_existing_routes() {
  let mut router = super::Router::new();

  router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  router.mount("/Users/:Name", |_| Response::success("user"));

  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    router.mount("/users/:Other", |_| Response::success("conflict"));
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
  let mut router = super::Router::new();

  router.mount("/Users/:Name/", |_| Response::success("user"));
  router.add_options(&[
    RouterOption::AllowCaseInsensitiveLookup,
    RouterOption::AddMissingTrailingSlash,
  ]);

  let path =
    resolve_lookup_path(&router.options, "/uSeRs/Alice", |candidate| {
      router.routes.contains(candidate)
    });
  let matched = router.routes.at(&path).unwrap();

  assert_eq!(path, "/uSeRs/Alice/");
  assert_eq!(matched.parameters.get("Name"), Some("Alice"));
}

#[test]
fn synchronous_module_locks_allow_other_modules_to_progress() {
  struct SignalingModule(std::sync::mpsc::Sender<()>);

  impl crate::module::Module for SignalingModule {
    fn on_pre_route(&mut self, _: &crate::context::HookContext) {
      self.0.send(()).unwrap();
    }
  }

  struct BlockingModule {
    entered: std::sync::mpsc::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
    blocked: bool,
  }

  impl crate::module::Module for BlockingModule {
    fn on_pre_route(&mut self, _: &crate::context::HookContext) {
      if !self.blocked {
        self.blocked = true;

        self.entered.send(()).unwrap();
        self
          .release
          .recv_timeout(std::time::Duration::from_secs(5))
          .unwrap();
      }
    }
  }

  let (signal, progress) = std::sync::mpsc::channel();
  let (entered, entry) = std::sync::mpsc::channel();
  let (release, resume) = std::sync::mpsc::channel();
  let modules: std::sync::Arc<std::sync::Mutex<Vec<super::SharedModule>>> =
    std::sync::Arc::new(std::sync::Mutex::new(vec![
      std::sync::Arc::new(std::sync::Mutex::new(Box::new(SignalingModule(
        signal,
      )))),
      std::sync::Arc::new(std::sync::Mutex::new(Box::new(BlockingModule {
        entered,
        release: resume,
        blocked: false,
      }))),
    ]));
  let scheduling = std::sync::Arc::new(super::ModuleScheduling::default());
  let first_scheduling = scheduling.clone();
  let first_modules = modules.clone();

  scheduling
    .concurrent
    .store(true, std::sync::atomic::Ordering::Relaxed);

  let first = std::thread::spawn(move || {
    super::call_modules(&first_modules, &first_scheduling, |module| {
      module.on_pre_route(&crate::context::HookContext {
        peer_address: None,
        url:          url::Url::parse("gemini://localhost/").unwrap(),
        parameters:   None,
        certificate:  None,
      });
    });
  });

  progress
    .recv_timeout(std::time::Duration::from_secs(5))
    .unwrap();
  entry
    .recv_timeout(std::time::Duration::from_secs(5))
    .unwrap();

  let second = std::thread::spawn(move || {
    super::call_modules(&modules, &scheduling, |module| {
      module.on_pre_route(&crate::context::HookContext {
        peer_address: None,
        url:          url::Url::parse("gemini://localhost/").unwrap(),
        parameters:   None,
        certificate:  None,
      });
    });
  });
  let advanced = progress.recv_timeout(std::time::Duration::from_secs(5));

  release.send(()).unwrap();
  first.join().unwrap();
  second.join().unwrap();
  advanced.expect("the first module was blocked by the second module");
}

#[test]
fn module_scheduling_options_are_shared_across_router_clones() {
  let mut router = super::Router::new();
  let mut cloned = router.clone();
  let enabled = || {
    router
      .module_scheduling
      .concurrent
      .load(std::sync::atomic::Ordering::Relaxed)
  };

  assert!(!enabled());
  cloned
    .add_options(&[crate::router_option::RouterOption::AllowConcurrentModules]);
  assert!(enabled());
  cloned.toggle_options(&[
    crate::router_option::RouterOption::AllowConcurrentModules,
  ]);
  assert!(!enabled());
  cloned.toggle_options(&[
    crate::router_option::RouterOption::AllowConcurrentModules,
  ]);
  assert!(enabled());
  router.remove_options(&[
    crate::router_option::RouterOption::AllowConcurrentModules,
  ]);
  assert!(!cloned
    .module_scheduling
    .concurrent
    .load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn synchronous_hook_phases_take_exclusive_or_shared_permits() {
  let mut router = super::Router::new();

  router.attach(SchedulingProbe);
  super::call_modules(&router.modules, &router.module_scheduling, |_| {
    assert!(router.module_scheduling.synchronous.try_read().is_err());
  });
  router
    .add_options(&[crate::router_option::RouterOption::AllowConcurrentModules]);
  super::call_modules(&router.modules, &router.module_scheduling, |_| {
    assert!(router.module_scheduling.synchronous.try_read().is_ok());
    assert!(router.module_scheduling.synchronous.try_write().is_err());
  });
}

struct SchedulingProbe;

impl crate::module::Module for SchedulingProbe {}

#[async_trait::async_trait]
impl crate::module::AsyncModule for SchedulingProbe {}

#[cfg_attr(feature = "tokio", tokio::test(flavor = "current_thread"))]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn exclusive_async_phases_wait_for_existing_concurrent_phases() {
  use std::{future::Future, pin::pin, task::Poll};

  let mut router = super::Router::new();
  let context = crate::context::HookContext {
    peer_address: None,
    url:          url::Url::parse("gemini://localhost/").unwrap(),
    parameters:   None,
    certificate:  None,
  };

  router.attach_async(SchedulingProbe).await;

  for phase in [super::HookPhase::PreRoute, super::HookPhase::PostRoute] {
    let shared = router.module_scheduling.asynchronous.read().await;
    let mut dispatch = pin!(super::call_async_modules(
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

  router
    .add_options(&[crate::router_option::RouterOption::AllowConcurrentModules]);

  let scheduling = router.module_scheduling.clone();
  let modules = router.async_modules.clone();
  let shared = scheduling.asynchronous.read().await;
  let mut concurrent = pin!(super::call_async_modules(
    &modules,
    &scheduling,
    &context,
    super::HookPhase::PreRoute,
  ));

  std::future::poll_fn(|context| {
    assert!(concurrent.as_mut().poll(context).is_ready());
    Poll::Ready(())
  })
  .await;
  router.remove_options(&[
    crate::router_option::RouterOption::AllowConcurrentModules,
  ]);

  let mut exclusive = pin!(super::call_async_modules(
    &modules,
    &scheduling,
    &context,
    super::HookPhase::PostRoute,
  ));

  std::future::poll_fn(|context| {
    assert!(exclusive.as_mut().poll(context).is_pending());
    Poll::Ready(())
  })
  .await;
  drop(shared);

  exclusive.await;
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

  let mut router = super::Router::new();

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
