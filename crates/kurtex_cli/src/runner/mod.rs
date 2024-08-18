use std::path::PathBuf;

use anyhow::{Context, Error};
use clap::ArgMatches;
use kurtex_core::AnyResult;
use tokio::time;

use crate::settings;
use kurtex_core::config::loader::ConfigLoader;
use kurtex_core::runner::{RuntimeOptions, TestRunner, TestRunnerOptions};
use kurtex_core::util::tokio::run_async;
use kurtex_core::walk::{Extensions, Walk};

/// A trait for exposing functionality to the CLI.
pub trait Runner {
  type Options;

  fn new(matches: Self::Options) -> Self;
  fn run(self) -> AnyResult;
}

#[derive(Clone)]
pub struct CliRunner {
  options: ArgMatches,
}

pub const VALID_CONFIG_FILES: [&str; 2] = ["kurtex.config", "ktx.config"];

pub const VALID_EXTENSIONS: [&str; 8] =
  ["js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx"];

impl Runner for CliRunner {
  type Options = ArgMatches;

  fn new(options: Self::Options) -> Self {
    Self { options }
  }

  fn run(mut self) -> Result<(), Error> {
    let mut opts = self.options;
    let root_dir = opts.remove_one::<PathBuf>("root");
    let watch = opts.remove_one::<bool>("watch").unwrap();
    let globals = opts.remove_one::<bool>("globals").unwrap();
    let parallel = opts.remove_one::<bool>("parallel").unwrap();

    let config_path = opts.remove_one::<String>("config").unwrap();
    let mut config_path = PathBuf::from(config_path);

    let current_dir = std::env::current_dir().context("Unable to get CWD")?;
    let root_dir = root_dir.unwrap_or(current_dir);
    root_dir.join(&config_path).clone_into(&mut config_path);

    let now = time::Instant::now();
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();

    let config_path = if config_path.exists() {
      config_path
    } else {
      let mut paths = find_kurtex_config(&root_dir);
      let first_match = paths.drain(..1).next();

      first_match.unwrap()
    };

    let config_path_ = config_path.clone();
    let mut runner_options = TestRunnerOptions {
      watch,
      globals,
      config_path,
      root_dir,
      parallel,
      ..Default::default()
    };

    let config_loader = ConfigLoader::new(config_path_.display().to_string());

    run_async(
      async {
        let config_file = config_loader.load().await.unwrap();
        runner_options.adjust_config_file(config_file);
        let test_runner = TestRunner::new(
          runner_options,
          RuntimeOptions::new_from_snapshot(settings::RUNTIME_SNAPSHOT),
        );

        test_runner.run().await.unwrap();

        // TODO: remove
        Ok(())
      },
      Some(rt),
    );

    #[cfg(debug_assertions)]
    println!("Elapsed time: {:?} ms", now.elapsed().as_millis());

    Ok(())
  }
}

pub fn find_kurtex_config(root_dir: &PathBuf) -> Vec<PathBuf> {
  let mut paths = Walk::new(&VALID_CONFIG_FILES, &root_dir)
    .with_extensions(Extensions(VALID_EXTENSIONS.to_vec()))
    .build()
    .collect::<Vec<PathBuf>>();

  assert!(!paths.is_empty(), "Unable to find the Kurtex configuration file. ");

  paths
}
