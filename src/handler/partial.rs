use crate::context::RouteContext;

#[allow(clippy::module_name_repetitions)]
pub trait Partial: Send + Sync {
  fn call(&mut self, context: &RouteContext) -> String;
}

impl<T> Partial for T
where T: FnMut(&RouteContext) -> String + Send + Sync
{
  fn call(&mut self, context: &RouteContext) -> String { (*self)(context) }
}
