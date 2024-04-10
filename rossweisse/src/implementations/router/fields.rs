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
use quote::quote;

pub fn fields(arguments: TokenStream, item: syn::ItemStruct) -> TokenStream {
  let field_initializers = syn::parse_macro_input!(
    arguments as super::parser::FieldInitializers<syn::Expr>
  );
  let router_identifier = item.ident;
  let (named_fields, has_fields) = match item.fields {
    syn::Fields::Named(fields) => (fields, true),
    syn::Fields::Unit =>
      (
        syn::FieldsNamed {
          brace_token: syn::token::Brace::default(),
          named:       Default::default(),
        },
        false,
      ),
    _ =>
      panic!(
        "`#[rossweisse::router]` can only be used on `struct`s with named \
         fields or unit structs"
      ),
  };
  let mut default_expressions = vec![];
  let new_method_fields = named_fields.named.iter().map(|field| {
    let name = &field.ident;
    let initialiser = field_initializers
      .0
      .iter()
      .find(|initialiser| initialiser.ident == name.clone().unwrap())
      .map(|initialiser| &initialiser.expr)
      .unwrap_or_else(|| {
        default_expressions.push({
          let default_expression: syn::Expr =
            syn::parse_quote! { ::std::default::Default::default() };

          default_expression
        });

        default_expressions.last().unwrap()
      });

    quote! {
        #name: #initialiser,
    }
  });
  let new_methods = if has_fields {
    quote! {
      fn _new() -> Self {
        Self {
          #(#new_method_fields)*
          router: ::windmark::router::Router::new(),
        }
      }
    }
  } else {
    quote! {
      fn _new() -> Self {
        Self {
          router: ::windmark::router::Router::new(),
        }
      }
    }
  };
  let output_fields = named_fields.named;
  let output = quote! {
    struct #router_identifier {
      #output_fields
      router: ::windmark::router::Router,
    }

    impl #router_identifier {
      #new_methods

      pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.router.run().await
      }

      pub fn router(&mut self) -> &mut ::windmark::router::Router {
        &mut self.router
      }
    }
  };

  output.into()
}
