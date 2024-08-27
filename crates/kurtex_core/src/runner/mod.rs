use std::rc::Rc;

use rayon::prelude::*;
use rccell::RcCell;

use crate::{AnyResult, watcher};
use crate::deno::ExtensionLoader;
use crate::ops::CollectorRegistryExt;
use crate::reporter::Reporter;
use crate::runner::collector::{FileCollector, TestRunnerConfig};
use crate::runner::runner::TestRunner;
use crate::runtime::{KurtexRuntime, KurtexRuntimeOptions};

pub mod collector;
pub mod reporter;
pub mod runner;

// TODO: extract config from deno.json
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
    is_main: true,
  });
  let runtime_rc = RcCell::new(runtime);

  let file_collector = FileCollector::new(config.clone(), runtime_rc.clone());
  let collector_ctx = file_collector.run().await.unwrap();
  let test_runner =
    TestRunner::new(collector_ctx.clone(), config.clone(), runtime_rc.clone());
  
  let module_graph = test_runner.run_files().await?;
  println!("{:#?}", module_graph);

  let context = collector_ctx.borrow_mut();
  let reporter = &context.reporter;
  reporter.report_finished(&context);

  if (config.watch) {
    reporter.watcher_started(&context);
    watcher::start_watcher(&context, config.clone()).await?;
  }

  Ok(())
}
