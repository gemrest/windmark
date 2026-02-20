//! `cargo run --example stateless_module`

use windmark::{response::Response, router::Router};

fn smiley(_context: windmark::context::RouteContext) -> Response {
  Response::success("😀")
}

fn emojis(router: &mut Router) { router.mount("/smiley", smiley); }

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .attach_stateless(emojis)
    .run()
    .await
}
