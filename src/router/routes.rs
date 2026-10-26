use crate::{context::Parameters, handler::RouteResponse};
use std::{borrow::Cow, sync::Arc};

#[derive(Clone, Default)]
pub(super) struct Routes {
  exact:         matchit::Router<Arc<dyn RouteResponse>>,
  folded:        Option<matchit::Router<Arc<dyn RouteResponse>>>,
  registrations: Vec<(String, Arc<dyn RouteResponse>)>,
}

pub(super) struct MatchedRoute<'router> {
  pub value:      &'router Arc<dyn RouteResponse>,
  pub parameters: Parameters,
}

impl Routes {
  pub fn insert(
    &mut self,
    path: &str,
    handler: Arc<dyn RouteResponse>,
  ) -> Result<(), matchit::InsertError> {
    let path = normalize_pattern(path);
    let mut folded = self.folded.clone();

    if let Some(routes) = &mut folded {
      routes.insert(fold_pattern(&path), handler.clone())?;
    }

    self.exact.insert(path.clone(), handler.clone())?;
    self.registrations.push((path, handler));

    self.folded = folded;

    Ok(())
  }

  pub fn enable_case_insensitive(
    &mut self,
  ) -> Result<(), matchit::InsertError> {
    if self.folded.is_some() {
      return Ok(());
    }

    let mut routes = matchit::Router::new();

    for (path, handler) in &self.registrations {
      routes.insert(fold_pattern(path), handler.clone())?;
    }

    self.folded = Some(routes);

    Ok(())
  }

  pub fn disable_case_insensitive(&mut self) { self.folded = None; }

  pub fn contains(&self, path: &str) -> bool {
    let path = normalize_path(path, None);

    self.folded.as_ref().map_or_else(
      || self.exact.at(&path).is_ok(),
      |routes| routes.at(&path.to_ascii_lowercase()).is_ok(),
    )
  }

  pub fn at(
    &self,
    path: &str,
  ) -> Result<MatchedRoute<'_>, matchit::MatchError> {
    let mut offsets = Vec::new();
    let mut lookup = normalize_path(path, Some(&mut offsets));
    let routes = self.folded.as_ref().map_or(&self.exact, |routes| {
      lookup.to_mut().make_ascii_lowercase();

      routes
    });
    let matched = routes.at(&lookup)?;

    Ok(MatchedRoute {
      value:      matched.value,
      parameters: Parameters::from_lookup(
        &matched.params,
        &lookup,
        path,
        &offsets,
      ),
    })
  }
}

/// Rewrite segments written in the `:name` and `*name` parameter syntax of
/// matchit 0.7 into their braced form, or return `None` when there are none.
/// Matchit 0.9 treats those segments as literal text, so they would otherwise
/// mount without error and never match.
pub(super) fn braced_legacy_parameters(path: &str) -> Option<String> {
  if !path.split('/').any(is_legacy_parameter) {
    return None;
  }

  let segments = path.split('/').map(|segment| {
    if is_legacy_parameter(segment) {
      format!("{{{}}}", segment.strip_prefix(':').unwrap_or(segment))
    } else {
      segment.to_owned()
    }
  });

  Some(segments.collect::<Vec<_>>().join("/"))
}

fn is_legacy_parameter(segment: &str) -> bool {
  segment.len() > 1 && segment.starts_with([':', '*'])
}

fn normalize_pattern(path: &str) -> String {
  map_static_text(path, |text| normalize_path(text, None))
}

fn normalize_path<'path>(
  path: &'path str,
  mut offsets: Option<&mut Vec<usize>>,
) -> Cow<'path, str> {
  if !path.contains('%') {
    return Cow::Borrowed(path);
  }

  let bytes = path.as_bytes();
  let mut normalized = Vec::with_capacity(bytes.len());
  let mut position = 0;

  if let Some(offsets) = &mut offsets {
    offsets.reserve(bytes.len() + 1);
    offsets.push(0);
  }

  while position < bytes.len() {
    if bytes[position] == b'%' {
      if let Some(digits) = bytes.get(position + 1..position + 3) {
        if let (Some(high), Some(low)) = (
          (digits[0] as char).to_digit(16),
          (digits[1] as char).to_digit(16),
        ) {
          let decoded = u8::try_from(high * 16 + low)
            .expect("two hex digits fit in a byte");

          if decoded.is_ascii_alphanumeric() || b"-._~".contains(&decoded) {
            normalized.push(decoded);

            if let Some(offsets) = &mut offsets {
              offsets.push(position + 3);
            }
          } else {
            normalized.extend([
              b'%',
              digits[0].to_ascii_uppercase(),
              digits[1].to_ascii_uppercase(),
            ]);

            if let Some(offsets) = &mut offsets {
              offsets.extend([position + 1, position + 2, position + 3]);
            }
          }

          position += 3;

          continue;
        }
      }
    }

    normalized.push(bytes[position]);

    if let Some(offsets) = &mut offsets {
      offsets.push(position + 1);
    }

    position += 1;
  }

  Cow::Owned(
    String::from_utf8(normalized)
      .expect("normalising ASCII escapes preserves UTF-8"),
  )
}

fn fold_pattern(path: &str) -> String {
  map_static_text(path, |text| Cow::Owned(text.to_ascii_lowercase()))
}

/// Apply `transform` to the static text of a route pattern, copying `{name}`
/// and `{*name}` parameters through unchanged. Escaped braces (`{{` and `}}`)
/// are static text.
fn map_static_text(
  pattern: &str,
  transform: impl Fn(&str) -> Cow<'_, str>,
) -> String {
  let mut mapped = String::with_capacity(pattern.len());
  let mut remainder = pattern;

  while let Some(start) = find_parameter(remainder) {
    let end = remainder[start..]
      .find('}')
      .map_or(remainder.len(), |length| start + length + 1);

    mapped.push_str(&transform(&remainder[..start]));
    mapped.push_str(&remainder[start..end]);

    remainder = &remainder[end..];
  }

  mapped.push_str(&transform(remainder));

  mapped
}

fn find_parameter(pattern: &str) -> Option<usize> {
  let bytes = pattern.as_bytes();
  let mut position = 0;

  while position < bytes.len() {
    if bytes[position] == b'{' {
      if bytes.get(position + 1) != Some(&b'{') {
        return Some(position);
      }

      position += 1;
    }

    position += 1;
  }

  None
}
