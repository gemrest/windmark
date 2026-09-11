use std::{
  future::Future,
  pin::pin,
  sync::{Arc, Mutex},
  task::Poll,
};
use windmark::{module::AsyncModule, router::Router};

struct Initializer(Arc<Mutex<Vec<&'static str>>>);

#[async_trait::async_trait]
impl AsyncModule for Initializer {
  async fn on_attach(&mut self, router: &mut Router) {
    self.0.lock().unwrap().push("started");
    #[cfg(feature = "tokio")]
    tokio::task::yield_now().await;
    #[cfg(feature = "async-std")]
    async_std::task::yield_now().await;
    router.set_port(0);
    self.0.lock().unwrap().push("finished");
  }
}

#[cfg_attr(feature = "tokio", tokio::test(flavor = "current_thread"))]
#[cfg_attr(feature = "async-std", async_std::test)]
async fn attachment_awaits_initialization_and_returns_the_router() {
  let events = Arc::new(Mutex::new(Vec::new()));
  let mut router = Router::new();
  let address = std::ptr::from_ref(&router);
  let mut attachment = pin!(router.attach_async(Initializer(events.clone())));

  assert!(events.lock().unwrap().is_empty());
  std::future::poll_fn(|context| {
    assert!(attachment.as_mut().poll(context).is_pending());

    Poll::Ready(())
  })
  .await;
  assert_eq!(*events.lock().unwrap(), ["started"]);

  let attached = attachment.await;

  assert_eq!(*events.lock().unwrap(), ["started", "finished"]);
  assert_eq!(std::ptr::from_ref(attached), address);
  attached.set_port(1965);
}
