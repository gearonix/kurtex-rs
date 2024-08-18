use std::path::PathBuf;
use std::env;

pub fn kurtex_tmp_dir() -> PathBuf {
  env::temp_dir().join("kurtex-tmp")
}
