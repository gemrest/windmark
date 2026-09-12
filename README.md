# Windmark

[![crates.io](https://img.shields.io/crates/v/windmark.svg)](https://crates.io/crates/windmark)
[![docs.rs](https://docs.rs/windmark/badge.svg)](https://docs.rs/windmark)
[![github.com](https://github.com/gemrest/windmark/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/windmark/actions/workflows/check.yaml)

Windmark is an elegant and highly performant async Gemini server framework for
the modern age!

Now supporting both [Tokio](https://tokio.rs/) and [`async-std`](https://async.rs/)!

## Usage

> [!NOTE]
> Rossweisse lets you define a Windmark router using a struct and route
> attributes. See the [Rossweisse guide](./rossweisse/) for an example.

### Features

| Feature            | Description                                                                                             |
| ------------------ | ------------------------------------------------------------------------------------------------------- |
| `default`          | Base Windmark framework using [Tokio](https://tokio.rs/)                                                |
| `auto-deduce-mime` | Enables `Response::binary_success_auto` to infer the MIME type of binary content                   |
| `tokio`            | Marks [Tokio](https://tokio.rs/) as the asynchronous runtime                                            |
| `async-std`        | Marks [`async-std`](https://async.rs/) as the asynchronous runtime                                      |

### Add Windmark and Tokio as Dependencies

```toml
# Cargo.toml

[dependencies]
windmark = "0.7.0"
tokio = { version = "1.26.0", features = ["full"] }

# If you would like to use the built-in MIME deduction when `Success`-ing a file
# (recommended)
# windmark = { version = "0.7.0", features = ["auto-deduce-mime"] }
```

### Implementing a Windmark Server

```rust,no_run
// src/main.rs

use windmark::response::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::router::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/", |_| Response::success("Hello, World!"))
    .set_error_handler(|_|
      Response::permanent_failure("This route does not exist!")
    )
    .run()
    .await
}
```

### Implementing a Windmark Server Using Rossweisse

```rust
// src/main.rs

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

// ...
```

## Examples

Examples can be found within the
[`examples/`](https://github.com/gemrest/windmark/tree/main/examples) directory
along with a rundown of each of their purposes and useful facts.

Each entry in [the examples guide](./examples/README.md) lists the required
features. Generate local credentials with `just gen-key`, then use the listed
command or `just example example_name` (optionally followed by `async-std`).

## Modules

Modules are composable extensions which can be procedurally mounted onto Windmark
routers.

### Examples

- [Simple Stateless Module](https://github.com/gemrest/windmark/blob/main/examples/stateless_module.rs)
  \- Mounts the `/smiley` route, returning an 😀 emoji
- [Simple Stateful Module](https://github.com/gemrest/windmark/blob/main/examples/stateful_module.rs)
  \- Adds a click tracker (route hit tracker) that additionally notifies before and after route visits
- [Windmark Comments](https://github.com/gemrest/windmark-comments) - A fully featured comment engine
  for your capsule

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
