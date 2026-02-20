//! `cargo run --example responses --features response-macros`

use windmark::success;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount(
      "/",
      success!(
        "# Index\n\nWelcome!\n\n=> /test Test Page\n=> /time Unix Epoch"
      ),
    )
    .mount("/test", success!("This is a test page.\n=> / back"))
    .mount(
      "/failure",
      windmark::temporary_failure!("Woops ... temporarily."),
    )
    .mount(
      "/time",
      success!(std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos()),
    )
    .mount(
      "/redirect",
      windmark::permanent_redirect!("gemini://localhost/test"),
    )
    .run()
    .await
}
