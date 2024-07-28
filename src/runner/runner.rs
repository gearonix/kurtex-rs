use crate::config::{get_or_init_cli_config, get_or_init_runtime_cfg};
use crate::runtime::runtime::RuntimeConfig;
use crate::CliConfig;
use deno_core::error::AnyError;
use globwalk;

const TEST_FILES_MAX_DEPTH: u32 = 25;

pub struct Runner;

pub struct RunnerOptions<'a> {
  cli: &'a CliConfig,
  runtime: &'a RuntimeConfig,
}

impl<'a> RunnerOptions<'a> {
  fn new(cli: &'a CliConfig, runtime: &'a RuntimeConfig) -> Self {
    RunnerOptions { cli, runtime }
  }
}

impl Runner {
  pub fn run_with_options() -> Result<(), AnyError> {
    let cli_config = get_or_init_cli_config(None);
    let runtime_config = get_or_init_runtime_cfg(None);

    let options = RunnerOptions::new(cli_config, runtime_config);

    if options.cli.watch {
      // TODO
      // options.runtime.enable_watch_mode();
    }

    let walker = globwalk::GlobWalkerBuilder::from_patterns(
      &runtime_config.root,
      &runtime_config.options.includes,
    )
    .max_depth(TEST_FILES_MAX_DEPTH as usize)
    .build()?
    .into_iter()
    .filter_map(Result::ok);

    for file in walker {
      println!("file: {:?}", file);
    }

    Ok(())
    // let test_files = glob()
  }

  pub fn run(opts: RunnerOptions) {}
}
