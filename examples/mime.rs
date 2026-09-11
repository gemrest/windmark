//! `cargo run --example mime`

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/mime", |_| {
      let mut response = windmark::response::Response::success("Hello!");

      response.with_mime("text/plain");

      response
    })
    .run()
    .await
}
