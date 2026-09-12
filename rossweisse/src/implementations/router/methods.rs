use proc_macro::TokenStream;

pub fn methods(_arguments: TokenStream, item: &syn::ItemImpl) -> TokenStream {
  let (routes, route_paths): (Vec<_>, Vec<_>) = item
    .items
    .iter()
    .filter_map(|item| {
      let syn::ImplItem::Fn(method) = item else {
        return None;
      };
      let route_attribute = method
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("route"))?;
      let is_index = route_attribute
        .parse_args::<syn::Ident>()
        .is_ok_and(|argument| argument == "index");
      let path = if is_index {
        "/".to_owned()
      } else {
        format!("/{}", method.sig.ident)
      };

      Some((&method.sig.ident, path))
    })
    .unzip();
  let (implementation_generics, type_generics, where_clause) =
    item.generics.split_for_impl();
  let name = &item.self_ty;

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
