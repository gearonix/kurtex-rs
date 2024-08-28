use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;
use std::{env, future};

use deno_core::futures::channel::mpsc;
use deno_core::futures::{SinkExt, StreamExt};
use deno_graph::{ModuleGraph, ModuleSpecifier};
use notify::{INotifyWatcher, Watcher};

use crate::reporter::Reporter;
use crate::runner::collector::{
  FileCollector, RunnerCollectorContext, TestRunnerConfig,
};
use crate::runner::runner::TestRunner;
use crate::watcher::resolver::WatcherResolver;
use crate::watcher::watcher::{
  AsyncWatcherDebouncer, DebounceEventResult, DebouncedEventKind,
  DEBOUNCER_CHANNEL_BUFFER,
};
use crate::AnyResult;

pub mod resolver;
pub mod watcher;

pub type RestartRunnerFn =
  dyn Fn(Vec<PathBuf>, &FileCollector, &mut TestRunner);

// TODO: improve watcher options (according to graph),
// custom folder scope selection
pub async fn start_watcher(
  module_graph: Rc<ModuleGraph>,
  trigger: Box<RestartRunnerFn>,
  file_collector: &FileCollector,
  test_runner: &mut TestRunner,
) -> AnyResult {
  let root_dir = env::current_dir().unwrap();

  watch_test_files(
    root_dir,
    module_graph,
    trigger,
    file_collector,
    test_runner,
  )
  .await?;

  Ok(())
}

async fn watch_test_files<P: AsRef<Path>>(
  path: P,
  module_graph: Rc<ModuleGraph>,
  trigger: Box<RestartRunnerFn>,
  file_collector: &FileCollector,
  test_runner: &mut TestRunner,
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

            let changed_files =
              resolver.resolve_dependency_tests(path.clone());
            // ctx.reporter.watcher_rerun(&changed_files, path);
            trigger(changed_files, file_collector, test_runner)
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

fn init_watcher() -> AnyResult<(
  AsyncWatcherDebouncer,
  mpsc::Receiver<DebounceEventResult>,
)> {
  let (outer_tx, outer_rx) =
    mpsc::channel::<DebounceEventResult>(DEBOUNCER_CHANNEL_BUFFER);

  let mut watcher = AsyncWatcherDebouncer::<INotifyWatcher>::new(
    Duration::from_millis(1500),
    outer_tx,
  );

  Ok((watcher, outer_rx))
}
