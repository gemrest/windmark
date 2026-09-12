//! `cargo run --example callbacks`

use windmark::{context::HookContext, response::Response};

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/", |_| Response::success("Hello!"))
    .set_pre_route_callback(|context: &HookContext| {
      println!(
        "accepted connection from {} to {}",
        context.peer_address.unwrap().ip(),
        context.url
      )
    })
    .set_post_route_callback(|context: &HookContext, content: &mut Response| {
      if let Some(text) = content.content_mut() {
        *text = text.replace("Hello", "Hi");
      }

      println!(
        "prepared response for {}",
        context.peer_address.unwrap().ip()
      )
    })
    .run()
    .await
}
