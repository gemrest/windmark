use async_trait::async_trait;

use crate::{context::ErrorContext, response::Response};

#[allow(clippy::module_name_repetitions)]
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
