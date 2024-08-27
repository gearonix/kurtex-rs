use crate::runner::collector::RunnerCollectorContext;
use crate::watcher::watcher::DebouncedEvent;
use std::path::PathBuf;
use crate::runtime::KurtexRuntime;

pub struct WatcherResolver {
  cached_files: Vec<PathBuf>,
  changed_tests: Vec<PathBuf>,
  runtime: KurtexRuntime
}

impl WatcherResolver {
  pub fn resolve_file(
    &mut self,
    file_path: PathBuf,
    context: &RunnerCollectorContext,
  ) {
    if context.file_map.contains_key(&file_path) {
      return;
    }
  }
}
