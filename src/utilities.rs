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

/// Normalizes a path by removing all trailing slashes, unless it's the root
/// path "/".
///
/// # Examples
///
/// ```rust
/// assert_eq!(
///   windmark::utilities::normalize_path_slashes("/foo///"),
///   "/foo"
/// );
/// assert_eq!(windmark::utilities::normalize_path_slashes("/foo/"), "/foo");
/// assert_eq!(windmark::utilities::normalize_path_slashes("/foo"), "/foo");
/// assert_eq!(windmark::utilities::normalize_path_slashes("/"), "/");
/// ```
#[must_use]
pub fn normalize_path_slashes(path: &str) -> String {
  if path == "/" {
    return "/".to_string();
  }

  if path.ends_with('/') {
    path.trim_end_matches('/').to_string()
  } else {
    path.to_string()
  }
}
