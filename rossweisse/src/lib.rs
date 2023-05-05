// This file is part of Windmark <https://github.com/gemrest/windmark>.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Copyright (C) 2022-2023 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

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
  let output = match syn::parse::<Item>(item.clone()) {
    Ok(Item::Struct(item)) => implementations::fields(arguments, item),
    Ok(Item::Impl(item)) => implementations::methods(arguments, item),
    _ => panic!("`#[rossweisse::router]` can only be used on `struct`s"),
  };

  output.into()
}

/// Marks a method of a router implementation as a route to mount
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
  let output = match syn::parse::<Item>(item.clone()) {
    Ok(Item::Fn(item)) => implementations::route(arguments, item),
    _ => panic!("`#[rossweisse::route]` can only be used on `fn`s"),
  };

  output.into()
}
