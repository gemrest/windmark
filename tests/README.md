# Tests

Run the complete workspace with an explicitly selected runtime:

```sh
cargo test --workspace --features windmark/tokio
cargo test --workspace --no-default-features --features windmark/tokio,windmark/logger,windmark/auto-deduce-mime,windmark/response-macros
cargo test --workspace --no-default-features --features windmark/async-std,windmark/logger,windmark/auto-deduce-mime,windmark/response-macros
```

The qualified runtime feature also selects the runtime for Rossweisse's
Windmark development dependency.

## Layout

`router.rs` is loaded as a private unit-test module to exercise internal
helpers without publishing them. Integration targets are explicitly listed in
`Cargo.toml`, so Cargo does not also compile that file as a separate crate.

## Consumer Compatibility

`consumer.rs` checks the independent crate in `consumer/`.

The failing fixtures preserve known API limitations so changes to that
behaviour are deliberate.

Format both the workspace and the independent consumer crate.
