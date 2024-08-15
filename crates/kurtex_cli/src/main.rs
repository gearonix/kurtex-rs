use crate::module_specifier::{VALID_CONFIG_FILES, VALID_EXTENSIONS};
use crate::runner::Runner;
use crate::walk::{Extensions, Walk};
use anyhow::{Context, Error};
use clap::builder::Command;
use clap::{Arg, ArgMatches};
use std::env;
use std::path::{Path, PathBuf};
use tokio::time;
use tracing_subscriber::filter::FilterExt;

mod module_specifier;
mod runner;
mod walk;

const CLI_SHORT_NAME: &str = "ktx";

fn main() -> Result<(), Error> {
  init_tracing();

  let cli = build_cli();
  let mut matches = cli.get_matches();

  let runtime =
    tokio::runtime::Builder::new_current_thread().enable_all().build()?;
  let root_dir = matches.remove_one::<PathBuf>("root");
  let mut config_path = matches.remove_one::<PathBuf>("config").unwrap();

  Ok(())
}

pub struct CliRunner {
  options: ArgMatches,
}

impl Runner for CliRunner {
  type Options = ArgMatches;

  fn new(options: Self::Options) -> Self {
    Self { options }
  }

  fn run(mut self) -> () {
    let root_dir = self.options.remove_one::<PathBuf>("root");
    let mut config_path = self.options.remove_one::<PathBuf>("config").unwrap();

    let current_dir = std::env::current_dir().context("Unable to get CWD")?;
    let root_dir =
      root_dir.unwrap_or(std::env::current_dir().context("Unable to get CWD")?);
    current_dir.join(&config_path).clone_into(&mut config_path);
    let now = time::Instant::now();

    let config_path = if &config_path.exists() {
      config_path
    } else {
      let mut paths = Walk::new(&VALID_CONFIG_FILES, root_dir)
        .with_extensions(Extensions(VALID_EXTENSIONS.to_vec()))
        .build()
        .collect::<Vec<PathBuf>>();

      assert!(
        !paths.is_empty(),
        "Unable to find the Kurtex configuration file. "
      );

      paths.drain(..1).next().unwrap()
    };
  }
}

fn init_tracing() {
  use tracing_subscriber::{filter::Targets, prelude::*};

  tracing_subscriber::registry()
    .with(env::var("KURTEX_LOG").map_or_else(
      |_| Targets::new(),
      |env_var| env_var.parse::<Targets>().unwrap(),
    ))
    .with(
      tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(std::io::stderr)
        .boxed(),
    )
    .init();
}

fn build_cli() -> Command {
  Command::new(CLI_SHORT_NAME)
    .arg(
      Arg::new("root")
        .long("root")
        .value_name("ROOT_DIR")
        .help("Root path")
        .require_equals(true)
        .value_hint(clap::ValueHint::DirPath)
        .value_parser(clap::value_parser!(String)),
    )
    .arg(
      Arg::new("config")
        .long("config")
        .short('c')
        .help("Path to config file")
        .default_value("./kurtex.config.ts")
        .require_equals(true)
        .value_hint(clap::ValueHint::FilePath)
        .value_parser(clap::value_parser!(String)),
    )
    .arg(
      Arg::new("watch")
        .long("watch")
        .short('w')
        .help("Enable watch mode")
        .value_parser(clap::value_parser!(bool)),
    )
    .arg(
      Arg::new("globals")
        .long("globals")
        .help("Inject apis globally")
        .value_parser(clap::value_parser!(bool)),
    )
}

pub mod exits {
  #[allow(unused)]
  pub const SUCCESS: i32 = 0;
  pub const RUNTIME_ERROR: i32 = 1;
}
