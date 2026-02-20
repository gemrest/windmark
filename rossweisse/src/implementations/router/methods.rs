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
