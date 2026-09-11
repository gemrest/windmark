use syn::{
  parse::{self, Parse},
  punctuated::Punctuated,
  Expr,
  Ident,
  Token,
};

pub struct FieldInitializer {
  pub identifier: Ident,
  pub expression: Expr,
}

impl Parse for FieldInitializer {
  fn parse(input: parse::ParseStream<'_>) -> syn::Result<Self> {
    let identifier = input.parse()?;
    let _: Token![=] = input.parse()?;
    let expression = input.parse()?;

    Ok(Self {
      identifier,
      expression,
    })
  }
}

pub struct FieldInitializers(pub Punctuated<FieldInitializer, Token![,]>);

impl Parse for FieldInitializers {
  fn parse(input: parse::ParseStream<'_>) -> syn::Result<Self> {
    Punctuated::parse_terminated(input).map(Self)
  }
}
