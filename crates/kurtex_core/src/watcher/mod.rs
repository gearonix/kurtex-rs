use std::future;
use std::future::Future;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use deno_core::futures::{SinkExt, StreamExt};
use deno_core::futures::channel::mpsc;
use deno_graph::ModuleGraph;
use notify::{INotifyWatcher, Watcher};

use crate::{AnyResult, EmitRuntimeOptions};
use crate::reporter::Reporter;
use crate::runner::collector::{
  RunnerCollectorContext, TestRunnerConfig,
};
use crate::watcher::resolver::WatcherResolver;
use crate::watcher::watcher::{
  AsyncWatcherDebouncer, DebouncedEventKind, DebounceEventResult,
  DEBOUNCER_CHANNEL_BUFFER,
};

pub mod resolver;
pub mod watcher;

pub type RestartRunnerFn =
  dyn Fn(Vec<PathBuf>, Rc<TestRunnerConfig>, Rc<EmitRuntimeOptions>);

// TODO: improve watcher options (according to graph),
// custom folder scope selection
pub async fn start_watcher(
  trigger: Box<RestartRunnerFn>,
  module_graph: Rc<ModuleGraph>,
  config: Rc<TestRunnerConfig>,
  emit_opts: Rc<EmitRuntimeOptions>,
  ctx: &RunnerCollectorContext,
) -> AnyResult {
  let path = config.root_dir.as_ref();
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

            if !changed_files.is_empty() {
              ctx.reporter.watcher_rerun(&changed_files, path);
              trigger(changed_files, config.clone(), emit_opts.clone())
            }
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
