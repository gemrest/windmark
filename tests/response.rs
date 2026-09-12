use windmark::response::Response;

#[test]
fn payload_access_is_exclusive_and_statuses_remain_numeric() {
  for status in [10, 20, 40, 51, 79] {
    let mut response = Response::new(status, "message");

    assert_eq!(response.status(), status);
    assert!(response.binary_content().is_none());
    assert!(response.binary_content_mut().is_none());
    response.content_mut().unwrap().push('!');
    assert_eq!(response.content(), Some("message!"));
  }

  for status in [21, 22] {
    let mut response = Response::new(status, "ignored");

    assert_eq!(response.status(), status);
    assert!(response.content().is_none());
    assert!(response.content_mut().is_none());
    response.binary_content_mut().unwrap().extend([0xff, 0]);
    assert_eq!(response.binary_content(), Some([0xff, 0].as_slice()));
  }
}

#[test]
fn metadata_overrides_can_be_edited_and_cleared() {
  let mut response = Response::success("body");

  response.with_mime("text/plain");
  response.with_character_set("ascii");
  response.with_languages(["en", "fr"]);
  assert_eq!(response.mime(), Some("text/plain"));
  assert_eq!(response.character_set(), Some("ascii"));
  assert_eq!(response.languages().unwrap(), ["en", "fr"]);
  response.languages_mut().as_mut().unwrap().clear();
  assert_eq!(response.languages(), Some([].as_slice()));

  *response.mime_mut() = None;
  *response.character_set_mut() = None;
  *response.languages_mut() = None;

  assert!(response.mime().is_none());
  assert!(response.character_set().is_none());
  assert!(response.languages().is_none());
}

#[test]
fn replacing_a_response_discards_the_old_payload_and_overrides() {
  let mut response = Response::binary_success([0xff], "image/png");

  assert!(response.binary_content().is_some());

  response = Response::not_found("Missing");

  assert_eq!(response.status(), 51);
  assert_eq!(response.content(), Some("Missing"));
  assert!(response.binary_content().is_none());
  assert!(response.mime().is_none());
}
