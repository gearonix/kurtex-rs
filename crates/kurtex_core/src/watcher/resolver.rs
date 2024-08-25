use std::path::PathBuf;

pub struct WatcherResolver {
  cached_files: Vec<PathBuf>,
  changed_tests: Vec<PathBuf>
}

impl WatcherResolver {}
