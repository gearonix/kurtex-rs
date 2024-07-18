use crate::CliConfig;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::PathBuf;

use crate::config::get_or_init_cli_config;
use crate::error::CliError;

pub fn resolve_kurtex_config() {
  let CliConfig { config: config_path, .. } = get_or_init_cli_config(None);
  let config_path = config_path.as_ref().ok_or(CliError::MissingConfigPath)?;
  let config_ext = resolve_config_extension(config_path)?;

  let resolved_config = match config_ext {
    ConfigExtension::Json => serialize_json_config(config_path),
    ConfigExtension::JavaScript | ConfigExtension::TypeScript => {}
  };
}

#[derive(Debug)]
pub enum ConfigExtension {
  JavaScript,
  TypeScript,
  Json,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KurtexConfig {
  hello: String,
}

pub fn resolve_config_extension(
  config_path: &String,
) -> Result<ConfigExtension, CliError> {
  let config_path = PathBuf::from(config_path);
  let file_extension = config_path
    .extension()
    .and_then(OsStr::to_str)
    .ok_or(CliError::MissingConfigExtension)?;

  match file_extension {
    "ts" => Ok(ConfigExtension::TypeScript),
    "js" => Ok(ConfigExtension::JavaScript),
    "json" => Ok(ConfigExtension::Json),
    _ => Err(CliError::MissingConfigExtension),
  }
}

pub fn serialize_json_config(
  config_path: &String,
) -> Result<KurtexConfig, Box<dyn Error>> {
  let cfg_contents = read_to_string(config_path).map_err(|e| {
    format!("Failed to read config file {}: {}", config_path, e.to_string())
  })?;
  let serialized_cfg: KurtexConfig = serde_json::from_str(&cfg_contents)?;

  Ok(serialized_cfg)
}
