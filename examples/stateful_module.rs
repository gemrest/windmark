//! `cargo run --example stateful_module`

use std::sync::atomic::{AtomicUsize, Ordering};
use windmark::{context::HookContext, response::Response, router::Router};

#[derive(Default)]
struct Clicker {
  clicks: AtomicUsize,
}

impl windmark::module::Module for Clicker {
  fn on_attach(&mut self, _router: &mut Router) {
    println!("module 'clicker' has been attached!");
  }

  fn on_pre_route(&self, context: &HookContext) {
    let clicks = self.clicks.fetch_add(1, Ordering::Relaxed) + 1;

    println!(
      "module 'clicker' has been called before the route '{}' with {} clicks!",
      context.url.path(),
      clicks,
    );
  }

  fn on_post_route(&self, context: &HookContext) {
    println!(
      "module 'clicker' clicker has been called after the route '{}' with {} \
       clicks!",
      context.url.path(),
      self.clicks.load(Ordering::Relaxed),
    );
  }
}

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  pretty_env_logger::formatted_builder()
    .parse_filters("windmark=trace")
    .try_init()?;

  let mut router = Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  router.attach(Clicker::default());
  router.mount("/", |_| Response::success("Hello!"));

  router.run().await
}
