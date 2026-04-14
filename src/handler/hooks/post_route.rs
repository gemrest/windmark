use crate::{context::HookContext, response::Response};

#[allow(clippy::module_name_repetitions)]
pub trait PostRouteHook: Send + Sync {
  fn call(&mut self, context: &HookContext, response: &mut Response);
}

impl<T> PostRouteHook for T
where T: FnMut(&HookContext, &mut Response) + Send + Sync
{
  fn call(&mut self, context: &HookContext, response: &mut Response) {
    (*self)(context, response);
  }
}
