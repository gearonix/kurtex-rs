use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use deno_core::error::AnyError;
use globwalk;

use crate::context::{ContextProvider, RUNTIME_CONFIG};
use crate::deno::module_resolver::{EsmModuleResolver, EsmResolverOptions};
use crate::runtime::runtime::RuntimeConfig;
use crate::utils::tokio::{create_pinned_future, run_concurrently};
use crate::CLI_CONFIG;

const TEST_FILES_MAX_DEPTH: u32 = 25;

pub struct Runner;

impl Runner {
  pub async fn run_with_options() -> Result<(), AnyError> {
    let cli_config = ContextProvider::get(&CLI_CONFIG).unwrap();
    let runtime_config = ContextProvider::get(&RUNTIME_CONFIG).unwrap();
    let RuntimeConfig { options: runtime_opts, .. } = runtime_config;

    let now = std::time::Instant::now();
    let mut esm_resolver =
      EsmModuleResolver::new(EsmResolverOptions { include_bindings: true });
    let esm_resolver = Arc::new(Mutex::new(esm_resolver));
    if cli_config.watch {
      // TODO
      // options.runtime.enable_watch_mode();
    }

    let process_test_file = {
      move |esm_resolver: Arc<Mutex<EsmModuleResolver>>, file_path: PathBuf| async move {
        let mut resolver = esm_resolver.lock().unwrap();

        let module_id = resolver
          .process_esm_file(file_path.display().to_string())
          .await
          .unwrap();

        println!("module_id: {:?}", module_id);
      }
    };

    let processed_tasks = Self::collect_test_files(&runtime_config)?
      .map(move |file_path: PathBuf| {
        let esm_resolver = Arc::clone(&esm_resolver);
        create_pinned_future(process_test_file(esm_resolver, file_path))
      })
      .collect();

    if runtime_opts.parallel {
      run_concurrently(processed_tasks).await;
      println!("now: {:?}", now.elapsed());
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
