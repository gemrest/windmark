# Windmark

[![crates.io](https://img.shields.io/crates/v/windmark.svg)](https://crates.io/crates/windmark)
[![docs.rs](https://docs.rs/windmark/badge.svg)](https://docs.rs/windmark)
[![github.com](https://github.com/gemrest/windmark/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/windmark/actions/workflows/check.yaml)

Windmark is an elegant and highly performant, async Gemini server framework for
the modern age!

Now supporting both [Tokio](https://github.com/tokio-rs/tokio) and [`async-std`](https://github.com/async-rs/async-std)!

## Usage

### Add Windmark and Tokio as Dependencies

```toml
# Cargo.toml

[dependencies]
windmark = "0.3.6"
tokio = { version = "1.26.0", features = ["full"] }

# If you would like to use the built-in logger (recommended)
# windmark = { version = "0.3.6", features = ["logger"] }

# If you would like to use the built-in MIME dedection when `Success`-ing a file
# (recommended)
# windmark = { version = "0.3.6", features = ["auto-deduce-mime"] }

# If you would like to use macro-based responses (as seen below)
# windmark = { version = "0.3.6", features = ["response-macros"] }
```

### Implement a Windmark server

```rust
// src/main.rs

use windmark::Response;

#[windmark::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_file("windmark_public.pem")
    .mount("/", windmark::success!("Hello, World!"))
    .set_error_handler(
      windmark::permanent_failure!("This route does not exist!")
    )
    .run()
    .await
}
```

## Examples

Examples can be found within the
[`examples/`](https://github.com/gemrest/windmark/tree/main/examples) directory
along with a rundown of each of their purposes and useful facts.

An example of a fully featured Gemini capsule written using Windmark can be
found [here](https://github.com/gemrest/locus). This example Gemini capsule also
happens to be the source code for [Fuwn's](https://github.com/Fuwn) (this
library's author) personal Gemini capsule!

## Modules

Modules are reusable extensions which can be procedurally mounted onto Windmark
routers.

[Add yours!](https://github.com/gemrest/windmark/edit/main/README.md)

- [Windmark Comments](https://github.com/gemrest/windmark-comments)

## Capsules using Windmark

[Add yours!](https://github.com/gemrest/windmark/edit/main/README.md)

- <https://fuwn.me/>

## License

This project is licensed with the
[GNU General Public License v3.0](https://github.com/gemrest/windmark/blob/main/LICENSE).
