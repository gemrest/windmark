use crate::context::HookContext;

/// Each module runs one hook at a time. Different modules may run concurrently
/// across requests, while each request visits modules in registration order.
///
/// If a hook panics, subsequent requests skip that module because its lock is
/// poisoned; other modules remain available.
pub trait Module {
  /// The router calls this hook during attachment, before registering
  /// request hooks.
  fn on_attach(&mut self, _: &mut crate::router::Router) {}

  /// The router calls this hook before dispatching a request, even if the
  /// request does not match a route.
  fn on_pre_route(&mut self, _: &HookContext) {}

  /// The router calls this hook after the handler completes, before
  /// response serialisation.
  fn on_post_route(&mut self, _: &HookContext) {}
}

/// Each module runs one hook at a time, including across awaits. Different
/// modules may run concurrently across requests, while each request visits
/// modules in registration order.
#[async_trait::async_trait]
pub trait AsyncModule: Send + Sync {
  /// The router calls this hook during attachment, before registering
  /// request hooks.
  async fn on_attach(&mut self, _: &mut crate::router::Router) {}

  /// The router calls this hook before dispatching a request, even if the
  /// request does not match a route.
  async fn on_pre_route(&mut self, _: &HookContext) {}

  /// The router calls this hook after the handler completes, before
  /// response serialisation.
  async fn on_post_route(&mut self, _: &HookContext) {}
}
