use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{parse::Parser, punctuated::Punctuated, Error, MetaNameValue, Token};

pub fn expand(
  arguments: TokenStream,
  mut item: syn::ItemStruct,
) -> syn::Result<TokenStream> {
  let field_initializers =
    Punctuated::<MetaNameValue, Token![,]>::parse_terminated
      .parse(arguments)?;

  if matches!(item.fields, syn::Fields::Unit) {
    item.fields = syn::Fields::Named(syn::parse_quote!({}));
    item.semi_token = None;
  }

  let syn::Fields::Named(fields) = &mut item.fields else {
    return Err(Error::new_spanned(
      item.fields,
      "`#[rossweisse::router]` requires a struct with named fields or a unit \
       struct",
    ));
  };
  let mut supplied_fields = HashSet::new();

  for initializer in &field_initializers {
    let identifier = initializer.path.require_ident()?;

    if !supplied_fields.insert(identifier.to_string()) {
      return Err(Error::new_spanned(
        identifier,
        "duplicate field initialiser",
      ));
    }

    if !fields
      .named
      .iter()
      .any(|field| field.ident.as_ref() == Some(identifier))
    {
      return Err(Error::new_spanned(identifier, "unknown field initialiser"));
    }
  }

  if let Some(field) = fields
    .named
    .iter()
    .find(|field| field.ident.as_ref().is_some_and(|name| name == "router"))
  {
    return Err(Error::new_spanned(
      field,
      "the field name `router` is reserved by Rossweisse",
    ));
  }

  let initializers = fields.named.iter().map(|field| {
    let name = &field.ident;
    let conditions = crate::conditional_attributes(&field.attrs);
    let initializer: syn::Expr = field_initializers
      .iter()
      .find(|initializer| initializer.path.get_ident() == name.as_ref())
      .map_or_else(
        || syn::parse_quote! { ::std::default::Default::default() },
        |initializer| initializer.value.clone(),
      );

    quote! {
      #(#conditions)*
      #name: #initializer,
    }
  });
  let initialization = quote! {
    Self {
      #(#initializers)*
      router: ::windmark::router::Router::new(),
    }
  };

  fields
    .named
    .push(syn::parse_quote!(router: ::windmark::router::Router));

  let name = &item.ident;
  let conditions = crate::conditional_attributes(&item.attrs);
  let (implementation_generics, type_generics, where_clause) =
    item.generics.split_for_impl();

  Ok(
    quote! {
      #item

      #(#conditions)*
      impl #implementation_generics #name #type_generics #where_clause {
        fn _new() -> Self { #initialization }

        pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
          self.router.run().await
        }

        pub fn router(&mut self) -> &mut ::windmark::router::Router {
          &mut self.router
        }
      }
    }
    .into(),
  )
}
