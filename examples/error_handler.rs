//! `cargo run --example error_handler`

use std::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc,
};

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let error_count = Arc::new(AtomicUsize::new(0));

  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .set_error_handler(move |_| {
      let count = error_count.fetch_add(1, Ordering::Relaxed) + 1;

      println!("{count} errors so far");

      Response::permanent_failure("e")
    })
    .mount("/error", |_| {
      let nothing = None::<String>;

      Response::success(nothing.unwrap())
    })
    .run()
    .await
}
