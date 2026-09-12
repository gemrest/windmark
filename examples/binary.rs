//! `cargo run --example binary`
//!
//! Optionally, you can run this example with the `auto-deduce-mime` feature
//! enabled.

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut router = windmark::router::Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  #[cfg(feature = "auto-deduce-mime")]
  router.mount("/automatic", |_| {
    Response::binary_success_auto(include_bytes!("../LICENSE-MIT"))
  });
  router.mount("/specific", |_| {
    Response::binary_success(include_bytes!("../LICENSE-MIT"), "text/plain")
  });
  router.mount("/direct", |_| {
    Response::binary_success("This is a string.", "text/plain")
  });

  router.run().await
}
