use std::future::Future;
use std::pin::Pin;

pub async fn run_in_parallel<T>(handles: Vec<T>) -> Vec<()>
where
  T: FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>>,
{
  let tasks: Vec<_> =
    handles.into_iter().map(|handle| tokio::spawn(handle())).collect();

  let mut output = Vec::new();

  for handle in tasks {
    output.push(handle.await.unwrap())
  }

  output
}

pub fn create_pinned_future<F>(
  fut: F,
) -> impl FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>>
where
  F: 'static + Future<Output = ()> + Send,
{
  move || Box::pin(fut)
}

fn test() -> Pin<Box<dyn Future<Output = bool>>> {
  Box::pin(async { true })
}
