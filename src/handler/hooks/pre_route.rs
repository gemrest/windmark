use crate::context::HookContext;

#[allow(clippy::module_name_repetitions)]
pub trait PreRouteHook: Send + Sync {
  fn call(&self, context: &HookContext);
}

impl<T> PreRouteHook for T
where T: Fn(&HookContext) + Send + Sync
{
  fn call(&self, context: &HookContext) { (*self)(context) }
}
