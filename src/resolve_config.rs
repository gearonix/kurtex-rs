use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::Error as AnyError;
use deno_core::{anyhow, v8};
use serde::{Deserialize, Serialize};

use crate::context::ContextProvider;
use crate::deno::module_resolver::{EsmModuleResolver, EsmResolverOptions};
use crate::error::CliError;
use crate::utils::fs::read_json_file;
use crate::{CliConfig, CLI_CONFIG, TOKIO_RUNTIME};

pub fn resolve_kurtex_config() -> Result<KurtexOptions, AnyError> {
  let CliConfig { config: config_path, .. } =
    ContextProvider::get(&CLI_CONFIG).unwrap();
  let config_path = config_path.as_ref().ok_or(CliError::MissingConfigPath)?;
  let config_ext = resolve_config_extension(config_path)?;

  match config_ext {
    ConfigExtension::Json => serialize_json_config(config_path),
    ConfigExtension::JavaScript | ConfigExtension::TypeScript => {
      execute_esm_config(config_path)
    }
  }
}

#[derive(Debug)]
pub enum ConfigExtension {
  JavaScript,
  TypeScript,
  Json,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KurtexOptions {
  #[serde(default)]
  pub includes: Vec<Cow<'static, str>>,
  #[serde(default)]
  pub excludes: Vec<Cow<'static, str>>,
  #[serde(default)]
  pub watch: bool,
  #[serde(default)]
  pub parallel: bool
}

const DEFAULT_INCLUDES: &'static [&'static str] =
  &["**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}"];

const DEFAULT_EXCLUDES: &'static [&'static str] =
  &["**/node_modules/**", "**/dist/**"];

impl Default for KurtexOptions {
  fn default() -> Self {
    let to_vec = |v: &'static [&'static str]| {
      v.iter().map(|&s| Cow::Borrowed(s)).collect()
    };

    KurtexOptions {
      includes: to_vec(DEFAULT_INCLUDES),
      excludes: to_vec(DEFAULT_EXCLUDES),
      watch: false,
      parallel: false
    }
  }
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
  config_path: &str,
) -> Result<KurtexOptions, AnyError> {
  read_json_file(config_path, CliError::FailedToReadConfigFile.into())
}

pub fn execute_esm_config(
  config_path: &str,
) -> Result<KurtexOptions, AnyError> {
  let tokio = ContextProvider::get(&TOKIO_RUNTIME).unwrap();

  tokio.block_on(process_esm_file(config_path))
}

async fn process_esm_file(
  config_path: &str,
) -> Result<KurtexOptions, AnyError> {
  let mut resolver = EsmModuleResolver::new(EsmResolverOptions::default());

  let module_id = resolver.process_esm_file(config_path).await?;
  let exports: v8::Local<v8::Object> =
    resolver.extract_file_exports(module_id, None::<&str>).await?;
  let kurtex_config = resolver
    .serialize_v8_object::<KurtexOptions>(exports)
    .await
    .map_err(|e| CliError::InvalidConfigOptions(e))?;

  Ok(kurtex_config)
}
