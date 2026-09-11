use crate::{context::Parameters, handler::RouteResponse};
use std::sync::Arc;

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
    path: String,
    handler: Arc<dyn RouteResponse>,
  ) -> Result<(), matchit::InsertError> {
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
    self.folded.as_ref().map_or_else(
      || self.exact.at(path).is_ok(),
      |routes| routes.at(&path.to_ascii_lowercase()).is_ok(),
    )
  }

  pub fn at(
    &self,
    path: &str,
  ) -> Result<MatchedRoute<'_>, matchit::MatchError> {
    if let Some(routes) = &self.folded {
      let lookup = path.to_ascii_lowercase();
      let matched = routes.at(&lookup)?;

      Ok(MatchedRoute {
        value:      matched.value,
        parameters: Parameters::from_lookup(&matched.params, &lookup, path),
      })
    } else {
      let matched = self.exact.at(path)?;

      Ok(MatchedRoute {
        value:      matched.value,
        parameters: Parameters::from_parameters(&matched.params),
      })
    }
  }
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
