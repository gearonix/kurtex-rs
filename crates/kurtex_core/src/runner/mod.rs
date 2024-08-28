use std::path::PathBuf;
use std::rc::Rc;

use deno_core::futures;
use rayon::prelude::*;
use rccell::RcCell;

use crate::deno::ExtensionLoader;
use crate::ops::CollectorRegistryExt;
use crate::reporter::Reporter;
use crate::runner::collector::{
  FileCollector, FileCollectorOptions, TestRunnerConfig,
};
use crate::runner::runner::TestRunner;
use crate::runtime::{KurtexRuntime, KurtexRuntimeOptions};
use crate::{watcher, AnyResult};

pub mod collector;
pub mod reporter;
pub mod runner;

// TODO: extract config from deno.json
pub struct EmitRuntimeOptions {
  pub runtime_snapshot: &'static [u8],
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

  let file_collector =
    FileCollector::new(config.clone(), runtime_rc.clone());
  let collector_ctx =
    file_collector.run(FileCollectorOptions::default()).await.unwrap();
  let mut test_runner = TestRunner::new(
    collector_ctx.clone(),
    config.clone(),
    runtime_rc.clone(),
  );

  test_runner.run_files().await;

  let module_graph = {
    let runtime = runtime_rc.borrow_mut();
    runtime.build_graph().await
  };

  let context = collector_ctx.borrow_mut();
  let reporter = &context.reporter;

  reporter.report_finished(&context);

  if (config.watch) {
    let restart_runner = Box::new(
      |changed_files: Vec<PathBuf>,
       file_collector: &FileCollector,
       test_runner: &mut TestRunner| {
        futures::executor::block_on(async move {
          let collector_ctx = file_collector
            .run(FileCollectorOptions { existing_paths: changed_files })
            .await
            .unwrap();

          test_runner.with_context(collector_ctx).run_files().await;
        });
      },
    );

    reporter.watcher_started(&context);

    watcher::start_watcher(
      module_graph,
      restart_runner,
      &file_collector,
      &mut test_runner,
    )
    .await?;
  }

  Ok(())
}
