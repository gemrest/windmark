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

fn normalize_pattern(path: &str) -> String {
  let mut normalized = String::with_capacity(path.len());

  for segment in path.split_inclusive('/') {
    let parameter = segment.find([':', '*']).unwrap_or(segment.len());

    normalized.push_str(&normalize_path(&segment[..parameter], None));
    normalized.push_str(&segment[parameter..]);
  }

  normalized
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
  let mut folded = String::with_capacity(path.len());

  for segment in path.split_inclusive('/') {
    let parameter = segment.find([':', '*']).unwrap_or(segment.len());

    folded.push_str(&segment[..parameter].to_ascii_lowercase());
    folded.push_str(&segment[parameter..]);
  }

  folded
}
