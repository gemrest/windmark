//! This module provides the response type returned by handlers.

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

/// A response holds the status and content returned by a handler.
///
/// The server replaces responses whose status or metadata cannot be encoded
/// with a temporary failure. Text bodies use LF line endings, including a
/// final newline after a non-empty footer. Binary bodies are sent unchanged.
#[derive(Clone)]
#[non_exhaustive]
pub struct Response {
  pub status:  i32,
  pub mime:    Option<String>,
  pub content: String,

  /// The router emits these bytes verbatim instead of `content` for status
  /// `21`/`22`.
  pub binary_content: Option<Vec<u8>>,
  pub character_set:  Option<String>,
  pub languages:      Option<Vec<String>>,
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
  }

  #[must_use]
  pub fn binary_success(
    content: impl AsRef<[u8]>,
    mime: impl Into<String> + AsRef<str>,
  ) -> Self {
    let mut response = Self::new(21, String::new());

    response.binary_content = Some(content.as_ref().to_vec());

    response.with_mime(mime);

    response
  }

  #[cfg(feature = "auto-deduce-mime")]
  #[must_use]
  pub fn binary_success_auto(content: &[u8]) -> Self {
    let mut response = Self::new(22, String::new());

    response.with_mime(tree_magic_mini::from_u8(content));

    response.binary_content = Some(content.to_vec());

    response
  }

  #[must_use]
  pub fn new(status: i32, content: impl Into<String> + AsRef<str>) -> Self {
    Self {
      status,
      mime: None,
      content: content.into(),
      binary_content: None,
      character_set: None,
      languages: None,
    }
  }

  #[doc(hidden)]
  #[must_use]
  pub fn serialize_body(self, header: &str, footer: &str) -> Vec<u8> {
    if matches!(self.status, 21 | 22) {
      return self.binary_content.unwrap_or_default();
    }

    let mut body = Vec::with_capacity(self.body_capacity(header, footer));

    self.append_body(&mut body, header, footer);

    body
  }

  pub(crate) fn body_capacity(&self, header: &str, footer: &str) -> usize {
    match self.status {
      20 => header.len() + self.content.len() + footer.len() + 2,
      21 | 22 => self.binary_content.as_ref().map_or(0, Vec::len),
      _ => 0,
    }
  }

  pub(crate) fn append_body(
    &self,
    body: &mut Vec<u8>,
    header: &str,
    footer: &str,
  ) {
    match self.status {
      20 => {
        let text = self.mime.as_deref().is_none_or(|mime| {
          mime
            .split_once('/')
            .is_some_and(|(kind, _)| kind.eq_ignore_ascii_case("text"))
        });

        if text {
          append_text(body, header);
          append_text(body, &self.content);
        } else {
          body.extend_from_slice(header.as_bytes());
          body.extend_from_slice(self.content.as_bytes());
        }

        body.push(b'\n');

        if text {
          append_text(body, footer);

          if body.last() != Some(&b'\n') {
            body.push(b'\n');
          }
        } else {
          body.extend_from_slice(footer.as_bytes());
        }
      }

      21 | 22 =>
        if let Some(bytes) = &self.binary_content {
          body.extend_from_slice(bytes);
        },

      _ => {}
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

  pub fn with_languages<S>(&mut self, languages: impl AsRef<[S]>) -> &mut Self
  where S: Into<String> + AsRef<str> {
    self.languages = Some(
      languages
        .as_ref()
        .iter()
        .map(|s| s.as_ref().to_string())
        .collect::<Vec<String>>(),
    );

    self
  }
}

fn append_text(body: &mut Vec<u8>, mut text: &str) {
  while let Some(position) = text.find('\r') {
    body.extend_from_slice(&text.as_bytes()[..position]);
    body.push(b'\n');

    text = &text[position + 1..];
    text = text.strip_prefix('\n').unwrap_or(text);
  }

  body.extend_from_slice(text.as_bytes());
}

impl std::future::IntoFuture for Response {
  type IntoFuture = std::future::Ready<Self::Output>;
  type Output = Self;

  fn into_future(self) -> Self::IntoFuture { std::future::ready(self) }
}
