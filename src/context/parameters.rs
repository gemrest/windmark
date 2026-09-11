#[derive(Clone, Default)]
pub struct Parameters(Vec<(String, String)>);

impl Parameters {
  pub(crate) fn from_parameters(parameters: &matchit::Params<'_, '_>) -> Self {
    Self(
      parameters
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
    )
  }

  pub(crate) fn from_lookup(
    parameters: &matchit::Params<'_, '_>,
    lookup: &str,
    original: &str,
  ) -> Self {
    Self(
      parameters
        .iter()
        .map(|(name, value)| {
          // Matchit borrows each value from the lookup path. ASCII case folding
          // preserves byte offsets, so the original path has the same spans.
          let start = value.as_ptr().addr() - lookup.as_ptr().addr();
          let end = start + value.len();

          (name.to_owned(), original[start..end].to_owned())
        })
        .collect(),
    )
  }

  #[must_use]
  pub fn get(&self, key: &str) -> Option<&str> {
    self
      .0
      .iter()
      .find(|(k, _)| k == key)
      .map(|(_, v)| v.as_str())
  }

  #[must_use]
  pub const fn is_empty(&self) -> bool { self.0.is_empty() }

  pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
    self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
  }
}
