use crate::context::HookContext;

#[allow(clippy::module_name_repetitions)]
pub trait PreRouteHook: Send + Sync {
  fn call(&mut self, context: &HookContext);
}

impl<T> PreRouteHook for T
where T: FnMut(&HookContext) + Send + Sync
{
  fn call(&mut self, context: &HookContext) { (*self)(context) }
}
