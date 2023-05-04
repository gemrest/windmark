// This file is part of Windmark <https://github.com/gemrest/windmark>.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Copyright (C) 2022-2023 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

use syn::parse::{self, Parse};

pub struct FieldInitializer<T: Parse> {
  pub ident: syn::Ident,
  #[allow(unused)]
  eq_token:  syn::Token![=],
  pub expr:  T,
}

impl<T: Parse> parse::Parse for FieldInitializer<T> {
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
