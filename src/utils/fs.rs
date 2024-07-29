use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs::read_to_string;

pub fn read_json_file<T, E>(file_path: &str, error: E) -> Result<T, E>
where
  T: Debug + Serialize + for<'de> Deserialize<'de>,
  E: From<serde_json::Error>,
{
  let cfg_contents = read_to_string(file_path)
    .map_err(|e| {
      format!("Failed to read config file {}: {}", file_path, e.to_string())
    })
    .map_err(|e| error)?;

  let serialized_cfg: T = serde_json::from_str(cfg_contents.leak())?;

  Ok(serialized_cfg)
}
