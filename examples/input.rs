//! `cargo run --example input`

use windmark::{context::RouteContext, response::Response};

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/input", |context: RouteContext| {
      if let Some(name) = context.url.query() {
        match percent_encoding::percent_decode_str(name).decode_utf8() {
          Ok(name) => Response::success(format!("Your name is {name}!")),
          Err(_) => Response::bad_request("Input must be valid UTF-8."),
        }
      } else {
        Response::input("What is your name?")
      }
    })
    .mount("/sensitive", |context: RouteContext| {
      if context.url.query().is_some() {
        Response::success("Your input was received.")
      } else {
        Response::sensitive_input("What is your password?")
      }
    })
    .run()
    .await
}
