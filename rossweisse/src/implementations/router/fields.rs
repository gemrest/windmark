use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;

pub fn fields(arguments: TokenStream, item: syn::ItemStruct) -> TokenStream {
  let field_initializers =
    syn::parse_macro_input!(arguments as super::parser::FieldInitializers);
  let visibility = item.vis;
  let router_identifier = item.ident;
  let named_fields = match item.fields {
    syn::Fields::Named(fields) => fields,

    syn::Fields::Unit =>
      syn::FieldsNamed {
        brace_token: syn::token::Brace::default(),
        named:       Punctuated::default(),
      },

    syn::Fields::Unnamed(_) =>
      panic!(
        "`#[rossweisse::router]` can only be used on `struct`s with named \
         fields or unit structs"
      ),
  };
  let new_method_fields = named_fields.named.iter().map(|field| {
    let name = &field.ident;
    let initializer: syn::Expr = field_initializers
      .0
      .iter()
      .find(|initializer| name.as_ref() == Some(&initializer.identifier))
      .map_or_else(
        || syn::parse_quote! { ::std::default::Default::default() },
        |initializer| initializer.expression.clone(),
      );

    quote! {
      #name: #initializer,
    }
  });
  let new_methods = quote! {
    fn _new() -> Self {
      Self {
        #(#new_method_fields)*
        router: ::windmark::router::Router::new(),
      }
    }
  };
  let output_fields = named_fields.named;
  let output = quote! {
    #visibility struct #router_identifier {
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
