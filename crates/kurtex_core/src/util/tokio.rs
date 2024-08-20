use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;

use tokio::runtime::Runtime;

use crate::error::{AnyError, AnyResult};

pub async fn run_concurrently<T, O>(handles: Vec<T>) -> Vec<O>
where
  T: FnOnce() -> Pin<Box<dyn Future<Output = Result<O, AnyError>>>>,
  O: 'static,
{
  let local_set = tokio::task::LocalSet::new();

  local_set
    .run_until(async move {
      let tasks: Vec<_> = handles
        .into_iter()
        .map(|handle| tokio::task::spawn_local(handle()))
        .collect();

      let mut output = Vec::new();

      for handle in tasks {
        let handle_result = handle.await.unwrap();
        output.push(handle_result.unwrap());
      }

      output
    })
    .await
}

pub fn create_pinned_future<F, O>(
  fut: F,
) -> impl FnOnce() -> Pin<Box<dyn Future<Output = O>>>
where
  F: 'static + Future<Output = O>,
{
  move || Box::pin(fut)
}

pub fn run_async<R>(
  f: impl Future<Output = AnyResult<R>>,
  runtime: Option<Runtime>,
) {
  let runtime = runtime.unwrap_or_else(|| {
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .expect("Failed to build a runtime")
  });

  runtime.block_on(f).expect("Failed to run the given task");

  let handle = runtime.spawn(async {
    tokio::task::yield_now().await;
  });
  _ = runtime.block_on(handle);

  let (tx, rx) = channel::<()>();
  let timeout = std::thread::spawn(move || {
    if rx.recv_timeout(Duration::from_secs(10))
      == Err(RecvTimeoutError::Timeout)
    {
      panic!("Failed to shut down the runtime in time");
    }
  });

  drop(runtime);
  drop(tx);
  _ = timeout.join();
}
