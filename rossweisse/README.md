# Rossweisse

`struct`-based Router Framework for [Windmark](https://github.com/gemrest/windmark)

## Usage

Rossweisse is in it's infancy, and a much comprehensive interface is planned.

For now, a simple Rosswiesse router can be implemented like this:

```rust
use rossweisse::route;
use windmark::response::Response;

#[rossweisse::router]
struct Router;

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

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
