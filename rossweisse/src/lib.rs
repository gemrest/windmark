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

mod router_implementation;
mod router_struct;

use proc_macro::TokenStream;
use syn::{punctuated::Punctuated, Error, Item, Meta, Token};

/// Mark a `struct` as a router or an `impl` block as a router implementation.
///
/// Struct attributes and generics are preserved. Each field initialiser must
/// name a declared field, and repeated names are rejected. The `router` field
/// and the `new`, `_new`, `run`, and `router` methods are reserved for
/// generated code.
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
  syn::parse::<Item>(item)
    .and_then(|item| {
      match item {
        Item::Struct(item) => router_struct::expand(arguments, item),
        Item::Impl(item) => router_implementation::expand(&item),
        item =>
          Err(Error::new_spanned(
            item,
            "`#[rossweisse::router]` requires a struct or impl block",
          )),
      }
    })
    .unwrap_or_else(|error| error.into_compile_error().into())
}

/// Mark a method of a router implementation as a route to mount.
///
/// Within a router implementation, `#[route(index)]` mounts the method at `/`
/// without changing its name.
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
  syn::parse::<Item>(item)
    .and_then(|item| {
      let Item::Fn(function) = item else {
        return Err(Error::new_spanned(
          item,
          "`#[rossweisse::route]` requires a function",
        ));
      };

      if !arguments.is_empty() {
        let argument = syn::parse::<syn::Ident>(arguments)?;

        if argument != "index" {
          return Err(Error::new_spanned(
            argument,
            "the only supported route argument is `index`",
          ));
        }
      }

      Ok(quote::quote! { #function }.into())
    })
    .unwrap_or_else(|error| error.into_compile_error().into())
}

fn conditional_attributes(
  attributes: &[syn::Attribute],
) -> Vec<syn::Attribute> {
  attributes
    .iter()
    .filter_map(|attribute| {
      conditional_meta(&attribute.meta).map(|meta| syn::parse_quote!(#[#meta]))
    })
    .collect()
}

fn conditional_meta(meta: &Meta) -> Option<Meta> {
  if meta.path().is_ident("cfg") {
    return Some(meta.clone());
  }

  let Meta::List(list) = meta else {
    return None;
  };

  if !list.path.is_ident("cfg_attr") {
    return None;
  }

  let arguments = list
    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
    .ok()?;
  let mut arguments = arguments.into_iter();
  let predicate = arguments.next()?;
  let conditions: Vec<_> = arguments
    .filter_map(|meta| conditional_meta(&meta))
    .collect();

  if conditions.is_empty() {
    None
  } else {
    Some(syn::parse_quote!(cfg_attr(#predicate, #(#conditions),*)))
  }
}
