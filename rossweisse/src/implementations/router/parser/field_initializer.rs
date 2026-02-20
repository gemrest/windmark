use syn::parse::{self, Parse};

pub struct FieldInitializer<T: Parse> {
  pub ident: syn::Ident,
  #[allow(unused)]
  eq_token:  syn::Token![=],
  pub expr:  T,
}

impl<T: Parse> Parse for FieldInitializer<T> {
  fn parse(input: parse::ParseStream<'_>) -> syn::Result<Self> {
    let ident = input.parse()?;
    let eq_token = input.parse()?;
    let expr = input.parse()?;

    Ok(Self {
      ident,
      eq_token,
      expr,
    })
  }
}
