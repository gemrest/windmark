use matchit::Params;
use openssl::x509::X509;
use url::Url;

mod parameters;

pub use parameters::Parameters;

#[derive(Clone)]
pub struct RouteContext {
  pub peer_address: Option<std::net::SocketAddr>,
  pub url:          Url,
  pub parameters:   Parameters,
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
      parameters: Parameters::from_parameters(parameters),
      certificate,
    }
  }
}

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

#[derive(Clone)]
pub struct ErrorContext {
  pub peer_address: Option<std::net::SocketAddr>,
  pub url:          Url,
  pub certificate:  Option<X509>,
}

impl ErrorContext {
  #[must_use]
  pub fn new(
    peer_address: std::io::Result<std::net::SocketAddr>,
    url: Url,
    certificate: Option<X509>,
  ) -> Self {
    Self {
      peer_address: peer_address.ok(),
      url,
      certificate,
    }
  }
}
