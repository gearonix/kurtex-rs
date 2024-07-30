use std::borrow::Cow;
use std::path::PathBuf;

use deno_core::error::AnyError;
use globwalk;

use crate::context::{ContextProvider, RUNTIME_CONFIG};
use crate::deno::module_resolver::{EsmModuleResolver, EsmResolverOptions};
use crate::runtime::runtime::RuntimeConfig;
use crate::utils::tokio::{create_pinned_future, run_in_parallel};
use crate::CLI_CONFIG;

const TEST_FILES_MAX_DEPTH: u32 = 25;

pub struct Runner;

impl Runner {
  pub async fn run_with_options() -> Result<(), AnyError> {
    let cli_config = ContextProvider::get(&CLI_CONFIG).unwrap();
    let runtime_config = ContextProvider::get(&RUNTIME_CONFIG).unwrap();
    let RuntimeConfig { options: runtime_opts, .. } = runtime_config;

    let mut esm_resolver =
      EsmModuleResolver::new(EsmResolverOptions { include_bindings: true });

    if cli_config.watch {
      // TODO
      // options.runtime.enable_watch_mode();
    }

    async fn print_hi_closure(file_path: PathBuf) {
      println!("print hi, {}", file_path.display());
    }

    let process_test_file = move |file_path: PathBuf| {
      create_pinned_future(print_hi_closure(file_path))
    };

    let processed_tasks = Self::collect_test_files(&runtime_config)?
      .map(process_test_file)
      .collect();

    if runtime_opts.parallel {
      run_in_parallel(processed_tasks).await;
    } else {
      for task in processed_tasks {
        task().await;
      }
    }

    Ok(())
  }

  fn collect_test_files(
    runtime_cfg: &RuntimeConfig,
  ) -> Result<impl Iterator<Item = std::path::PathBuf> + '_, AnyError> {
    let runtime_opts = &runtime_cfg.options;
    let walk_glob = |patterns: &Vec<Cow<'static, str>>| {
      globwalk::GlobWalkerBuilder::from_patterns(&runtime_cfg.root, patterns)
        .max_depth(TEST_FILES_MAX_DEPTH as usize)
        .build()
        .unwrap()
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
    };

    #[allow(unused_mut)]
    let mut included_cases = walk_glob(&runtime_opts.includes);
    let mut excluded_cases = walk_glob(&runtime_opts.excludes);

    Ok(included_cases.filter(move |included_path| {
      !excluded_cases.any(|excluded_path| excluded_path.eq(included_path))
    }))
  }

  pub fn run() {}
}
