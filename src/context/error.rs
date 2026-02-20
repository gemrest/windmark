use openssl::x509::X509;
use url::Url;

#[allow(clippy::module_name_repetitions)]
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
