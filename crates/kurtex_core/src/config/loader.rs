use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Error as AnyError};
use deno_core::{anyhow, v8};
use jsonc_parser::parse_to_serde_value;
use serde::{Deserialize, Serialize};

use crate::context::ContextProvider;
use crate::deno::module_resolver::{
  EsmModuleResolver, EsmResolverOptions, EsmSerdeResolver,
};
use crate::utils::fs::read_json_file;
use crate::{AnyResult, CliConfig, CLI_CONFIG, TOKIO_RUNTIME};

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
  pub watch: bool,

  #[serde(default)]
  pub parallel: bool,
}

impl Default for KurtexConfig {
  fn default() -> Self {
    KurtexConfig {
      includes: DEFAULT_INCLUDES.to_vec(),
      excludes: DEFAULT_EXCLUDES.to_vec(),
      watch: false,
      parallel: false,
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

  pub fn load(&self) -> AnyResult<KurtexConfig> {
    assert!(self.config_path, "Config path not found.");

    match self.resolve_config_extension()? {
      ConfigExtension::Json => self.parse_json_file(),
      ConfigExtension::JavaScript | ConfigExtension::TypeScript => {
        self.parse_esm_file()
      }
    }
  }

  fn resolve_config_extension(&self) -> AnyResult<ConfigExtension> {
    let file_extension = PathBuf::from(&self.config_path)
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
    let cfg_contents = fs::read_to_string(&self.config_path).map_err(|e| {
      format!(
        "Failed to read config file {}: {}",
        &self.config_path,
        e.to_string()
      )
    });

    serde_json::from_str(cfg_contents.leak())?
  }

  fn parse_esm_file(&self) -> Result<KurtexConfig, AnyError> {
    let rt = tokio::runtime::Handle::current();

    rt.block_on(async move {
      let mut resolver = EsmModuleResolver::new(EsmResolverOptions::default());
      let module_id =
        resolver.process_esm_file(&self.config_path, true).await?;

      let (exports, scope) = resolver
        .extract_file_exports::<v8::Local<v8::Object>, &str>(module_id, None)
        .await?;
      let kurtex_config =
        EsmSerdeResolver::serialize::<KurtexConfig>(scope, exports)
          .await
          .map_err(|e| anyhow!("Failed to parse config: Invalid settings"));

      Ok(kurtex_config)
    })
  }
}
