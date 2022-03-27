# Windmark

[![crates.io](https://img.shields.io/crates/v/windmark.svg)](https://crates.io/crates/windmark)
[![docs.rs](https://docs.rs/windmark/badge.svg)](https://docs.rs/windmark)
[![github.com](https://github.com/gemrest/windmark/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/windmark/actions/workflows/check.yaml)

Windmark is An elegant and highly performant async Gemini server framework.

## Usage

### Add Windmark as a dependency

```toml
# Cargo.toml

[dependencies]
windmark = "0.1.1"

# If you would like to use the built-in logger (reccomended)
# windmark = { version = "0.1.1", features = ["logger"] }
```

### Implement a Windmark server

```rust
// src/main.rs

use windmark::Response;

#[windmark::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
  windmark::Router::new()
    .set_private_key_file("windmark_private.pem")
    .set_certificate_chain_file("windmark_pair.pem")
    .mount("/", |_| Response::Success("Hello, World!".into()))
    .set_error_handler(|_| {
      Response::PermanentFailure("This route does not exist!".into())
    })
    .run()
    .await
}
```

## Examples

Examples can be found within the [`examples/`](./examples) directory.

## License

This project is licensed with the [GNU General Public License v3.0](./LICENSE).
