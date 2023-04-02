// This file is part of Windmark <https://github.com/gemrest/windmark>.
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
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
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

//! Content and response handlers

#[cfg(feature = "response-macros")]
mod macros;

macro_rules! response {
  ($name:ident, $status:expr) => {
    pub fn $name<S>(content: S) -> Self
    where S: Into<String> + AsRef<str> {
      Self::new($status, content.into())
    }
  };
}

/// The content and response type a handler should reply with.
#[derive(Clone)]
pub struct Response {
  pub status:        i32,
  pub mime:          Option<String>,
  pub content:       String,
  pub character_set: Option<String>,
  pub language:      Option<String>,
}

impl Response {
  response!(input, 10);

  response!(sensitive_input, 11);

  response!(temporary_redirect, 30);

  response!(permanent_redirect, 31);

  response!(temporary_failure, 40);

  response!(server_unavailable, 41);

  response!(cgi_error, 42);

  response!(proxy_error, 43);

  response!(slow_down, 44);

  response!(permanent_failure, 50);

  response!(not_found, 51);

  response!(gone, 52);

  response!(proxy_refused, 53);

  response!(bad_request, 59);

  response!(client_certificate_required, 60);

  response!(certificate_not_authorised, 61);

  response!(certificate_not_valid, 62);

  #[allow(clippy::needless_pass_by_value)]
  pub fn success(content: impl ToString) -> Self {
    Self::new(20, content.to_string())
      .with_mime("text/gemini")
      .with_language("en")
      .with_character_set("utf-8")
      .clone()
  }

  #[must_use]
  pub fn binary_success(
    content: &[u8],
    mime: impl Into<String> + AsRef<str>,
  ) -> Self {
    Self::new(21, String::from_utf8_lossy(content))
      .with_mime(mime)
      .clone()
  }

  #[cfg(feature = "auto-deduce-mime")]
  #[must_use]
  pub fn binary_success_auto(content: &[u8]) -> Self {
    Self::new(22, String::from_utf8_lossy(content))
      .with_mime(&tree_magic::from_u8(&*content))
      .clone()
  }

  #[must_use]
  pub fn new(status: i32, content: impl Into<String> + AsRef<str>) -> Self {
    Self {
      status,
      mime: None,
      content: content.into(),
      character_set: None,
      language: None,
    }
  }

  pub fn with_mime(
    &mut self,
    mime: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.mime = Some(mime.into());

    self
  }

  pub fn with_character_set(
    &mut self,
    character_set: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.character_set = Some(character_set.into());

    self
  }

  pub fn with_language(
    &mut self,
    language: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.language = Some(language.into());

    self
  }
}
