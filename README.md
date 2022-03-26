# Windmark

[![crates.io](https://img.shields.io/crates/v/windmark.svg)](https://crates.io/crates/windmark)
[![docs.rs](https://docs.rs/windmark/badge.svg)](https://docs.rs/windmark)
[![github.com](https://github.com/gemrest/windmark/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/windmark/actions/workflows/check.yaml)

Windmark is a simple and highly performant Gemini server framework.

## Usage

### Add Windmark as a dependency

```toml
# Cargo.toml

[dependencies]
windmark = "0.1.0"

# If you would like to use the built-in logger (reccomended)
# windmark = { version = "0.1.0", features = ["logger"] }
```

### Implement a Windmark server

```rust
use windmark::response::Response;

fn main() -> std::io::Result<()> {
  windmark::Router::new()
    .mount("/", |_, _, _| Response::Success("Hello, World!".into()))
    .set_error_handler(|_, _, _| {
      Response::PermanentFailure("This route does not exist!".into())
    })
    .run()
}
```

## Examples

Examples can be found within the [`examples/`](./examples) directory.

## License

This project is licensed with the [GNU General Public License v3.0](./LICENSE).
