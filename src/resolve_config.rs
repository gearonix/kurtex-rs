use std::borrow::Cow;
use std::env;
use std::ffi::OsStr;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Error as AnyError;
use deno_core::{anyhow, v8, ModuleId};
use serde::{Deserialize, Serialize};

use crate::config::get_or_init_cli_config;
use crate::deno::module_loader::TsModuleLoader;
use crate::error::CliError;
use crate::CliConfig;

pub fn resolve_kurtex_config() -> Result<KurtexOptions, AnyError> {
  let CliConfig { config: config_path, .. } = get_or_init_cli_config(None);
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
  let cfg_contents = read_to_string(config_path)
    .map_err(|e| {
      format!("Failed to read config file {}: {}", config_path, e.to_string())
    })
    .map_err(|e| CliError::FailedToReadConfigFile)?;
  let serialized_cfg: KurtexOptions = serde_json::from_str(&cfg_contents)?;

  Ok(serialized_cfg)
}

pub fn execute_esm_config(
  config_path: &str,
) -> Result<KurtexOptions, AnyError> {
  let runtime =
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

  runtime.block_on(process_esm_file(config_path))
}

async fn resolve_module_id(
  deno_runtime: &mut deno_core::JsRuntime,
  file_path: &str,
  is_main_module: bool,
) -> Result<ModuleId, AnyError> {
  // NOTE: remove current_dir
  let module_specifier = env::current_dir()
    .map_err(AnyError::from)
    .and_then(|current_dir| {
      deno_core::resolve_path(file_path, current_dir.as_path())
        .map_err(AnyError::from)
    })
    .unwrap();

  if is_main_module {
    deno_runtime.load_main_es_module(&module_specifier).await
  } else {
    deno_runtime.load_side_es_module(&module_specifier).await
  }
}

async fn process_esm_file(
  config_path: &str,
) -> Result<KurtexOptions, AnyError> {
  let deno_runtime =
    &mut deno_core::JsRuntime::new(deno_core::RuntimeOptions {
      module_loader: Some(Rc::new(TsModuleLoader)),
      ..Default::default()
    });

  let mod_id = resolve_module_id(deno_runtime, config_path, true).await?;

  deno_runtime.mod_evaluate(mod_id).await?;
  deno_runtime.run_event_loop(Default::default()).await?;

  let global = deno_runtime.get_module_namespace(mod_id)?;
  let scope = &mut deno_runtime.handle_scope();
  let glb_open = global.open(scope);

  let default_export = v8::String::new(scope, "default").unwrap();

  let exported_config = glb_open
    .get(scope, default_export.into())
    .ok_or(CliError::MissingDefaultExport)?;
  let exported_config = v8::Local::<v8::Object>::try_from(exported_config)?;
  let serialized_cfg: KurtexOptions =
    deno_core::serde_v8::from_v8(scope, exported_config.into())
      .map_err(|e| CliError::InvalidConfigOptions(e))?;

  Ok(serialized_cfg)
}
