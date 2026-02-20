//! `cargo run --example default_logger --features logger,response-macros`

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut router = windmark::router::Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  #[cfg(feature = "logger")]
  {
    router.enable_default_logger(true);
  }
  router.mount(
    "/",
    windmark::success!({
      log::info!("Hello!");

      "Hello!"
    }),
  );

  router.run().await
}
