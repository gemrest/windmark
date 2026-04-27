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
  pub status:         i32,
  pub mime:           Option<String>,
  pub content:        String,
  /// Raw body for status `21`/`22`; the router emits these bytes verbatim
  /// instead of `content`.
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
    let mut response = Self::new(20, content.to_string());

    response
      .with_mime("text/gemini")
      .with_languages(["en"])
      .with_character_set("utf-8");

    response
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
    match self.status {
      20 => {
        let mut body = Vec::with_capacity(
          header.len() + self.content.len() + footer.len() + 1,
        );

        body.extend_from_slice(header.as_bytes());
        body.extend_from_slice(self.content.as_bytes());
        body.push(b'\n');
        body.extend_from_slice(footer.as_bytes());

        body
      }
      21 | 22 => self.binary_content.unwrap_or_default(),
      _ => Vec::new(),
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

impl std::future::IntoFuture for Response {
  type IntoFuture = std::future::Ready<Self::Output>;
  type Output = Self;

  fn into_future(self) -> Self::IntoFuture { std::future::ready(self) }
}
