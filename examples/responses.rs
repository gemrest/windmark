//! `cargo run --example responses`

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/", |_| {
      Response::success(
        "# Index\n\nWelcome!\n\n=> /test Test Page\n=> /time Unix Epoch",
      )
    })
    .mount("/test", |_| {
      Response::success("This is a test page.\n=> / back")
    })
    .mount("/failure", |_| {
      Response::temporary_failure("Woops ... temporarily.")
    })
    .mount("/time", |_| {
      Response::success(std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos())
    })
    .mount("/redirect", |_| {
      Response::permanent_redirect("gemini://localhost/test")
    })
    .run()
    .await
}
