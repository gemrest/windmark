use crate::context::HookContext;

/// Synchronous modules run hooks in registration order for each request.
///
/// Hook phases run exclusively across requests unless
/// `RouterOption::AllowConcurrentModules` is enabled. Concurrent phases may
/// invoke the same module simultaneously; modules own any synchronisation
/// needed by their state. Attachment still receives exclusive access.
/// If a hook panics, subsequent invocations skip that module. Invocations
/// already running may finish; other modules remain available.
pub trait Module: Send + Sync {
  /// The router calls this hook during attachment, before registering
  /// request hooks.
  fn on_attach(&mut self, _: &mut crate::router::Router) {}

  /// The router calls this hook before dispatching a request, even if the
  /// request does not match a route.
  fn on_pre_route(&self, _: &HookContext) {}

  /// The router calls this hook after the handler completes, before
  /// response serialisation.
  fn on_post_route(&self, _: &HookContext) {}
}

/// Asynchronous modules run hooks in registration order for each request.
///
/// Hook phases run exclusively across requests unless
/// `RouterOption::AllowConcurrentModules` is enabled. Concurrent phases may
/// invoke the same module while an earlier invocation awaits work. Modules
/// own their state synchronisation; attachment still receives exclusive access.
#[async_trait::async_trait]
pub trait AsyncModule: Send + Sync {
  /// The router calls this hook during attachment, before registering
  /// request hooks.
  async fn on_attach(&mut self, _: &mut crate::router::Router) {}

  /// The router calls this hook before dispatching a request, even if the
  /// request does not match a route.
  async fn on_pre_route(&self, _: &HookContext) {}

  /// The router calls this hook after the handler completes, before
  /// response serialisation.
  async fn on_post_route(&self, _: &HookContext) {}
}
