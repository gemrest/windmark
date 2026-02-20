//! `cargo run --example certificate --features response-macros`

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/secret", |context: windmark::context::RouteContext| {
      if let Some(certificate) = context.certificate {
        Response::success(format!("Your public key is '{}'.", {
          (|| -> Result<String, openssl::error::ErrorStack> {
            Ok(format!(
              "{:?}",
              certificate.public_key()?.rsa()?.public_key_to_pem()?
            ))
          })()
          .unwrap_or_else(|_| {
            "An error occurred while reading your public key.".to_string()
          })
        }))
      } else {
        Response::client_certificate_required(
          "This is a secret route ... Identify yourself!",
        )
      }
    })
    .mount(
      "/invalid",
      windmark::certificate_not_valid!("Your certificate is invalid."),
    )
    .run()
    .await
}
