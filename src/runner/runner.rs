use deno_core::error::AnyError;
use globwalk;
use globwalk::DirEntry;
use regex::Regex;

use crate::config::{get_or_init_cli_config, get_or_init_runtime_cfg};
use crate::runtime::runtime::RuntimeConfig;

const TEST_FILES_MAX_DEPTH: u32 = 25;

pub struct Runner;

impl Runner {
  pub fn run_with_options() -> Result<(), AnyError> {
    let cli_config = get_or_init_cli_config(None);
    let runtime_config = get_or_init_runtime_cfg(None);

    let RuntimeConfig { options: runtime_opts, root, .. } = runtime_config;

    if cli_config.watch {
      // TODO
      // options.runtime.enable_watch_mode();
    }

    Self::collect_test_files(&runtime_config)?.for_each(|file_path| {
      println!("file_path: {:?}", file_path);
    });

    Ok(())
  }

  fn collect_test_files(
    runtime_cfg: &RuntimeConfig,
  ) -> Result<impl Iterator<Item = DirEntry> + '_, AnyError> {
    let runtime_opts = &runtime_cfg.options;

    let walker = globwalk::GlobWalkerBuilder::from_patterns(
      &runtime_cfg.root,
      &runtime_opts.includes,
    )
    .max_depth(TEST_FILES_MAX_DEPTH as usize)
    .build()?
    .into_iter()
    .filter_map(Result::ok);

    let test_files = walker.filter(|dir_entry| {
      let walker_path = dir_entry.path().to_string_lossy();

      let is_excluded = runtime_opts.excludes.iter().any(|excluded_path| {
        let excluded = runtime_cfg.root.join(excluded_path.as_ref());
        let exclude_re =
          Regex::new(excluded.to_string_lossy().as_ref()).unwrap();

        
        println!("exclude_re: {:?}", exclude_re);
        println!("excluded: {:?}", excluded);
        
        exclude_re.is_match(walker_path.as_ref())
      });

      !is_excluded
    });

    Ok(test_files)
  }

  pub fn run() {}
}
