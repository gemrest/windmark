//! `cargo run --example partial`

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .add_header(|_| "This is fancy art.\n".to_string())
    .add_footer(|context: windmark::context::RouteContext| {
      format!("\nYou came from '{}'.", context.url.path())
    })
    .add_footer(|_| "\nCopyright (C) 2022".to_string())
    .mount("/", windmark::success!("Hello!"))
    .run()
    .await
}
