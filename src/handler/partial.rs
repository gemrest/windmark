use crate::context::RouteContext;

#[allow(clippy::module_name_repetitions)]
pub trait Partial: Send + Sync {
  fn call(&self, context: &RouteContext) -> String;
}

impl<T> Partial for T
where T: Fn(&RouteContext) -> String + Send + Sync
{
  fn call(&self, context: &RouteContext) -> String { (*self)(context) }
}
