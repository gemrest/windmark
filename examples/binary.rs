//! `cargo run --example binary --features response-macros`
//!
//! Optionally, you can run this example with the `auto-deduce-mime` feature
//! enabled.

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut router = windmark::router::Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  #[cfg(feature = "auto-deduce-mime")]
  router.mount("/automatic", {
    windmark::binary_success!(include_bytes!("../LICENSE-MIT"))
  });
  router.mount("/specific", {
    windmark::binary_success!(include_bytes!("../LICENSE-MIT"), "text/plain")
  });
  router.mount("/direct", {
    windmark::binary_success!("This is a string.", "text/plain")
  });

  router.run().await
}
