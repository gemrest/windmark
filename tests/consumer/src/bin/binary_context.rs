fn main() {
  let body = b"hello";
  let mime = "text/plain";
  let _ = windmark::binary_success!(body, mime);
  let _ =
    windmark::binary_success!(context, context.url.path().as_bytes(), mime);
  let _ = windmark::binary_success!(context => context.url.path().as_bytes());
}
