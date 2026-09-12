use syn::{punctuated::Punctuated, Meta, Token};

mod fields;
mod methods;
mod parser;

pub use fields::fields;
pub use methods::methods;

fn conditional_attributes(
  attributes: &[syn::Attribute],
) -> Vec<syn::Attribute> {
  attributes
    .iter()
    .filter_map(|attribute| {
      conditional_meta(&attribute.meta).map(|meta| syn::parse_quote!(#[#meta]))
    })
    .collect()
}

fn conditional_meta(meta: &syn::Meta) -> Option<syn::Meta> {
  if meta.path().is_ident("cfg") {
    return Some(meta.clone());
  }

  let syn::Meta::List(list) = meta else {
    return None;
  };

  if !list.path.is_ident("cfg_attr") {
    return None;
  }

  let arguments = list
    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
    .ok()?;
  let mut arguments = arguments.into_iter();
  let predicate = arguments.next()?;
  let conditions: Vec<_> = arguments
    .filter_map(|meta| conditional_meta(&meta))
    .collect();

  if conditions.is_empty() {
    None
  } else {
    Some(syn::parse_quote!(cfg_attr(#predicate, #(#conditions),*)))
  }
}
