use proc_macro::TokenStream;
use quote::quote;

pub fn methods(_arguments: TokenStream, item: &syn::ItemImpl) -> TokenStream {
  let mut mounts = Vec::new();

  for member in &item.items {
    let syn::ImplItem::Fn(method) = member else {
      continue;
    };
    let name = &method.sig.ident;

    if ["new", "_new", "run", "router"]
      .iter()
      .any(|reserved| name == reserved)
    {
      return syn::Error::new_spanned(
        name,
        "this method name is reserved by Rossweisse",
      )
      .to_compile_error()
      .into();
    }

    let Some(route_attribute) = method.attrs.iter().find(|attribute| {
      attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "route")
    }) else {
      continue;
    };
    let is_index = route_attribute
      .parse_args::<syn::Ident>()
      .is_ok_and(|argument| argument == "index");
    let path = if is_index {
      "/".to_owned()
    } else {
      format!("/{name}")
    };
    let conditions = super::conditional_attributes(&method.attrs);

    mounts.push(quote! {
      #(#conditions)*
      router.router.mount(#path, |context| Self::#name(context));
    });
  }

  let (implementation_generics, _, where_clause) =
    item.generics.split_for_impl();
  let name = &item.self_ty;
  let conditions = super::conditional_attributes(&item.attrs);

  quote! {
    #item

    #(#conditions)*
    impl #implementation_generics #name #where_clause {
      pub fn new() -> Self {
        let mut router = Self::_new();

        #(#mounts)*

        router
      }
    }
  }
  .into()
}
