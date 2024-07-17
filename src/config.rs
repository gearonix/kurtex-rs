use crate::runtime::runtime::RuntimeConfig;
use crate::CliConfig;
use std::sync::OnceLock;

pub fn get_or_init_cli_config(cfg: Option<CliConfig>) -> &'static CliConfig {
    static CLI_CONFIG: OnceLock<CliConfig> = OnceLock::new();

    CLI_CONFIG.get_or_init(|| cfg.unwrap_or(CliConfig::default()))
}

pub fn get_or_init_runtime_cfg(cfg: Option<RuntimeConfig>) -> &'static RuntimeConfig {
    static RUNTIME_CONFIG: OnceLock<RuntimeConfig> = OnceLock::new();

    RUNTIME_CONFIG.get_or_init(|| cfg.unwrap_or(RuntimeConfig::default()))
}
