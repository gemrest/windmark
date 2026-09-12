//! `cargo run --example parameters`

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount(
      "/language/:language",
      |context: windmark::context::RouteContext| {
        Response::success(format!(
          "Your language of choice is {}.",
          context.parameters.get("language").unwrap()
        ))
      },
    )
    .mount(
      "/name/:first/:last",
      |context: windmark::context::RouteContext| {
        Response::success(format!(
          "Your name is {} {}.",
          context.parameters.get("first").unwrap(),
          context.parameters.get("last").unwrap()
        ))
      },
    )
    .run()
    .await
}
