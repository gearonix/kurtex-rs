use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use std::{env, future};

use deno_core::futures::channel::mpsc;
use deno_core::futures::{SinkExt, StreamExt};
use deno_graph::{ModuleGraph, ModuleSpecifier};
use notify::{INotifyWatcher, Watcher};

use crate::reporter::Reporter;
use crate::runner::collector::{RunnerCollectorContext, TestRunnerConfig};
use crate::watcher::resolver::WatcherResolver;
use crate::watcher::watcher::{
  AsyncWatcherDebouncer, DebounceEventResult, DebouncedEventKind,
  DEBOUNCER_CHANNEL_BUFFER,
};
use crate::AnyResult;

pub mod resolver;
pub mod watcher;

// TODO: improve watcher options (according to graph),
// custom folder scope selection
pub async fn start_watcher(
  collector_ctx: &RunnerCollectorContext,
  _config: Rc<TestRunnerConfig>,
  module_graph: Rc<ModuleGraph>,
) -> AnyResult {
  let root_dir = env::current_dir().unwrap();

  watch_test_files(root_dir, &collector_ctx, module_graph).await?;

  Ok(())
}

async fn watch_test_files<P: AsRef<Path>>(
  path: P,
  ctx: &RunnerCollectorContext,
  module_graph: Rc<ModuleGraph>,
) -> AnyResult {
  let path = path.as_ref();
  let (mut watcher, outer_rx) = init_watcher()?;
  let mut resolver = WatcherResolver::new(module_graph);

  watcher.watch(path);

  outer_rx
    .for_each(|debounce_result| {
      match debounce_result {
        Ok(events) => events.iter().for_each(|ev| {
          if ev.kind == DebouncedEventKind::Update {
            let path = ev.path.clone();

            let changed_files = resolver.resolve_dependency_tests(path.clone());
            ctx.reporter.watcher_rerun(&changed_files, path);
            
            
          }
        }),
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
