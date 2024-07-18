use std::collections::HashMap;
use std::hash::Hash;
use std::path::PathBuf;

use crate::config::get_or_init_runtime_cfg;

#[derive(Debug)]
pub struct RuntimeManager;

#[derive(Debug, Clone)]
pub struct RuntimeOptions<'a> {
  pub root: &'a PathBuf,
  pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct KurtexConfigOptions {
  includes: Vec<String>,
  excludes: Vec<String>,
  global: bool,
  jsdom: bool,
  parallel: bool,
  update: bool,
  watch: bool,
  root: String,
}

#[derive(Debug)]
pub struct RuntimeConfig {
  // server
  module_cache: HashMap<PathBuf, String>,
}

impl Default for RuntimeConfig {
  fn default() -> Self {
    RuntimeConfig { module_cache: HashMap::new() }
  }
}

impl RuntimeManager {
  pub fn start(opts: &RuntimeOptions) {
    let root_dir = &opts.root;

    let mut __pending_modules__: HashMap<PathBuf, String> = HashMap::new();

    get_or_init_runtime_cfg(Some(RuntimeConfig {
      module_cache: __pending_modules__,
    }));

    // Self::execute_files(opts)
  }

  // pub fn execute_files(opts: &RuntimeOptions) {
  //   let result = Vec::new();
  //
  //   for file in opts.files {}
  //
  //   result
  // }
  //
  // fn cached_request(file_path: &PathBuf, callstack: Vec<String>) {
  //   let RuntimeConfig { module_cache, .. } = get_or_init_runtime_cfg(None);
  //
  //   if (module_cache.contains_key(file_path)) {
  //     return module_cache.get(file_path);
  //   }
  //
  //   // module_cache.insert(file_path)
  // }
  //
  // fn direct_request(file_path: &PathBuf, callstack: &mut Vec<PathBuf>) {
  //   callstack.push(file_path.clone());
  // }
}
