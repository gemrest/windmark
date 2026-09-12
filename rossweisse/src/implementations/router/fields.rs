use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;

pub fn fields(
  arguments: TokenStream,
  mut item: syn::ItemStruct,
) -> TokenStream {
  let field_initializers =
    syn::parse_macro_input!(arguments as super::parser::FieldInitializers);

  if matches!(item.fields, syn::Fields::Unit) {
    item.fields = syn::Fields::Named(syn::parse_quote!({}));
    item.semi_token = None;
  }

  let syn::Fields::Named(fields) = &mut item.fields else {
    return syn::Error::new_spanned(
      item.fields,
      "`#[rossweisse::router]` requires a struct with named fields or a unit \
       struct",
    )
    .to_compile_error()
    .into();
  };
  let mut supplied_fields = HashSet::new();

  for initializer in &field_initializers.0 {
    if !supplied_fields.insert(initializer.identifier.to_string()) {
      return syn::Error::new_spanned(
        &initializer.identifier,
        "duplicate field initialiser",
      )
      .to_compile_error()
      .into();
    }

    if !fields
      .named
      .iter()
      .any(|field| field.ident.as_ref() == Some(&initializer.identifier))
    {
      return syn::Error::new_spanned(
        &initializer.identifier,
        "unknown field initialiser",
      )
      .to_compile_error()
      .into();
    }
  }

  if let Some(field) = fields
    .named
    .iter()
    .find(|field| field.ident.as_ref().is_some_and(|name| name == "router"))
  {
    return syn::Error::new_spanned(
      field,
      "the field name `router` is reserved by Rossweisse",
    )
    .to_compile_error()
    .into();
  }

  let initializers = fields.named.iter().map(|field| {
    let name = &field.ident;
    let conditions = super::conditional_attributes(&field.attrs);
    let initializer: syn::Expr = field_initializers
      .0
      .iter()
      .find(|initializer| name.as_ref() == Some(&initializer.identifier))
      .map_or_else(
        || syn::parse_quote! { ::std::default::Default::default() },
        |initializer| initializer.expression.clone(),
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
  let conditions = super::conditional_attributes(&item.attrs);
  let (implementation_generics, type_generics, where_clause) =
    item.generics.split_for_impl();

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
  .into()
}
