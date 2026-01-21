# Windmark

[![crates.io](https://img.shields.io/crates/v/windmark.svg)](https://crates.io/crates/windmark)
[![docs.rs](https://docs.rs/windmark/badge.svg)](https://docs.rs/windmark)
[![github.com](https://github.com/gemrest/windmark/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/windmark/actions/workflows/check.yaml)

Windmark is an elegant and highly performant async Gemini server framework for
the modern age!

Now supporting both [Tokio](https://tokio.rs/) and [`async-std`](https://async.rs/)!

## Usage

> [!NOTE]
> A macro-based "`struct`-router" is in active development as a simplified
> alternative to the standard server creation approach. Check out
> [Rossweisse](./rossweisse/) for more information!

### Features

| Feature            | Description                                                                                             |
| ------------------ | ------------------------------------------------------------------------------------------------------- |
| `default`          | Base Windmark framework using [Tokio](https://tokio.rs/)                                                |
| `logger`           | Enables the default [`pretty_env_logger`](https://github.com/seanmonstar/pretty-env-logger) integration |
| `auto-deduce-mime` | Exposes `Response`s and macros that automatically fill MIMEs for non-Gemini responses                   |
| `response-macros`  | Simple macros for all `Response`s                                                                       |
| `tokio`            | Marks [Tokio](https://tokio.rs/) as the asynchronous runtime                                            |
| `async-std`        | Marks [`async-std`](https://async.rs/) as the asynchronous runtime                                      |
| `prelude`          | Exposes the `prelude` module containing the most used Windmark features                                 |

### Add Windmark and Tokio as Dependencies

```toml
# Cargo.toml

[dependencies]
windmark = "0.4.2"
tokio = { version = "1.26.0", features = ["full"] }

# If you would like to use the built-in logger (recommended)
# windmark = { version = "0.4.2", features = ["logger"] }

# If you would like to use the built-in MIME deduction when `Success`-ing a file
# (recommended)
# windmark = { version = "0.4.2", features = ["auto-deduce-mime"] }

# If you would like to use macro-based responses (as seen below)
# windmark = { version = "0.4.2", features = ["response-macros"] }
```

### Implementing a Windmark Server

```rust
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

use windmark::response::Response;

#[rossweisse::router]
struct Router;

#[rossweisse::router]
impl Router {
  #[rossweisse::route(index)]
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

Run an example by cloning this repository and running `cargo run --example example_name`.

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

This project is licensed with the
[GNU General Public License v3.0](https://github.com/gemrest/windmark/blob/main/LICENSE).
