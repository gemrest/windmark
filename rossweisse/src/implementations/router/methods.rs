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

use proc_macro::TokenStream;

pub fn methods(
  _arguments: TokenStream,
  mut item: syn::ItemImpl,
) -> TokenStream {
  let routes = item
    .items
    .iter_mut()
    .filter_map(|item| {
      let syn::ImplItem::Fn(method) = item else {
        return None;
      };
      let route_attrribute = method
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("route"))?;
      let arguments = quote::ToTokens::into_token_stream(route_attrribute)
        .to_string()
        .trim_end_matches(")]")
        .trim_start_matches("#[route(")
        .to_string();

      if arguments == "index" {
        method.sig.ident =
          syn::Ident::new("__router_index", method.sig.ident.span());
      }

      Some(method.sig.ident.clone())
    })
    .collect::<Vec<_>>();
  let (implementation_generics, type_generics, where_clause) =
    item.generics.split_for_impl();
  let name = &item.self_ty;
  let route_paths = routes
    .iter()
    .map(|route| {
      format!(
        "/{}",
        if route == "__router_index" {
          String::new()
        } else {
          route.to_string()
        }
      )
    })
    .collect::<Vec<_>>();

  quote::quote! {
    #item

    impl #implementation_generics #name #type_generics #where_clause {
      pub fn new() -> Self {
        let mut router = Self::_new();

        #(
          router.router.mount(#route_paths, |context| {
            Self::#routes(context)
          });
        )*

        router
      }
    }
  }
  .into()
}
