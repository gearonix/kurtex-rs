use deno_core::error::AnyError;
use deno_core::v8::Context;
use std::collections::HashMap;
use std::env;
use std::hash::Hash;
use std::path::PathBuf;

use crate::context::{ContextProvider, RUNTIME_CONFIG, TOKIO_RUNTIME};
use crate::error::CliError;
use crate::resolve_config::{resolve_kurtex_config, KurtexOptions};
use crate::runner::runner::Runner;

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
  pub module_cache: HashMap<PathBuf, String>,
  pub watch: bool,
  pub root: PathBuf,
  pub options: KurtexOptions,
}

impl Default for RuntimeConfig {
  fn default() -> Self {
    let options = resolve_kurtex_config()
      .map_err(|_e| CliError::FailedToReadConfigFile)
      .unwrap();

    RuntimeConfig {
      module_cache: HashMap::new(),
      watch: false,
      options,
      root: env::current_dir().unwrap(),
    }
  }
}

impl RuntimeConfig {
  pub fn enable_watch_mode(&mut self) {
    self.watch = true
  }
}

impl RuntimeManager {
  pub fn start(opts: &RuntimeOptions) -> Result<(), AnyError> {
    let root_dir = opts.root.clone();
    let mut __pending_modules__: HashMap<PathBuf, String> = HashMap::new();
    let tokio = ContextProvider::get(&TOKIO_RUNTIME).unwrap();

    ContextProvider::init_once(
      &RUNTIME_CONFIG,
      RuntimeConfig {
        module_cache: __pending_modules__,
        root: root_dir,
        ..RuntimeConfig::default()
      },
    );

    tokio.block_on(Runner::run_with_options())
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
