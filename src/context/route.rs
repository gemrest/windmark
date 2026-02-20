use std::collections::HashMap;

use matchit::Params;
use openssl::x509::X509;
use url::Url;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct RouteContext {
  pub peer_address: Option<std::net::SocketAddr>,
  pub url:          Url,
  pub parameters:   HashMap<String, String>,
  pub certificate:  Option<X509>,
}

impl RouteContext {
  #[must_use]
  pub fn new(
    peer_address: std::io::Result<std::net::SocketAddr>,
    url: Url,
    parameters: &Params<'_, '_>,
    certificate: Option<X509>,
  ) -> Self {
    Self {
      peer_address: peer_address.ok(),
      url,
      parameters: crate::utilities::params_to_hashmap(parameters),
      certificate,
    }
  }
}
