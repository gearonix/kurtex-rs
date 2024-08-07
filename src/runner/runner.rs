use std::borrow::Cow;
use std::cell::RefCell;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;

use deno_core::error::AnyError;
use globwalk;

use crate::context::{ContextProvider, RUNTIME_CONFIG};
use crate::deno::module_resolver::{
  extract_op_state, EsmModuleResolver, EsmResolverOptions,
};
use crate::runner::collector::CollectorFile;
use crate::runner::context::CollectorContext;
use crate::runner::ops::{CollectorRegistryOps, OpsLoader};
use crate::runtime::runtime::RuntimeConfig;
use crate::utils::tokio::{create_pinned_future, run_concurrently};
use crate::CLI_CONFIG;

const TEST_FILES_MAX_DEPTH: u32 = 25;

pub struct Runner;

impl Runner {
  pub async fn run_with_options() -> Result<(), AnyError> {
    let cli_config = ContextProvider::get(&CLI_CONFIG).unwrap();
    let runtime_config = ContextProvider::get(&RUNTIME_CONFIG).unwrap();
    let RuntimeConfig { options: runtime_opts, .. } = runtime_config;

    let collector_ops_loader: Box<dyn OpsLoader> =
      Box::new(CollectorRegistryOps::new());

    let esm_resolver = EsmModuleResolver::new(EsmResolverOptions {
      loaders: Vec::from([collector_ops_loader]),
    });
    let esm_resolver = Rc::new(RefCell::new(esm_resolver));

    if cli_config.watch {
      // TODO
      // options.runtime.enable_watch_mode();
    }

    async fn process_test_file(
      esm_resolver: Rc<RefCell<EsmModuleResolver>>,
      file_path: PathBuf,
    ) -> Result<Rc<CollectorFile>, AnyError> {
      let collector_file = CollectorFile {
        file_path: file_path.clone(),
        ..CollectorFile::default()
      };
      let collector_file = Rc::new(collector_file);

      let mut resolver = esm_resolver.borrow_mut();

      let op_state = resolver.get_op_state()?;
      let mut op_state = op_state.borrow_mut();
      let collector_ctx = extract_op_state::<CollectorContext>(&mut op_state)?;

      collector_ctx.clear();

      let module_id = resolver
        .process_esm_file(file_path.display().to_string(), false)
        .await
        .unwrap();

      let obtained_collectors = collector_ctx.get_all_nodes();

      for collector in obtained_collectors {
        // TODO: rewrite RcMutMutateError
        let collected_node = collector
          .with_mut(|clr| clr.collect_node(collector_file.clone()).unwrap())
          .unwrap();

        let mut file_nodes = collector_file.nodes.borrow_mut();
        file_nodes.push(collected_node);

        collector_ctx.register_node(collector);
      }
      *collector_file.collected.borrow_mut() = true;
      // let collector_file = Rc::try_unwrap(collector_file).unwrap();

      Ok(collector_file)
    }

    let processed_tasks = Self::collect_test_files(&runtime_config)?
      .map(move |file_path: PathBuf| {
        let esm_resolver = Rc::clone(&esm_resolver);
        create_pinned_future(process_test_file(esm_resolver, file_path))
      })
      .collect();

    if runtime_opts.parallel {
      let files = run_concurrently(processed_tasks).await;

      println!("files: {:?}", files);
    } else {
      for task in processed_tasks {
        task().await;
      }
    }

    Ok(())
  }

  fn collect_test_files(
    runtime_cfg: &RuntimeConfig,
  ) -> Result<impl Iterator<Item = PathBuf> + '_, AnyError> {
    let runtime_opts = &runtime_cfg.options;
    let walk_glob = |patterns: &Vec<Cow<'static, str>>| {
      globwalk::GlobWalkerBuilder::from_patterns(&runtime_cfg.root, patterns)
        .max_depth(TEST_FILES_MAX_DEPTH as usize)
        .build()
        .unwrap()
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
    };

    #[allow(unused_mut)]
    let mut included_cases = walk_glob(&runtime_opts.includes);
    let mut excluded_cases = walk_glob(&runtime_opts.excludes);

    Ok(included_cases.filter(move |included_path| {
      !excluded_cases.any(|excluded_path| excluded_path.eq(included_path))
    }))
  }

  pub fn run() {}
}
