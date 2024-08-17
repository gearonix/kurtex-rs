use std::env;
use std::path::PathBuf;

pub mod collector;
pub mod config;
pub mod deno;
pub mod runner;
pub mod util;
pub mod walk;

pub type AnyResult<T = ()> = Result<T, anyhow::Error>;
pub type AnyError = anyhow::Error;

pub fn kurtex_tmp_dir() -> PathBuf {
  env::temp_dir().join("kurtex-tmp")
}
