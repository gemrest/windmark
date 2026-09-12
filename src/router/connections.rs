use super::{RequestHandler, Stream, TcpStream};
use futures_util::{
  future::{AbortHandle, Abortable, BoxFuture},
  stream::FuturesUnordered,
  FutureExt,
  StreamExt,
};
use openssl::ssl::{Ssl, SslAcceptor};
use std::{
  future::Future,
  io,
  num::NonZeroUsize,
  panic::AssertUnwindSafe,
  pin::Pin,
  sync::Arc,
  task::{Context, Poll},
  time::Duration,
};
#[cfg(feature = "tokio")]
use tokio::io::AsyncWriteExt;

#[cfg(feature = "tokio")]
pub(super) type TcpListener = tokio::net::TcpListener;
#[cfg(feature = "async-std")]
pub(super) type TcpListener = async_std::net::TcpListener;

/// These limits apply to each server when it starts.
///
/// Defaults preserve unlimited connections and stage durations. A zero drain
/// timeout cancels active connections immediately when shutdown is requested.
/// Cancellation drops unfinished handler futures; it cannot undo their effects
/// or interrupt synchronous code that blocks a runtime worker.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct ConnectionLimits {
  /// This cap includes connections waiting for their TLS handshake.
  pub max_connections:   Option<NonZeroUsize>,
  /// This deadline covers the complete TLS handshake.
  pub handshake_timeout: Option<Duration>,
  /// This deadline covers the complete request, including its CRLF terminator.
  pub request_timeout:   Option<Duration>,
  /// This deadline covers writing a response to the client.
  pub write_timeout:     Option<Duration>,
  /// This deadline covers sending the TLS close notification.
  pub shutdown_timeout:  Option<Duration>,
  /// Active connections may finish during this period after acceptance stops.
  pub drain_timeout:     Duration,
}

pub(super) async fn with_timeout<F: Future>(
  duration: Option<Duration>,
  future: F,
) -> io::Result<F::Output> {
  let Some(duration) = duration else {
    return Ok(future.await);
  };
  #[cfg(feature = "tokio")]
  let result = tokio::time::timeout(duration, future).await;
  #[cfg(feature = "async-std")]
  let result = async_std::future::timeout(duration, future).await;

  result.map_err(|_| {
    io::Error::new(io::ErrorKind::TimedOut, "connection deadline elapsed")
  })
}

struct ConnectionTask {
  completion: BoxFuture<'static, ()>,
  abort:      AbortHandle,
}

impl Future for ConnectionTask {
  type Output = ();

  fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
    self.completion.as_mut().poll(context)
  }
}

impl Drop for ConnectionTask {
  fn drop(&mut self) { self.abort.abort(); }
}

fn spawn_connection(
  handler: Arc<RequestHandler>,
  acceptor: Arc<SslAcceptor>,
  stream: TcpStream,
) -> ConnectionTask {
  let (abort, registration) = AbortHandle::new_pair();
  let connection = async move {
    let ssl = Ssl::new(acceptor.context())?;
    let mut stream = Stream::new(ssl, stream)?;
    let limits = handler.connection_limits;

    with_timeout(limits.handshake_timeout, Pin::new(&mut stream).accept())
      .await??;

    if let Err(error) = handler.handle(&mut stream).await {
      error!("request handling error: {error}");

      // An incomplete write must not look like a completed response to clients.
      return Ok(());
    }

    #[cfg(feature = "tokio")]
    let shutdown = stream.shutdown();
    #[cfg(feature = "async-std")]
    let shutdown = std::future::poll_fn(|context| {
      async_std::io::Write::poll_close(Pin::new(&mut stream), context)
    });

    with_timeout(limits.shutdown_timeout, shutdown).await??;

    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
  };
  let connection = async move {
    match AssertUnwindSafe(connection).catch_unwind().await {
      Ok(Ok(())) => {}
      Ok(Err(error)) => error!("connection error: {error}"),
      Err(_) => error!("connection task panicked"),
    }
  };
  let connection = async move {
    let _ = Abortable::new(connection, registration).await;
  };
  #[cfg(feature = "tokio")]
  let task = tokio::spawn(connection);
  #[cfg(feature = "async-std")]
  let task = async_std::task::spawn(connection);
  let completion = async move {
    #[cfg(feature = "tokio")]
    if let Err(error) = task.await {
      error!("connection task failed: {error}");
    }

    #[cfg(feature = "async-std")]
    task.await;
  }
  .boxed();

  ConnectionTask {
    completion,
    abort,
  }
}

pub(super) async fn serve(
  listener: TcpListener,
  handler: Arc<RequestHandler>,
  acceptor: Arc<SslAcceptor>,
  shutdown: impl Future<Output = ()>,
) {
  let limits = handler.connection_limits;
  let mut connections = FuturesUnordered::new();
  let shutdown = shutdown.fuse();

  futures_util::pin_mut!(shutdown);

  loop {
    let can_accept = limits
      .max_connections
      .is_none_or(|maximum| connections.len() < maximum.get());
    let event = {
      let incoming = async {
        if !can_accept {
          std::future::pending::<()>().await;
        }

        listener.accept().await
      }
      .fuse();
      let completed = async {
        if connections.is_empty() {
          std::future::pending::<()>().await;
        }

        connections.next().await;
      }
      .fuse();

      futures_util::pin_mut!(incoming, completed);

      futures_util::select_biased! {
        () = shutdown => break,
        () = completed => None,
        connection = incoming => Some(connection),
      }
    };

    if let Some(connection) = event {
      match connection {
        Ok((stream, _)) =>
          connections.push(spawn_connection(
            handler.clone(),
            acceptor.clone(),
            stream,
          )),
        Err(error) => error!("TCP accept error: {error}"),
      }
    }
  }

  drop(listener);

  let drain = async { while connections.next().await.is_some() {} };
  let _ = with_timeout(Some(limits.drain_timeout), drain).await;
}
