use proc_macro::TokenStream;

pub fn route(arguments: TokenStream, item: &syn::ItemFn) -> TokenStream {
  if !arguments.is_empty() {
    let argument = syn::parse_macro_input!(arguments as syn::Ident);

    if argument != "index" {
      return syn::Error::new_spanned(
        argument,
        "the only supported route argument is `index`",
      )
      .to_compile_error()
      .into();
    }
  }

  quote::quote! { #item }.into()
}
