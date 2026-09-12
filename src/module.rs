use crate::context::HookContext;

/// Synchronous modules run hooks in registration order for each request.
///
/// Each module remains exclusive. Hook phases run exclusively across requests
/// unless `RouterOption::AllowConcurrentModules` is enabled.
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

/// Asynchronous modules run hooks in registration order for each request.
///
/// Each module remains exclusive, including across awaits. Hook phases run
/// exclusively across requests unless `RouterOption::AllowConcurrentModules`
/// is enabled.
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
