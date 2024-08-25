use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use std::{env, future};

use deno_core::futures::channel::mpsc;
use deno_core::futures::{SinkExt, StreamExt};
use notify::{INotifyWatcher, RecursiveMode, Watcher};

use crate::runner::collector::{RunnerCollectorContext, TestRunnerConfig};
use crate::watcher::watcher::{
  AsyncWatcherDebouncer, DebounceEventResult, InnerEvent,
  DEBOUNCER_CHANNEL_BUFFER,
};
use crate::AnyResult;

pub mod resolver;
pub mod watcher;

// TODO: improve watcher options,
// custom folder scope selection
pub async fn start_watcher(
  _collector_ctx: &RunnerCollectorContext,
  _config: Rc<TestRunnerConfig>,
) -> AnyResult {
  let root_dir = env::current_dir().unwrap();

  watch_test_files(root_dir).await?;

  Ok(())
}

async fn watch_test_files<P: AsRef<Path>>(path: P) -> AnyResult {
  let path = path.as_ref();
  let (mut watcher, outer_rx) = init_watcher()?;

  watcher.watch(path);

  outer_rx
    .for_each(|debounce_result| {
      match debounce_result {
        Ok(events) => events.iter().for_each(|ev| println!("{:?}", ev)),
        Err(err) => {
          eprintln!("Watcher: error while processing events {:?}", err);
          watcher.close();
        }
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

  let mut watcher = AsyncWatcherDebouncer::<INotifyWatcher>::new(
    Duration::from_millis(1500),
    outer_tx,
  );

  Ok((watcher, outer_rx))
}
