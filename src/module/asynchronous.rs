use crate::context::HookContext;

#[async_trait::async_trait]
pub trait AsyncModule: Send + Sync {
  /// Called right after the module is attached.
  async fn on_attach(&mut self, _: &mut crate::router::Router) {}

  /// Called before a route is mounted.
  async fn on_pre_route(&mut self, _: HookContext) {}

  /// Called after a route is mounted.
  async fn on_post_route(&mut self, _: HookContext) {}
}
