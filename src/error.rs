use std::fmt;
use std::fmt::Formatter;

#[derive(PartialEq, Eq, Debug)]
pub enum CliError {
  // TODO: config path was not found
  ConfigPathNotFound,
  MissingConfigPath,
  WrongConfigExtension,
  MissingConfigExtension,
}


impl std::error::Error for CliError {}

impl fmt::Display for CliError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    return match self {
      // TODO:
      Self::ConfigPathNotFound => write!(f, "config path was not found"),
      Self::WrongConfigExtension => write!(f, "config extension is wrong"),
      Self::MissingConfigExtension => write!(f, "config extension is wrong"),
      _ => unreachable!()
    };
  }
}
