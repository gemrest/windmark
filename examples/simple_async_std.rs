//! `cargo run --example simple_async_std --features async-std`

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/", |_| {
      windmark::response::Response::success("Hello, async-std!")
    })
    .run()
    .await
}
