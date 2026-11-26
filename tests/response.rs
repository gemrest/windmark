use windmark::response::Response;

const NON_UTF8: &[u8] =
  b"\x89PNG\r\n\x1a\n\x80\x81\xFE\xFF\xC0\xC1\xF5\x00\x7F";

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

#[test]
fn binary_success_preserves_non_utf8_bytes() {
  let response = Response::binary_success(NON_UTF8, "image/png");

  assert_eq!(response.status(), 21);
  assert_eq!(response.mime(), Some("image/png"));
  assert_eq!(response.binary_content(), Some(NON_UTF8));
}

#[test]
fn serialize_body_writes_binary_bytes_verbatim() {
  let response = Response::binary_success(NON_UTF8, "image/png");
  let body = response.serialize_body("ignored-header", "ignored-footer");

  assert_eq!(body, NON_UTF8);
}

#[cfg(feature = "auto-deduce-mime")]
#[test]
fn binary_success_auto_preserves_non_utf8_bytes() {
  let response = Response::binary_success_auto(NON_UTF8);

  assert_eq!(response.status(), 22);
  assert_eq!(response.binary_content(), Some(NON_UTF8));
  assert_eq!(
    response.serialize_body("ignored-header", "ignored-footer"),
    NON_UTF8,
  );
}

#[test]
fn serialize_body_text_success_wraps_with_header_and_footer() {
  let response = Response::success("body");
  let body = response.serialize_body("HEADER\n", "FOOTER");

  assert_eq!(body, b"HEADER\nbody\nFOOTER\n");
}

#[test]
fn public_binary_serialization_returns_the_existing_allocation() {
  for status in [21, 22] {
    let mut response = Response::new(status, "ignored");
    let mut bytes = Vec::with_capacity(1024);

    bytes.extend_from_slice(NON_UTF8);

    let pointer = bytes.as_ptr();
    let capacity = bytes.capacity();

    *response.binary_content_mut().unwrap() = bytes;

    let body = response.serialize_body("ignored-header", "ignored-footer");

    assert_eq!(body.as_ptr(), pointer);
    assert_eq!(body.capacity(), capacity);
    assert_eq!(body, NON_UTF8);
  }
}

#[test]
fn text_serialization_normalizes_line_endings_and_terminates_the_footer() {
  let response = Response::success("one\r\ntwo\rthree");

  assert_eq!(
    response.serialize_body("header\r\n", "footer"),
    b"header\none\ntwo\nthree\nfooter\n"
  );
  assert_eq!(
    Response::binary_success(b"one\r\ntwo\r", "text/plain")
      .serialize_body("", ""),
    b"one\r\ntwo\r"
  );
}
