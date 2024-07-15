use crate::CliConfig;
use std::sync::OnceLock;

pub fn get_or_init_cli_config(cfg: CliConfig) -> &'static CliConfig {
    static CLI_CONFIG: OnceLock<CliConfig> = OnceLock::new();

    CLI_CONFIG.get_or_init(|| cfg)
}
