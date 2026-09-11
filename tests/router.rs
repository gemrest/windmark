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
    matcher.at(candidate).is_ok()
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
fn case_insensitive_lookup_lowercases_the_path() {
  assert_eq!(
    resolve(
      &[RouterOption::AllowCaseInsensitiveLookup],
      "/FoO",
      &["/foo"]
    ),
    "/foo",
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
    "/foo",
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
    let partials: Vec<Box<dyn crate::handler::Partial>> = values
      .iter()
      .enumerate()
      .map(|(index, value)| {
        let calls = calls.clone();
        let value = (*value).to_owned();

        Box::new(move |context: &crate::context::RouteContext| {
          assert_eq!(context.url.path(), "/footer");
          calls.lock().unwrap().push(index);
          value.clone()
        }) as Box<dyn crate::handler::Partial>
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
fn case_folding_preserves_existing_unicode_and_probe_order() {
  let options = [
    RouterOption::AllowCaseInsensitiveLookup,
    RouterOption::AddMissingTrailingSlash,
  ]
  .into_iter()
  .collect();
  let probes = std::cell::RefCell::new(Vec::new());
  let path = resolve_lookup_path(&options, "/Users/ÄLICE", |candidate| {
    probes.borrow_mut().push(candidate.to_owned());

    candidate == "/users/älice/"
  });

  assert_eq!(path, "/users/älice/");
  assert_eq!(*probes.borrow(), ["/users/älice", "/users/älice/"]);
}
