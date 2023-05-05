# Rossweisse

`struct`-based Router Framework for [Windmark](https://github.com/gemrest/windmark)

## Usage

Rossweisse is in it's infancy, and a much comprehensive interface is planned.

For now, a simple Rosswiesse router can be implemented like this:

```rust
use rossweisse::route;
use windmark::response::Response;

#[rossweisse::router]
struct Router {
  _phantom: (),
}

#[rossweisse::router]
impl Router {
  #[route(index)]
  pub fn index(
    _context: windmark::context::RouteContext,
  ) -> Response {
    Response::success("Hello, World!")
  }
}

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  {
    let mut router = Router::new();

    router.router().set_private_key_file("windmark_private.pem");
    router.router().set_certificate_file("windmark_public.pem");

    router
  }
  .run()
  .await
}
```

## License

This project is licensed with the
[GNU General Public License v3.0](https://github.com/gemrest/windmark/blob/main/LICENSE).
