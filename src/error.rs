use deno_core::error::AnyError;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum CliError {
  // TODO: config path was not found
  // TODO: refactor everything here this is wierd
  ConfigPathNotFound,
  MissingConfigPath,
  WrongConfigExtension,
  MissingConfigExtension,
  MissingDefaultExport,
  InvalidConfigOptions(deno_core::serde_v8::Error),
  FailedToReadConfigFile,
}

impl std::error::Error for CliError {}

impl fmt::Display for CliError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Self::ConfigPathNotFound => write!(f, "config path was not found"),
      Self::MissingConfigPath => write!(f, "missing config path"),
      Self::WrongConfigExtension => write!(f, "config extension is wrong"),
      Self::MissingConfigExtension => write!(f, "missing config extension"),
      Self::MissingDefaultExport => write!(f, "missing default export"),
      Self::InvalidConfigOptions(error) => {
        write!(f, "invalid config options: {}", error.to_owned())
      }
      Self::FailedToReadConfigFile => write!(f, "failed to read config file"),
    }
  }
}
