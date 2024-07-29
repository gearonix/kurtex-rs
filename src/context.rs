use crate::runtime::runtime::RuntimeConfig;
use crate::CliConfig;
use std::sync::OnceLock;
use std::thread::LocalKey;
use tokio::runtime::Runtime;

thread_local! {
    pub static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();
    pub static CLI_CONFIG: OnceLock<CliConfig> = OnceLock::new();
    pub static RUNTIME_CONFIG: OnceLock<RuntimeConfig> = OnceLock::new();
}

pub struct ContextProvider;

impl ContextProvider {
  pub fn get<T>(resource: &'static LocalKey<OnceLock<T>>) -> Option<&'_ T> {
    resource.with(|rs| rs.get())
  }

  pub fn init_once<T>(
    resource: &'static LocalKey<OnceLock<T>>,
    val: T,
  ) -> &'static T
  where
    T: 'static + Send,
  {
    resource.with(|rs| rs.get_or_init(|| val))
  }
}
