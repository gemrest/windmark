#![deny(
  clippy::all,
  clippy::nursery,
  clippy::pedantic,
  future_incompatible,
  nonstandard_style,
  rust_2018_idioms,
  unsafe_code,
  unused,
  warnings
)]
#![recursion_limit = "128"]

mod implementations;

use proc_macro::TokenStream;
use syn::Item;

/// Marks a `struct` as a router or marks an `impl` block as a router
/// implementation
///
/// # Panics
///
/// Panics if used on an item that is not a `struct` or `impl` block.
///
/// # Examples
///
/// ```rust
/// use rossweisse::route;
/// use windmark::response::Response;
///
/// #[rossweisse::router]
/// struct Router {
///   _phantom: (),
/// }
///
/// #[rossweisse::router]
/// impl Router {
///   #[route]
///   pub fn index(_context: windmark::context::RouteContext) -> Response {
///     Response::success("Hello, World!")
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn router(arguments: TokenStream, item: TokenStream) -> TokenStream {
  match syn::parse::<Item>(item) {
    Ok(Item::Struct(item)) => implementations::fields(arguments, item),
    Ok(Item::Impl(item)) => implementations::methods(arguments, item),
    _ => panic!("`#[rossweisse::router]` can only be used on `struct`s"),
  }
}

/// Marks a method of a router implementation as a route to mount
///
/// # Panics
///
/// Panics if used on an item that is not a function.
///
/// # Examples
///
/// ```rust
/// use rossweisse::route;
/// use windmark::response::Response;
///
/// #[rossweisse::router]
/// impl Router {
///   #[route]
///   pub fn index(_context: windmark::context::RouteContext) -> Response {
///     Response::success("Hello, World!")
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn route(arguments: TokenStream, item: TokenStream) -> TokenStream {
  match syn::parse::<Item>(item) {
    Ok(Item::Fn(ref item)) => implementations::route(arguments, item),
    _ => panic!("`#[rossweisse::route]` can only be used on `fn`s"),
  }
}
