//! `cargo run --example default_logger`

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  pretty_env_logger::formatted_builder()
    .parse_filters("windmark=trace")
    .try_init()?;

  let mut router = windmark::router::Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  router.mount("/", |_| {
    log::info!("Hello!");
    Response::success("Hello!")
  });

  router.run().await
}
