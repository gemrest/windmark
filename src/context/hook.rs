use matchit::Params;
use openssl::x509::X509;
use url::Url;

use crate::context::Parameters;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct HookContext {
  pub peer_address: Option<std::net::SocketAddr>,
  pub url:          Url,
  pub parameters:   Option<Parameters>,
  pub certificate:  Option<X509>,
}

impl HookContext {
  #[must_use]
  pub fn new(
    peer_address: std::io::Result<std::net::SocketAddr>,
    url: Url,
    parameters: Option<Params<'_, '_>>,
    certificate: Option<X509>,
  ) -> Self {
    Self {
      peer_address: peer_address.ok(),
      url,
      parameters: parameters.map(|p| Parameters::from_parameters(&p)),
      certificate,
    }
  }
}
