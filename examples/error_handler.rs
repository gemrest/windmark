//! `cargo run --example error_handler`

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut error_count = 0;

  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .set_error_handler(move |_| {
      error_count += 1;

      println!("{} errors so far", error_count);

      Response::permanent_failure("e")
    })
    .mount("/error", |_| {
      let nothing = None::<String>;

      Response::success(nothing.unwrap())
    })
    .run()
    .await
}
