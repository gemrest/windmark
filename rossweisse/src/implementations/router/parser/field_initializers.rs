use syn::parse::{self, Parse};

use super::field_initializer::FieldInitializer;

pub struct FieldInitializers<T: Parse>(pub Vec<FieldInitializer<T>>);

impl<T: Parse> Parse for FieldInitializers<T> {
  fn parse(input: parse::ParseStream<'_>) -> syn::Result<Self> {
    Ok(Self(syn::punctuated::Punctuated::<FieldInitializer<T>, syn::Token![,]>::parse_terminated(input)?.into_iter().collect()))
  }
}
