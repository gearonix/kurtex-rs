use std::path::PathBuf;
use std::rc::Rc;

use deno_ast::ModuleSpecifier;
use deno_core::ModuleType;
use deno_graph::source::MemoryLoader;
use rayon::prelude::*;
use rccell::RcCell;

use kurtex_binding::ts_module_loader::{
  get_content_type_header, get_module_type_from_path,
};

use crate::deno::ExtensionLoader;
use crate::ops::CollectorRegistryExt;
use crate::reporter::Reporter;
use crate::runner::collector::{FileCollector, TestRunnerConfig};
use crate::runner::runner::TestRunner;
use crate::runtime::{KurtexRuntime, KurtexRuntimeOptions};
use crate::walk::Walk;
use crate::{watcher, AnyResult};

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
  test_runner.run_files().await;

  let config_c = config.clone();
  let runtime_c = runtime_rc.clone();

  // SPAWN START
  let runtime_root = config_c.root_dir.join("dev/src/main.ts");
  let mut runtime = runtime_c.borrow_mut();

  // TODO: temporary
  // TODO: additional files
  let mut source_paths = config_c.includes.clone();
  source_paths.extend(["src/**".to_string()]);

  let _ = runtime.process_esm_file(runtime_root.to_string_lossy(), true).await;

  let graph = runtime.build_graph().await.unwrap();

  println!("{:#?}", graph);

  // SPAWN END

  let context = collector_ctx.borrow_mut();
  let reporter = &context.reporter;
  reporter.report_finished(&context);

  if (config.watch) {
    reporter.watcher_started(&context);
    watcher::start_watcher(&context, config.clone()).await?;
  }

  Ok(())
}
