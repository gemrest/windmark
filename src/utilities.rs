//! Utilities to make cumbersome tasks simpler

use std::collections::HashMap;

/// Extract the queries from a URL into a `HashMap`.
#[must_use]
pub fn queries_from_url(url: &url::Url) -> HashMap<String, String> {
  let mut queries = HashMap::new();

  for (key, value) in url.query_pairs() {
    queries.insert(key.to_string(), value.to_string());
  }

  queries
}

#[must_use]
pub fn params_to_hashmap(
  params: &matchit::Params<'_, '_>,
) -> HashMap<String, String> {
  params
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}
