use std::future::Future;
use std::pin::Pin;
use tokio::task;

pub async fn run_concurrently<T, O>(handles: Vec<T>) -> Vec<O>
where
  T: FnOnce() -> Pin<Box<dyn Future<Output = O>>>,
  O: 'static,
{
  let local_set = task::LocalSet::new();

  local_set
    .run_until(async move {
      let tasks: Vec<_> = handles
        .into_iter()
        .map(|handle| tokio::task::spawn_local(handle()))
        .collect();

      let mut output = Vec::new();

      for handle in tasks {
        output.push(handle.await.unwrap())
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

fn test() -> Pin<Box<dyn Future<Output = bool>>> {
  Box::pin(async { true })
}
