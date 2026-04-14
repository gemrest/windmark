use async_trait::async_trait;

use crate::{context::RouteContext, response::Response};

#[allow(clippy::module_name_repetitions)]
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
