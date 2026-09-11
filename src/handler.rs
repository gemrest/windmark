use crate::{
  context::{ErrorContext, HookContext, RouteContext},
  response::Response,
};
use async_trait::async_trait;

#[async_trait]
pub trait RouteResponse: Send + Sync {
  async fn call(&self, context: RouteContext) -> Response;
}

#[async_trait]
impl<T, F> RouteResponse for T
where
  T: Fn(RouteContext) -> F + Send + Sync,
  F: std::future::Future<Output = Response> + Send + 'static,
{
  async fn call(&self, context: RouteContext) -> Response {
    (*self)(context).await
  }
}

#[async_trait]
pub trait ErrorResponse: Send + Sync {
  async fn call(&self, context: ErrorContext) -> Response;
}

#[async_trait]
impl<T, F> ErrorResponse for T
where
  T: Fn(ErrorContext) -> F + Send + Sync,
  F: std::future::Future<Output = Response> + Send + 'static,
{
  async fn call(&self, context: ErrorContext) -> Response {
    (*self)(context).await
  }
}

pub trait Partial: Send + Sync {
  fn call(&self, context: &RouteContext) -> String;
}

impl<T> Partial for T
where T: Fn(&RouteContext) -> String + Send + Sync
{
  fn call(&self, context: &RouteContext) -> String { (*self)(context) }
}

pub trait PreRouteHook: Send + Sync {
  fn call(&self, context: &HookContext);
}

impl<T> PreRouteHook for T
where T: Fn(&HookContext) + Send + Sync
{
  fn call(&self, context: &HookContext) { (*self)(context) }
}

pub trait PostRouteHook: Send + Sync {
  fn call(&self, context: &HookContext, response: &mut Response);
}

impl<T> PostRouteHook for T
where T: Fn(&HookContext, &mut Response) + Send + Sync
{
  fn call(&self, context: &HookContext, response: &mut Response) {
    (*self)(context, response);
  }
}
