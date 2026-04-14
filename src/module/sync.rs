use crate::context::HookContext;

pub trait Module {
  /// Called right after the module is attached.
  fn on_attach(&mut self, _: &mut crate::router::Router) {}

  /// Called before a route is mounted.
  fn on_pre_route(&mut self, _: &HookContext) {}

  /// Called after a route is mounted.
  fn on_post_route(&mut self, _: &HookContext) {}
}
