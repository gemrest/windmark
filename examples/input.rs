//! `cargo run --example input`

use windmark::{context::RouteContext, response::Response};

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/input", |context: RouteContext| {
      if let Some(name) = context.url.query() {
        Response::success(format!("Your name is {}!", name))
      } else {
        Response::input("What is your name?")
      }
    })
    .mount("/sensitive", |context: RouteContext| {
      if let Some(password) = context.url.query() {
        Response::success(format!("Your password is {}!", password))
      } else {
        Response::sensitive_input("What is your password?")
      }
    })
    .run()
    .await
}
