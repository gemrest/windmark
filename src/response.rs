//! This module provides the response type returned by handlers.

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
  status:        i32,
  payload:       Payload,
  mime:          Option<String>,
  character_set: Option<String>,
  languages:     Option<Vec<String>>,
}

#[derive(Clone)]
enum Payload {
  Text(String),
  Binary(Vec<u8>),
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

    response.payload = Payload::Binary(content.as_ref().to_vec());

    response.with_mime(mime);

    response
  }

  #[cfg(feature = "auto-deduce-mime")]
  #[must_use]
  pub fn binary_success_auto(content: &[u8]) -> Self {
    let mut response = Self::new(22, String::new());

    response.with_mime(tree_magic_mini::from_u8(content));

    response.payload = Payload::Binary(content.to_vec());

    response
  }

  #[must_use]
  pub fn new(status: i32, content: impl Into<String> + AsRef<str>) -> Self {
    Self {
      status,
      mime: None,
      payload: if matches!(status, 21 | 22) {
        Payload::Binary(Vec::new())
      } else {
        Payload::Text(content.into())
      },
      character_set: None,
      languages: None,
    }
  }

  #[doc(hidden)]
  #[must_use]
  pub fn serialize_body(self, header: &str, footer: &str) -> Vec<u8> {
    if let Payload::Binary(bytes) = self.payload {
      return bytes;
    }

    let mut body = Vec::with_capacity(self.body_capacity(header, footer));

    self.append_body(&mut body, header, footer);

    body
  }

  pub(crate) const fn body_capacity(
    &self,
    header: &str,
    footer: &str,
  ) -> usize {
    match &self.payload {
      Payload::Text(content) if self.status == 20 =>
        header.len() + content.len() + footer.len() + 2,
      Payload::Binary(bytes) => bytes.len(),
      Payload::Text(_) => 0,
    }
  }

  pub(crate) fn append_body(
    &self,
    body: &mut Vec<u8>,
    header: &str,
    footer: &str,
  ) {
    match &self.payload {
      Payload::Text(content) if self.status == 20 => {
        let text = self.mime.as_deref().is_none_or(|mime| {
          mime
            .split_once('/')
            .is_some_and(|(kind, _)| kind.eq_ignore_ascii_case("text"))
        });

        if text {
          append_text(body, header);
          append_text(body, content);
        } else {
          body.extend_from_slice(header.as_bytes());
          body.extend_from_slice(content.as_bytes());
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
      Payload::Binary(bytes) => body.extend_from_slice(bytes),
      Payload::Text(_) => {}
    }
  }

  /// Return the status supplied to the constructor, including internal binary
  /// statuses 21 and 22. Replace the response to change its status or payload
  /// kind.
  #[must_use]
  pub const fn status(&self) -> i32 { self.status }

  /// Return text content or header metadata, or `None` for binary responses.
  #[must_use]
  pub fn content(&self) -> Option<&str> {
    match &self.payload {
      Payload::Text(content) => Some(content),
      Payload::Binary(_) => None,
    }
  }

  /// Borrow text content or header metadata for editing.
  pub const fn content_mut(&mut self) -> Option<&mut String> {
    match &mut self.payload {
      Payload::Text(content) => Some(content),
      Payload::Binary(_) => None,
    }
  }

  /// Return the binary payload, or `None` for text and metadata responses.
  #[must_use]
  pub fn binary_content(&self) -> Option<&[u8]> {
    match &self.payload {
      Payload::Binary(bytes) => Some(bytes),
      Payload::Text(_) => None,
    }
  }

  /// Borrow the binary payload for editing or replacing its owned buffer.
  pub const fn binary_content_mut(&mut self) -> Option<&mut Vec<u8>> {
    match &mut self.payload {
      Payload::Binary(bytes) => Some(bytes),
      Payload::Text(_) => None,
    }
  }

  /// Return the response's media type override. Explicit character-set and
  /// language setters override parameters included in this value.
  #[must_use]
  pub fn mime(&self) -> Option<&str> { self.mime.as_deref() }

  /// Borrow the media type override; set it to `None` to restore the default.
  pub const fn mime_mut(&mut self) -> &mut Option<String> { &mut self.mime }

  /// Return the response's character set override.
  #[must_use]
  pub fn character_set(&self) -> Option<&str> { self.character_set.as_deref() }

  /// Borrow the character set override; `None` preserves a MIME parameter or,
  /// for string responses without one, uses the router's default.
  pub const fn character_set_mut(&mut self) -> &mut Option<String> {
    &mut self.character_set
  }

  /// Return the response's language override, including an explicitly empty
  /// list.
  #[must_use]
  pub fn languages(&self) -> Option<&[String]> { self.languages.as_deref() }

  /// Borrow the language override; `None` preserves a MIME parameter or,
  /// for string responses without one, uses the router's default.
  pub const fn languages_mut(&mut self) -> &mut Option<Vec<String>> {
    &mut self.languages
  }

  /// Set the media type and its parameters. Explicit character-set and
  /// language overrides take precedence; router defaults fill omitted
  /// parameters on string responses. Invalid metadata produces a temporary
  /// failure when the server sends the response.
  pub fn with_mime(
    &mut self,
    mime: impl Into<String> + AsRef<str>,
  ) -> &mut Self {
    self.mime = Some(mime.into());

    self
  }

  /// Declare the payload's character set without transcoding it. String
  /// responses require UTF-8; use a binary response for pre-encoded content.
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
