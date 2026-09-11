# Examples

## [Async Stateful Module](./async_stateful_module.rs)

`cargo run --example async_stateful_module --features response-macros`

Demonstrates use of the `AsyncModule` trait by implementing the module
`Clicker` which tracks the global number of visits to the capsule.

This can easily be adapted to contain a hashmap of routes which are individually
tracked for clicks.

## [Async](./async.rs)

`cargo run --example async --features response-macros`

This example shows how to define async routes with an async response macro
and implement a click tracker using a shared variable protected by an async
mutex.

## [Binary](./binary.rs)

`cargo run --example binary --features response-macros`

Demonstrates the binary response functionality by using both manual
and automatic mime resolution (`--features auto-deduce-mime`).

## [Callbacks](./callbacks.rs)

`cargo run --example callbacks --features response-macros`

This example shows how to register pre-route and post-route callbacks.

## [Certificate](./certificate.rs)

`cargo run --example certificate --features response-macros`

Demonstrate the various certificate related responses as well as
reading the client certificate to give conditional access.

## [Default Logger](./default_logger.rs)

`cargo run --example default_logger --features logger,response-macros`

A simple example showing the use of the default logger implementation.

## [Empty](./empty.rs)

`cargo run --example empty`

An empty example which starts up a server but has no mounted routes.

## [Error Handler](./error_handler.rs)

`cargo run --example error_handler`

This example shows how to handle requests that do not match a route. It also
includes an `/error` route that deliberately panics; this panic does not invoke
the error handler.

## [Fix Path](./fix_path.rs)

`cargo run --example fix_path --features response-macros`

A simple example which demonstrates use of the path fixer that attempts to resolve the closest match of a route when an invalid route is visited.

This feature is limited to simple resolution patches such as resolving
trailing and missing trailing slashes. If your capsule requires a more sophisticated path fixer, please use any of the provided mechanisms to do so before your routes execute.

## [Input](./input.rs)

`cargo run --example input`

Demonstrates how to accept and inspect both standard and sensitive input.

## [MIME](./mime.rs)

`cargo run --example mime`

Demonstrate how to modify the MIME of a response before use.

## [Parameters](./parameters.rs)

`cargo run --example parameters --features response-macros`

Demonstrate the use of route parameters (not URL queries).

## [Partial](./partial.rs)

`cargo run --example partial --features response-macros`

Demonstrates use of appending headers and footers to routes, globally.

If you would like to conditionally append headers and footers based on route, please look into using a templating framework.

## [Query](./query.rs)

`cargo run --example query --features response-macros`

Demonstrates the inspection of URL queries parameters.

## [Responses](./responses.rs)

`cargo run --example responses --features response-macros`

Demonstrates the use of a wide variety of responses, additionally exposing the flexibility of response bodies types.

## [Simple `async-std`](./simple_async_std.rs)

`cargo run --example simple_async_std --no-default-features --features async-std`

Demonstrates how to explicitly specify Windmark to use the [`async-std`](https://github.com/async-rs/async-std) runtime.

Windmark uses Tokio when its default features are enabled. If you disable
default features, you must explicitly enable either `tokio` or `async-std`.

## [Simple Tokio](./simple_tokio.rs)

`cargo run --example simple_tokio`

Demonstrates how to explicitly specify Windmark to use the [Tokio](https://github.com/tokio-rs/tokio) runtime.

## [Stateful Module](./stateful_module.rs)

`cargo run --example stateful_module --features response-macros`

Demonstrates use of `Module`s by implementing a click tracker

Identical in functionality to the Async Stateful Module example, just not asynchronous.

## [Stateless Module](./stateless_module.rs)

`cargo run --example stateless_module`

Demonstrates use of a stateless module.

Unlike a `Module`, a stateless module is not encapsulated into a `struct`, but is a simple function which is used to perform operations.

Stateless modules are able to emulate stateful modules employing `static` variables. The earliest Windmark modules (add-ons) were made this way.

The only requirement of a module is to implement the signature of a stateless module: `FnMut(&mut Router) -> ()`.
