//! `cargo run --example async_stateful_module --features response-macros`

use windmark::{context::HookContext, router::Router};

#[derive(Default)]
struct Clicker {
  clicks: std::sync::Arc<std::sync::Mutex<usize>>,
}

#[async_trait::async_trait]
impl windmark::module::AsyncModule for Clicker {
  async fn on_attach(&mut self, _router: &mut Router) {
    println!("module 'clicker' has been attached!");
  }

  async fn on_pre_route(&mut self, context: &HookContext) {
    *self.clicks.lock().unwrap() += 1;

    println!(
      "module 'clicker' has been called before the route '{}' with {} clicks!",
      context.url.path(),
      self.clicks.lock().unwrap()
    );
  }

  async fn on_post_route(&mut self, context: &HookContext) {
    println!(
      "module 'clicker' clicker has been called after the route '{}' with {} \
       clicks!",
      context.url.path(),
      self.clicks.lock().unwrap()
    );
  }
}

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut router = Router::new();

  router.set_private_key_file("windmark_private.pem");
  router.set_certificate_file("windmark_public.pem");
  #[cfg(feature = "logger")]
  {
    router.enable_default_logger(true);
  }
  router.attach_async(Clicker::default());
  router.mount("/", windmark::success!("Hello!"));

  router.run().await
}
