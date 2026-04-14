//! `cargo run --example callbacks`

use windmark::context::HookContext;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/", windmark::success!("Hello!"))
    .set_pre_route_callback(|context: &HookContext| {
      println!(
        "accepted connection from {} to {}",
        context.peer_address.unwrap().ip(),
        context.url.to_string()
      )
    })
    .set_post_route_callback(
      |context: &HookContext, content: &mut windmark::response::Response| {
        content.content = content.content.replace("Hello", "Hi");

        println!(
          "closed connection from {}",
          context.peer_address.unwrap().ip()
        )
      },
    )
    .run()
    .await
}
