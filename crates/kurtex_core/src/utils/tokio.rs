use deno_core::error::AnyError;
use std::future::Future;
use std::pin::Pin;

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
