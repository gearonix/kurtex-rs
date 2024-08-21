use std::env;

use anyhow::{Context, Error};
use clap::builder::Command;
use clap::{Arg, ArgAction};
use tracing_subscriber::filter::FilterExt;

use crate::runner::{CliRunner, Runner};

mod runner;

fn main() -> Result<(), Error> {
  init_tracing();

  let cli = build_cli();
  let matches = cli.get_matches();

  CliRunner::new(matches).run()
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
  Command::new(env!("CARGO_BIN_NAME"))
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
        .action(ArgAction::SetFalse)
        .help("Enable watch mode")
        .value_parser(clap::value_parser!(bool)),
    )
    .arg(
      Arg::new("globals")
        .long("globals")
        .action(ArgAction::SetFalse)
        .help("Inject apis globally")
        .value_parser(clap::value_parser!(bool)),
    )
    .arg(
      Arg::new("parallel")
        .long("parallel")
        .action(ArgAction::SetTrue)
        .help("Run tasks in parallel")
        .value_parser(clap::value_parser!(bool)),
    )
}

pub mod exits {
  #[allow(unused)]
  pub const SUCCESS: i32 = 0;
  pub const RUNTIME_ERROR: i32 = 1;
}

pub mod settings {
  pub const RUNTIME_SNAPSHOT: &'static [u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/KURTEX_SNAPSHOT.bin"));
}
