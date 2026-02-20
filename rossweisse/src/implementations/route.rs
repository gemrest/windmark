use proc_macro::TokenStream;

pub fn route(_arguments: TokenStream, item: &syn::ItemFn) -> TokenStream {
  quote::quote! { #item }.into()
}
