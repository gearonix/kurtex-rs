use std::rc::Rc;

use rccell::RcCell;

use crate::deno::ExtensionLoader;
use crate::ops::CollectorRegistryExt;
use crate::reporter::Reporter;
use crate::runner::collector::{FileCollector, TestRunnerConfig};
use crate::runner::runner::TestRunner;
use crate::runtime::{KurtexRuntime, KurtexRuntimeOptions};
use crate::AnyResult;

pub mod collector;
pub mod runner;

#[derive(Default, Debug)]
pub struct EmitRuntimeOptions {
  pub runtime_snapshot: &'static [u8],
}

impl EmitRuntimeOptions {
  pub fn new_from_snapshot(runtime_snapshot: &'static [u8]) -> Self {
    EmitRuntimeOptions { runtime_snapshot }
  }
}

pub async fn run(
  config: TestRunnerConfig,
  emit_opts: EmitRuntimeOptions,
) -> AnyResult {
  let config = Rc::new(config);
  let collector_ops_loader: Box<dyn ExtensionLoader> =
    Box::new(CollectorRegistryExt::new());

  let runtime = KurtexRuntime::new(KurtexRuntimeOptions {
    loaders: vec![collector_ops_loader],
    snapshot: emit_opts.runtime_snapshot,
  });
  let runtime_rc = RcCell::new(runtime);

  let file_collector = FileCollector::new(config.clone(), runtime_rc.clone());
  let collector_ctx = file_collector.run().await.unwrap();

  let test_runner =
    TestRunner::new(collector_ctx.clone(), config.clone(), runtime_rc.clone());
  test_runner.run_files().await;

  let collector_ctx = collector_ctx.borrow_mut();
  let reporter = &collector_ctx.reporter;
  reporter.report_finished();

  Ok(())
}
