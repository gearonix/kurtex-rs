use std::fmt;
use std::fmt::Formatter;

#[derive(PartialEq, Eq, Debug)]
pub enum CliError {
    // TODO: config path was not found
    ConfigPathNotFound,
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        return match self {
            // TODO:
            Self::ConfigPathNotFound => write!(f, "config path was not found"),
        };
    }
}
