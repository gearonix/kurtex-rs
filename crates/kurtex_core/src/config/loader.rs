use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::error::AnyResult;
use anyhow::{anyhow, Error as AnyError};
use deno_core::{anyhow, v8};
use serde::{Deserialize, Serialize};

use crate::deno::runtime::{
  EsmSerdeResolver, KurtexRuntime, KurtexRuntimeOptions,
};

pub struct ConfigLoader {
  config_path: Cow<'static, str>,
}

pub enum ConfigExtension {
  JavaScript,
  TypeScript,
  Json,
}

// A single object in the `kurtex.config` file
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct KurtexConfig {
  #[serde(default)]
  pub includes: Vec<String>,

  #[serde(default)]
  pub excludes: Vec<String>,

  #[serde(default)]
  pub watch: Option<bool>,

  #[serde(default)]
  pub parallel: Option<bool>,
}

impl Default for KurtexConfig {
  fn default() -> Self {
    let to_vec =
      |v: &'static [&'static str]| v.iter().map(|&s| s.to_owned()).collect();

    KurtexConfig {
      includes: to_vec(DEFAULT_INCLUDES),
      excludes: to_vec(DEFAULT_EXCLUDES),
      watch: None,
      parallel: None,
    }
  }
}

const DEFAULT_INCLUDES: &'static [&'static str] =
  &["**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}"];

const DEFAULT_EXCLUDES: &'static [&'static str] =
  &["**/node_modules/**", "**/dist/**"];

impl ConfigLoader {
  pub fn new<S>(config_path: S) -> Self
  where
    S: Into<Cow<'static, str>>,
  {
    ConfigLoader { config_path: config_path.into() }
  }

  pub async fn load(&self) -> AnyResult<KurtexConfig> {
    let path_exists = Path::new(self.config_path.deref()).exists();

    assert!(path_exists, "Config path not found.");

    match self.resolve_config_extension()? {
      ConfigExtension::Json => self.parse_json_file(),
      ConfigExtension::JavaScript | ConfigExtension::TypeScript => {
        self.parse_esm_file().await
      }
    }
  }

  fn resolve_config_extension(&self) -> AnyResult<ConfigExtension> {
    let config_path = PathBuf::from(self.config_path.as_ref());

    let file_extension = config_path
      .extension()
      .and_then(OsStr::to_str)
      .ok_or_else(|| anyhow!("Failed to resolve config: missing extension."))?;

    match file_extension {
      "ts" => Ok(ConfigExtension::TypeScript),
      "js" => Ok(ConfigExtension::JavaScript),
      "json" => Ok(ConfigExtension::Json),
      _ => Err(anyhow!("Invalid config extension")),
    }
  }

  fn parse_json_file(&self) -> Result<KurtexConfig, AnyError> {
    let config_path = self.config_path.as_ref();

    let cfg_contents = fs::read_to_string(config_path).map_err(|e| {
      anyhow!(
        "Failed to read config file {}: {}",
        &self.config_path,
        e.to_string()
      )
    })?;

    serde_json::from_str(cfg_contents.leak())
      .map_err(|_| anyhow!("Failed to deserialize config file (json)"))
  }

  async fn parse_esm_file(&self) -> Result<KurtexConfig, AnyError> {
    let mut resolver = KurtexRuntime::new(KurtexRuntimeOptions::default());
    let module_id = resolver.process_esm_file(&self.config_path, true).await?;

    let (exports, scope) = resolver
      .extract_file_exports::<v8::Local<v8::Object>, &str>(module_id, None)
      .await?;
    EsmSerdeResolver::serialize::<KurtexConfig>(scope, exports)
      .await
      .map_err(|_| anyhow!("Failed to parse config: Invalid settings"))
  }
}
