use std::path::PathBuf;
use std::rc::Rc;

use rayon::prelude::*;
use rccell::RcCell;

use crate::deno::ExtensionLoader;
use crate::ops::CollectorRegistryExt;
use crate::reporter::Reporter;
use crate::runner::collector::{
  FileCollector, FileCollectorOptions, RunnerCollectorContext,
  TestRunnerConfig,
};
use crate::runner::runner::TestRunner;
use crate::runtime::{KurtexRuntime, KurtexRuntimeOptions};
use crate::{watcher, AnyResult};

pub mod collector;
pub mod reporter;
pub mod runner;

// TODO: extract config from deno.json
#[derive(Clone)]
pub struct EmitRuntimeOptions {
  pub runtime_snapshot: &'static [u8],
}

pub async fn launch(
  config: Rc<TestRunnerConfig>,
  emit_opts: Rc<EmitRuntimeOptions>,
) -> AnyResult {
  let (runtime, ctx) =
    launch_runner(config.clone(), emit_opts.clone(), None).await?;
  let module_graph = runtime.borrow_mut().build_graph().await;

  let context = ctx.borrow_mut();
  context.reporter.report_finished(&context);

  if (config.watch) {
    context.reporter.watcher_started(&context);

    fn restart_runner(
      changed_files: Vec<PathBuf>,
      config: Rc<TestRunnerConfig>,
      emit_opts: Rc<EmitRuntimeOptions>,
    ) {
      deno_core::unsync::spawn(async move {
        let launch_result =
          launch_runner(config, emit_opts, Some(changed_files)).await;

        if let Ok((_, ctx)) = launch_result {
          let context = ctx.borrow_mut();
          context.reporter.watcher_started(&context);
        }
      });
    };

    watcher::start_watcher(
      Box::new(restart_runner),
      module_graph,
      config,
      emit_opts,
      &context,
    )
    .await?;
  }

  Ok(())
}

async fn launch_runner(
  config: Rc<TestRunnerConfig>,
  emit_opts: Rc<EmitRuntimeOptions>,
  existing_paths: Option<Vec<PathBuf>>,
) -> AnyResult<(RcCell<KurtexRuntime>, RcCell<RunnerCollectorContext>)> {
  let runtime = create_runtime(emit_opts);

  let file_collector =
    FileCollector::new(config.clone(), runtime.clone());
  let collector_ctx =
    file_collector.run(FileCollectorOptions { existing_paths }).await?;

  let mut test_runner = TestRunner::new(
    collector_ctx.clone(),
    config.clone(),
    runtime.clone(),
  );

  test_runner.run_files().await;

  Ok((runtime, collector_ctx))
}

fn create_runtime(
  emit_options: Rc<EmitRuntimeOptions>,
) -> RcCell<KurtexRuntime> {
  let collector_ops_loader: Box<dyn ExtensionLoader> =
    Box::new(CollectorRegistryExt::new());

  // TODO: reduce performance
  let runtime = RcCell::new(KurtexRuntime::new(KurtexRuntimeOptions {
    loaders: vec![collector_ops_loader],
    snapshot: emit_options.runtime_snapshot,
    is_main: true,
  }));

  runtime
}
