use crate::config::{get_or_init_cli_config, get_or_init_runtime_cfg};

struct Runner;

impl Runner {
  pub fn run() {
    let cli = get_or_init_cli_config(None);
    let runtime = get_or_init_runtime_cfg(None);

    if (cli.watch) {
      todo!("watch");
    }
  }
}
