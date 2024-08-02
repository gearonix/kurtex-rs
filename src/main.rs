use std::env;
use std::ops::Deref;
use std::path::PathBuf;
use std::process::exit;

use clap::Parser;
use deno_core::error::AnyError;

use find_up::find_up_files;
use runtime::runtime::RuntimeManager;

use crate::context::{CLI_CONFIG, ContextProvider, TOKIO_RUNTIME};
use crate::error::CliError;
use crate::runtime::runtime::RuntimeOptions;

mod context;
mod deno;
mod error;
mod find_up;
mod resolve_config;
mod runner;
mod runtime;
mod utils;

#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
pub struct CliConfig {
  #[arg(short, long)]
  name: Option<String>,
  #[arg(short, long)]
  root: Option<String>,
  #[arg(short, long)]
  config: Option<String>,
  #[arg(short, long, default_value_t = false)]
  watch: bool,
  #[arg(short, long, default_value_t = false)]
  update: bool,
  #[arg(short, long, default_value_t = false)]
  global: bool,
  #[arg(short, long, default_value_t = false)]
  dev: bool,
}

#[derive(PartialEq, Eq, Debug)]
pub struct ConfigFiles(pub &'static [&'static str]);

pub static CONFIG_FILES: ConfigFiles =
  ConfigFiles(&["kurtex.config.ts", "kurtex.config.js", "kurtex.config.json"]);

impl Deref for ConfigFiles {
  type Target = [&'static str];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
fn main() -> Result<(), AnyError> {
  let args = CliConfig::parse();
  let tokio_runtime =
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

  ContextProvider::init_once(&TOKIO_RUNTIME, tokio_runtime);

  let root_dir = &args
    .root
    .as_deref()
    .map(PathBuf::from)
      // TODO: rewrite to execute dir
    .unwrap_or_else(|| env::current_dir().unwrap());

  let config_path = args
    .config
    .as_ref()
    .map(|cfg| root_dir.join(cfg))
    .or_else(|| find_up_files(&CONFIG_FILES, Some(root_dir.as_path())))
    .filter(|path| path.exists());

  if let Some(cfg_path) = &config_path {
    let cli_config =
      CliConfig { config: Some(cfg_path.display().to_string()), ..args };

    ContextProvider::init_once(&CLI_CONFIG, cli_config);

    RuntimeManager::start(&RuntimeOptions {
      root: root_dir,
      files: Vec::new(),
    })
  } else {
    eprintln!("kurtex: {}", CliError::ConfigPathNotFound);
    exit(exits::RUNTIME_ERROR);
  }
}

mod exits {
  #[allow(unused)]
  pub const SUCCESS: i32 = 0;
  pub const RUNTIME_ERROR: i32 = 1;
}
