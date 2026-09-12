use windmark::{context::RouteContext, response::Response, router::Router};

fn main() {
  let mut router = Router::new();

  router.mount("/explicit", |context: RouteContext| {
    Response::binary_success(context.url.path().as_bytes(), "text/plain")
  });
  router.mount("/inferred", |context: RouteContext| {
    Response::binary_success_auto(context.url.path().as_bytes())
  });
}
