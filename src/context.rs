use std::sync::OnceLock;

use tokio::runtime::Runtime;

use crate::runtime::runtime::RuntimeConfig;
use crate::CliConfig;

pub static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();
pub static CLI_CONFIG: OnceLock<CliConfig> = OnceLock::new();
pub static RUNTIME_CONFIG: OnceLock<RuntimeConfig> = OnceLock::new();

pub struct ContextProvider;

impl ContextProvider {
  pub fn get<T>(resource: &'static OnceLock<T>) -> Option<&'static T>
  where
    T: 'static,
  {
    resource.get()
  }

  pub fn init_once<T>(resource: &'static OnceLock<T>, val: T) -> &'static T
  where
    T: 'static,
  {
    resource.get_or_init(|| val)
  }
}
