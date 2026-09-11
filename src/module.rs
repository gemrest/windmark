use crate::context::HookContext;

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
