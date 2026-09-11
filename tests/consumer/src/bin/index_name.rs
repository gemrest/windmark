use rossweisse::route;

#[rossweisse::router]
struct Capsule;

#[rossweisse::router]
impl Capsule {
  #[route(index)]
  pub fn index(
    _: windmark::context::RouteContext,
  ) -> windmark::response::Response {
    windmark::response::Response::success("index")
  }
}

fn main() { let _ = Capsule::index; }
