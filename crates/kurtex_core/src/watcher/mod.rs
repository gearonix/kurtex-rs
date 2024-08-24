use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, future};

use deno_core::futures::channel::mpsc;
use deno_core::futures::StreamExt;
use notify::{INotifyWatcher, RecursiveMode, Watcher};

use crate::runner::collector::RunnerCollectorContext;
use crate::watcher::debouncer::{
  AsyncWatcherDebouncer, DebounceEventResult, DEBOUNCER_CHANNEL_BUFFER,
};
use crate::AnyResult;

pub mod debouncer;

#[derive(Debug, Clone)]
pub struct ChangedFile {
  paths: Vec<PathBuf>,
}

pub async fn start_watcher(
  collector_ctx: &RunnerCollectorContext,
) -> AnyResult {
  let root_dir = env::current_dir().unwrap();

  watch_test_files(root_dir).await?;

  Ok(())
}

async fn watch_test_files<P: AsRef<Path>>(path: P) -> AnyResult {
  let (mut debouncer, outer_rx) = init_watcher()?;

  debouncer.watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

  outer_rx
    .for_each(|debounce_result| {
      println!("debounce_result: {:?}", debounce_result);
      match debounce_result {
        Ok(events) => events.iter().for_each(|ev| println!("{:?}", ev)),
        Err(e) => println!("e: {:?}", e),
      }

      future::ready(())
    })
    .await;

  Ok(())
}

fn init_watcher(
) -> AnyResult<(AsyncWatcherDebouncer, mpsc::Receiver<DebounceEventResult>)> {
  let (outer_tx, outer_rx) =
    mpsc::channel::<DebounceEventResult>(DEBOUNCER_CHANNEL_BUFFER);

  let mut debouncer = AsyncWatcherDebouncer::<INotifyWatcher>::new(
    Duration::from_millis(1500),
    outer_tx,
  )?;

  Ok((debouncer, outer_rx))
} 