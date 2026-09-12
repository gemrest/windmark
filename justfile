import? 'cargo.just'

set allow-duplicate-recipes := true

default-features := "--features=auto-deduce-mime,response-macros,"

default:
  @just --list

fetch:
  curl https://raw.githubusercontent.com/Fuwn/justfiles/refs/heads/main/cargo.just > cargo.just

# `cargo fmt` resolves `rustfmt` via PATH, so prepend the nightly toolchain's
# bin to reach nightly rustfmt (the unstable rustfmt.toml options need it).
fmt:
  PATH="$(dirname "$(rustup which --toolchain nightly rustfmt)"):$PATH" cargo fmt

[private]
generic-task task async-feature:
  PATH="$(dirname "$(rustup which --toolchain nightly rustc)"):$PATH" \
    cargo {{ task }} --no-default-features \
    {{ default-features }}{{ async-feature }}

check async-feature:
  @just generic-task check {{ async-feature }}

clippy async-feature:
  @just generic-task clippy {{ async-feature }}

test async-feature:
  @just generic-task test {{ async-feature }}

checkf:
  @just fmt
  @just check tokio
  @just check async-std

checkfc:
  @just checkf
  @just clippy tokio
  @just clippy async-std

docs:
  rustup run nightly cargo doc --open --no-deps

example example async-feature="tokio":
  cargo run --example {{ example }} --no-default-features \
    {{ default-features }}{{ async-feature }}

gen-key:
  openssl req -new -subj /CN=localhost -x509 -newkey ec -pkeyopt \
    ec_paramgen_curve:prime256v1 -days 365 -nodes -out windmark_public.pem \
    -keyout windmark_private.pem -inform pem
