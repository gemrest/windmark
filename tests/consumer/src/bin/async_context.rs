use std::future::Future;
use windmark::context::RouteContext;

fn main() {
  let greeting = String::from("Hello");
  let route = windmark::success_async!(
    context,
    format!("{greeting} {}", context.url.path())
  );

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

    assert_eq!(response.content, format!("Hello {path}"));
  }

  let route = windmark::success_async!(
    context,
    std::future::ready(context.url.path()).await
  );

  windmark::router::Router::new().mount("/", route);
}
