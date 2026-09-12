use super::{connections, RequestHandler, TcpListener};
use openssl::ssl::SslAcceptor;
use std::{future::Future, io, net::SocketAddr, sync::Arc};

/// A server owns a bound listener and the configuration used to serve it.
///
/// Create a server with `Router::bind`. Running it consumes the server;
/// dropping it before it runs releases the listener. Dropping its running
/// future also cancels active connection tasks.
pub struct Server {
  pub(super) listener: TcpListener,
  pub(super) handler:  Arc<RequestHandler>,
  pub(super) acceptor: Arc<SslAcceptor>,
}

impl Server {
  /// Return the listener's local address, including its assigned port.
  ///
  /// # Errors
  ///
  /// This method returns an error if the operating system cannot retrieve
  /// the listener's address.
  pub fn local_addr(&self) -> io::Result<SocketAddr> {
    self.listener.local_addr()
  }

  /// Serve requests until an interrupt arrives under Tokio, then drain
  /// connections for the configured period. Under `async-std`, serve until
  /// the process terminates; use `run_until` for controlled shutdown.
  pub async fn run(self) {
    #[cfg(feature = "tokio")]
    let shutdown = async {
      let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(feature = "async-std")]
    let shutdown = std::future::pending();

    self.run_until(shutdown).await;
  }

  /// Serve requests until `shutdown` completes, then drain connections for
  /// the configured period and cancel any remaining handler futures.
  pub async fn run_until(self, shutdown: impl Future<Output = ()>) {
    info!("windmark is listening for connections");
    connections::serve(self.listener, self.handler, self.acceptor, shutdown)
      .await;
  }
}
