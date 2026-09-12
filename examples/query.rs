//! `cargo run --example query`

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/query", |context: windmark::context::RouteContext| {
      Response::success(format!(
        "You provided the following queries: '{:?}'",
        windmark::utilities::queries_from_url(&context.url)
      ))
    })
    .run()
    .await
}
