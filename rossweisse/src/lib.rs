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

mod implementations;

use proc_macro::TokenStream;
use syn::Item;

/// Mark a `struct` as a router or an `impl` block as a router implementation.
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
    Ok(item) =>
      syn::Error::new_spanned(
        item,
        "`#[rossweisse::router]` requires a struct or impl block",
      )
      .to_compile_error()
      .into(),
    Err(error) => error.to_compile_error().into(),
  }
}

/// Mark a method of a router implementation as a route to mount.
///
/// # Examples
///
/// ```rust
/// use rossweisse::route;
/// use windmark::response::Response;
///
/// # #[rossweisse::router]
/// # struct Router;
/// #
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
    Ok(item) =>
      syn::Error::new_spanned(
        item,
        "`#[rossweisse::route]` requires a function",
      )
      .to_compile_error()
      .into(),
    Err(error) => error.to_compile_error().into(),
  }
}
