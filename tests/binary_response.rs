use windmark::response::Response;

// PNG magic + bytes invalid as UTF-8 (lone continuation bytes, lone 4-byte
// lead, reserved C0/C1 leads, DEL).
const NON_UTF8: &[u8] = &[
  0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x80, 0x81, 0xFE, 0xFF, 0xC0,
  0xC1, 0xF5, 0x00, 0x7F,
];

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
