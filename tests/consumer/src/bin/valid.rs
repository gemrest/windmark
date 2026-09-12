use rossweisse::route;
use std::{
  future::IntoFuture,
  sync::atomic::{AtomicUsize, Ordering},
};
use windmark::{
  context::{ErrorContext, HookContext, RouteContext},
  handler::{
    ErrorResponse,
    Partial,
    PostRouteHook,
    PreRouteHook,
    RouteResponse,
  },
  module::{AsyncModule, Module},
  response::Response,
  router::Router,
  router_option::RouterOption,
};

#[rossweisse::router]
struct Empty;

#[rossweisse::router]
impl Empty {}

#[rossweisse::router(count = 7)]
struct Capsule {
  count: usize,
  label: String,
}

#[rossweisse::router]
impl Capsule {
  #[route(index)]
  pub fn index(_: RouteContext) -> Response { Response::success("index") }
}

struct Counter(AtomicUsize);

impl Module for Counter {
  fn on_pre_route(&self, _: &HookContext) {
    self.0.fetch_add(1, Ordering::Relaxed);
  }
}

struct AwaitableModule;

#[async_trait::async_trait]
impl AsyncModule for AwaitableModule {
  async fn on_pre_route(&self, _: &HookContext) {}
}

struct Banner;

impl Partial for Banner {
  fn call(&self, _: &RouteContext) -> String { "banner".to_owned() }
}

struct Observer;

impl PreRouteHook for Observer {
  fn call(&self, _: &HookContext) {}
}

impl PostRouteHook for Observer {
  fn call(&self, _: &HookContext, response: &mut Response) {
    if let Some(content) = response.content_mut() {
      content.push('!');
    }
  }
}

struct Replies;

#[async_trait::async_trait]
impl RouteResponse for Replies {
  async fn call(&self, _: RouteContext) -> Response {
    Response::success("route")
  }
}

#[async_trait::async_trait]
impl ErrorResponse for Replies {
  async fn call(&self, _: ErrorContext) -> Response {
    Response::not_found("missing")
  }
}

struct AwaitableResponse;

impl IntoFuture for AwaitableResponse {
  type IntoFuture = std::future::Ready<Response>;
  type Output = Response;

  fn into_future(self) -> Self::IntoFuture {
    std::future::ready(Response::success("awaitable"))
  }
}

struct Bytes;

impl AsRef<[u8]> for Bytes {
  fn as_ref(&self) -> &[u8] { b"bytes" }
}

fn context() -> RouteContext {
  RouteContext {
    peer_address: None,
    url:          "gemini://localhost/path".parse().unwrap(),
    parameters:   Default::default(),
    certificate:  None,
  }
}

async fn attach_module(router: &mut Router) {
  router.attach_async(AwaitableModule).await;
}

fn main() {
  let mut router = Router::new();
  let capsule = Capsule::new();
  let _empty = Empty::new();
  let _attachment = attach_module;
  let _route: &dyn RouteResponse = &Replies;
  let _error: &dyn ErrorResponse = &Replies;

  router.attach(Counter(AtomicUsize::new(0)));
  router.add_header(Banner);
  router.add_footer(Banner);
  router.set_pre_route_callback(Observer);
  router.set_post_route_callback(Observer);
  router.mount("/awaitable", |_| AwaitableResponse);
  router.add_options(&[RouterOption::AllowCaseInsensitiveLookup]);
  assert_eq!(capsule.count, 7);
  assert!(capsule.label.is_empty());
  assert_eq!(Capsule::index(context()).content().unwrap(), "index");

  let mut response = Response::input::<String>("prompt".to_owned());

  assert_eq!(response.content(), Some("prompt"));

  response = Response::new(79, "metadata");
  *response.mime_mut() = Some("text/plain; charset=iso-8859-1".to_owned());
  *response.character_set_mut() = Some("iso-8859-1".to_owned());
  *response.languages_mut() = Some(vec!["en".to_owned()]);

  assert!(response.serialize_body("header", "footer").is_empty());
  assert_eq!(Response::binary_success(Bytes, "text/plain").status(), 21);
  assert_eq!(Response::binary_success_auto(b"hello").status(), 22);
}
