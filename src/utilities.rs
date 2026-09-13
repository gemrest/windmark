//! This module provides URL query helpers.

use std::collections::HashMap;

/// Extract decoded query pairs into a map. The last value wins for repeated
/// keys; use `Url::query_pairs` directly to retain repeated keys.
///
/// These are form-style pairs: `+` represents a space. Gemini prompt input is
/// a single percent-encoded value and should be decoded without splitting it
/// into pairs or replacing literal plus signs.
#[must_use]
pub fn queries_from_url(url: &url::Url) -> HashMap<String, String> {
  let mut queries = HashMap::new();

  for (key, value) in url.query_pairs() {
    queries.insert(key.to_string(), value.to_string());
  }

  queries
}
