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

pub fn methods(_arguments: TokenStream, item: syn::ItemImpl) -> TokenStream {
  let routes = item
    .items
    .iter()
    .filter_map(|item| {
      if let syn::ImplItem::Fn(method) = item {
        if method
          .attrs
          .iter()
          .any(|attribute| attribute.path().is_ident("route"))
        {
          Some(method.sig.ident.clone())
        } else {
          None
        }
      } else {
        None
      }
    })
    .collect::<Vec<_>>();
  let (implementation_generics, type_generics, where_clause) =
    item.generics.split_for_impl();
  let name = &item.self_ty;
  let route_paths = routes
    .iter()
    .map(|route| format!("/{}", route))
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
