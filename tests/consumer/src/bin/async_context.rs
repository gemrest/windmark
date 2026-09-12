use std::future::Future;
use windmark::{context::RouteContext, response::Response};

fn main() {
  let route = |context: RouteContext| {
    async move { Response::success(std::future::ready(context.url.path()).await) }
  };

  for path in ["/one", "/two"] {
    let context = RouteContext {
      peer_address: None,
      url:          format!("gemini://localhost{path}").parse().unwrap(),
      parameters:   Default::default(),
      certificate:  None,
    };
    let future = route(context);
    let mut future = std::pin::pin!(future);
    let mut task = std::task::Context::from_waker(std::task::Waker::noop());
    let std::task::Poll::Ready(response) = future.as_mut().poll(&mut task)
    else {
      panic!("the response should be ready");
    };

    assert_eq!(response.content().unwrap(), path);
  }

  windmark::router::Router::new().mount("/", route);
}
