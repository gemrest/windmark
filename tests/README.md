# Tests

Run the complete workspace with an explicitly selected runtime:

```sh
cargo test --workspace --features windmark/tokio
cargo test --workspace --no-default-features --features windmark/tokio,windmark/auto-deduce-mime
cargo test --workspace --no-default-features --features windmark/async-std,windmark/auto-deduce-mime
```

The qualified runtime feature also selects the runtime for Rossweisse's
Windmark development dependency.

## Consumer Compatibility

`consumer.rs` checks the independent crate in `consumer/`.

The failing fixtures preserve known API limitations so changes to that
behaviour are deliberate.

Format both the workspace and the independent consumer crate.
