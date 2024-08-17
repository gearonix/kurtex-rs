use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use deno_core::anyhow::Context;
use deno_core::error::AnyError;
use deno_core::futures::StreamExt;
use globwalk;
use mut_rc::MutRc;

use crate::collector::{
  CollectorContext, CollectorFile, CollectorMetadata, CollectorMode,
  CollectorState, NodeCollectorManager,
};
use crate::config::loader::KurtexConfig;
use crate::deno::module_resolver::{
  extract_op_state, extract_op_state_mut, EsmModuleResolver, EsmResolverOptions,
};
use crate::deno::ops::{CollectorRegistryOps, OpsLoader};
use crate::util::tokio::{create_pinned_future, run_concurrently};
use crate::walk::Walk;
use crate::AnyResult;

#[derive(Default, Debug)]
pub struct TestRunnerOptions {
  pub watch: bool,
  pub globals: bool,
  pub parallel: bool,
  pub config_path: PathBuf,
  pub root_dir: PathBuf,
  pub includes: Vec<String>,
  pub excludes: Vec<String>,
}

impl TestRunnerOptions {
  pub fn adjust_config_file(&mut self, config: KurtexConfig) {
    self.parallel |= config.parallel;
    self.watch |= config.watch;
    self.includes = config.includes;
    self.excludes = config.excludes;
  }
}

type CollectorFileMap = HashMap<PathBuf, Rc<CollectorFile>>;

pub struct TestRunner {
  options: TestRunnerOptions,
}

impl TestRunner {
  pub fn new(options: TestRunnerOptions) -> Self {
    TestRunner { options }
  }

  pub async fn run(&self) -> AnyResult {
    // let cli_config = ContextProvider::get(&CLI_CONFIG).unwrap();
    // let runtime_config = ContextProvider::get(&RUNTIME_CONFIG).unwrap();
    // let RuntimeConfig { options: runtime_opts, .. } = runtime_config;

    let collector_ops_loader: Box<dyn OpsLoader> =
      Box::new(CollectorRegistryOps::new());

    let esm_resolver = EsmModuleResolver::new(EsmResolverOptions {
      loaders: Vec::from([collector_ops_loader]),
    });
    let esm_resolver = Rc::new(RefCell::new(esm_resolver));

    // if cli_config.watch {
    // TODO
    // options.runtime.enable_watch_mode();
    // }

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

      fn clear_collector_context(
        resolver: &mut EsmModuleResolver,
      ) -> Result<(), AnyError> {
        let op_state = resolver.get_op_state()?;
        let mut op_state = op_state.borrow_mut();

        let collector_ctx =
          extract_op_state_mut::<CollectorContext>(&mut op_state)?;

        collector_ctx.clear();

        Ok(())
      }

      async fn run_factory(
        clr: MutRc<NodeCollectorManager>,
        resolver: &mut EsmModuleResolver,
      ) {
        let clr = clr.get_clone().unwrap();
        let node_factory = clr.get_node_factory();

        if let Some(factory) = node_factory {
          resolver.call_v8_function(factory).await.unwrap();
        }
      }

      clear_collector_context(&mut resolver)?;

      #[allow(unused)]
      let module_id = resolver
        .process_esm_file(file_path.display().to_string(), false)
        .await
        .unwrap();

      let op_state = resolver.get_op_state()?;
      let op_state = op_state.borrow();
      let collector_ctx = extract_op_state::<CollectorContext>(&op_state)?;
      let collector_meta = extract_op_state::<CollectorMetadata>(&op_state)?;

      let obtained_collectors = collector_ctx.get_all_collectors();
      let obtained_collectors = obtained_collectors.borrow();

      for collector in obtained_collectors.iter() {
        collector_ctx.set_current_node(collector.clone());

        run_factory(collector.clone(), &mut resolver).await;

        // TODO: rewrite RcMutMutateError
        let collected_node = collector
          .with_mut(|clr| {
            clr
              .collect_node(collector_file.clone())
              .context("manager has been already collected")
          })
          .unwrap()?;

        {
          let running_mode = collected_node.mode.borrow();

          match *running_mode {
            CollectorMode::Only => {
              *collector_meta.only_mode.borrow_mut() = true
            }
            _ => (),
          }
        }

        let mut file_nodes = collector_file.nodes.borrow_mut();
        file_nodes.push(collected_node);
      }

      *collector_file.collected.borrow_mut() = true;

      Ok(collector_file)
    }

    let processed_tasks = Self::collect_test_files(&self.options)?
      .map(|file_path: PathBuf| {
        let esm_resolver = Rc::clone(&esm_resolver);

        create_pinned_future(process_test_file(esm_resolver, file_path))
      })
      .collect();

    if self.options.parallel {
      let files = run_concurrently(processed_tasks).await;
      println!("files: {:#?}", files);
    } else {
      let mut file_map: CollectorFileMap = HashMap::new();

      for task in processed_tasks {
        let file = task().await?;

        let file_path = file.file_path.clone();

        file_map.insert(file_path, file);
      }

      let mut resolver = esm_resolver.borrow_mut();
      let op_state = resolver.get_op_state()?;
      let op_state = op_state.borrow();

      let collector_meta = extract_op_state::<CollectorMetadata>(&op_state)?;
      Self::normalize_mode_settings(&mut file_map, &collector_meta);

      println!("file_map: {:#?}", file_map);
    }

    Ok(())
  }

  fn collect_test_files(
    opts: &TestRunnerOptions,
  ) -> Result<impl Iterator<Item = PathBuf>, AnyError> {
    let TestRunnerOptions { root_dir, includes, excludes, .. } = opts;

    // TODO
    let mut included_cases = Walk::new(&includes, root_dir).build();
    let mut excluded_cases = Walk::new(&excludes, root_dir).build();

    Ok(included_cases.filter(move |included_path| {
      !excluded_cases.any(|excluded_path| excluded_path.eq(included_path))
    }))
  }

  fn normalize_mode_settings(
    file_map: &mut CollectorFileMap,
    meta: &CollectorMetadata,
  ) {
    let only_mode_enabled = *meta.only_mode.borrow();

    fn interpret_only_mode(target_mode: &mut CollectorMode) {
      let updated_mode = match *target_mode {
        CollectorMode::Run => CollectorMode::Skip,
        CollectorMode::Only => CollectorMode::Run,
        rest => rest,
      };

      *target_mode = updated_mode;
    }

    // TODO: parallelism
    for file in file_map.values() {
      let mut nodes = file.nodes.borrow_mut();

      for file_node in nodes.iter_mut() {
        let mut running_mode = file_node.mode.borrow_mut();

        only_mode_enabled.then(|| interpret_only_mode(&mut running_mode));

        let mut node_tasks = file_node.tasks.borrow_mut();
        let mut scoped_only_mode = false;

        node_tasks.iter_mut().for_each(|task| {
          let mut task_mode = task.mode.borrow_mut();
          let mut task_state = task.state.borrow_mut();

          match *running_mode {
            CollectorMode::Skip => *task_mode = CollectorMode::Skip,
            _ => (),
          };

          match *task_mode {
            CollectorMode::Skip => {
              let updated_state = CollectorState::Custom(CollectorMode::Skip);
              *task_state = updated_state
            }
            CollectorMode::Only => scoped_only_mode = true,
            _ => (),
          };
        });

        scoped_only_mode.then(|| {
          node_tasks.iter_mut().for_each(|task| {
            let mut task_mode = task.mode.borrow_mut();

            interpret_only_mode(&mut task_mode);
          })
        });
      }
    }
  }
}
