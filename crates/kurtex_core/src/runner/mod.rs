use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use deno_core::error::AnyError;
use deno_core::futures::StreamExt;
use hashbrown::HashMap;
use rayon::prelude::*;
use rccell::RcCell;

use crate::collector::{
  CollectorContext, CollectorFile, CollectorMetadata, CollectorMode,
  CollectorState, NodeCollectorManager,
};
use crate::config::loader::KurtexConfig;
use crate::deno::ops::{CollectorRegistryExt, ExtensionLoader};
use crate::deno::runtime::{KurtexRuntime, KurtexRuntimeOptions};
use crate::error::AnyResult;
use crate::util::tokio::{create_pinned_future, run_concurrently};
use crate::walk::Walk;
use crate::{arc, arc_mut, concurrently, map_pinned_futures, CollectorNode};

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
    config.parallel.map(|par| self.parallel = par);
    config.watch.map(|watch| self.watch = watch);

    self.includes = config.includes;
    self.excludes = config.excludes;
  }
}

#[derive(Default, Debug)]
pub struct RuntimeOptions {
  runtime_snapshot: &'static [u8],
}

impl RuntimeOptions {
  pub fn new_from_snapshot(runtime_snapshot: &'static [u8]) -> Self {
    RuntimeOptions { runtime_snapshot }
  }
}

type CollectorFileMap = HashMap<PathBuf, Arc<CollectorFile>>;

pub struct TestRunner {
  options: TestRunnerOptions,
  runtime: RuntimeOptions,
}

impl TestRunner {
  pub fn new(options: TestRunnerOptions, runtime: RuntimeOptions) -> Self {
    TestRunner { options, runtime }
  }

  pub async fn run(&self) -> AnyResult {
    let collector_ops_loader: Box<dyn ExtensionLoader> =
      Box::new(CollectorRegistryExt::new());

    let runtime = RcCell::new(KurtexRuntime::new(KurtexRuntimeOptions {
      loaders: vec![collector_ops_loader],
      snapshot: self.runtime.runtime_snapshot,
    }));

    async fn process_test_file(
      file_path: PathBuf,
      runtime: RcCell<KurtexRuntime>,
    ) -> AnyResult<Arc<CollectorFile>> {
      let obtained_collectors = {
        let mut runtime = runtime.borrow_mut();

        runtime.mutate_state(|ctx: &mut CollectorContext| {
          *ctx = Default::default();
        })?;

        #[allow(unused)]
        let module_id = runtime
          .process_esm_file(file_path.display().to_string(), false)
          .await
          .unwrap();

        runtime.get_state(|ctx: &CollectorContext| ctx.acquire_collectors())?
      };

      async fn run_collector(
        collector: RcCell<NodeCollectorManager>,
        runtime: RcCell<KurtexRuntime>,
      ) -> AnyResult<Arc<Mutex<CollectorNode>>> {
        let mut runtime = runtime.borrow_mut();

        runtime.mutate_state_with(
          collector.clone(),
          |clr, ctx: &mut CollectorContext| {
            ctx.set_current(clr);
          },
        )?;

        let node_factory = {
          let clr = collector.borrow_mut();
          clr.get_node_factory()
        };

        if let Some(factory) = node_factory {
          runtime.call_v8_function(&factory).await.unwrap();
        }

        let collected_node = collector.borrow_mut().collect_node();

        #[rustfmt::skip]
        runtime.mutate_state_with(
          &collected_node,
          |CollectorNode { mode, .. },
           meta: &mut CollectorMetadata| match mode {
            CollectorMode::Only => meta.only_mode = true,
            _ => (),
          },
        )?;

        Ok(arc_mut!(collected_node).clone())
      }

      let nodes = concurrently!(obtained_collectors, run_collector(runtime), {
        runtime = runtime.clone()
      });

      Ok(arc!(CollectorFile { file_path, collected: true, nodes }))
    }

    let processed_files = map_pinned_futures!(
      Self::collect_test_files(&self.options)?,
      process_test_file(runtime),
      { runtime = runtime.clone() }
    );

    let mut file_map: CollectorFileMap = if self.options.parallel {
      concurrently!(processed_files)
        .into_iter()
        .map(|task| (task.file_path.clone(), task))
        .collect()
    } else {
      let mut file_map: CollectorFileMap = HashMap::new();

      for task in processed_files {
        let file = task().await?;
        file_map.insert(file.file_path.clone(), file);
      }
      file_map
    };

    let mut runtime = runtime.borrow_mut();
    runtime.get_state_with(&mut file_map, |fm, meta: &CollectorMetadata| {
      Self::normalize_mode_settings(fm, &meta);
    })?;

    println!("file_map: {:#?}", file_map);

    Ok(())
  }

  fn collect_test_files(
    opts: &TestRunnerOptions,
  ) -> Result<impl Iterator<Item = PathBuf>, AnyError> {
    let TestRunnerOptions { root_dir, includes, excludes, .. } = opts;

    let included_cases = Walk::new(&includes, root_dir).build();
    let mut excluded_cases = Walk::new(&excludes, root_dir).build();

    // TODO rewrite: **/node_modules/**
    // TODO: parallel
    Ok(included_cases.filter(move |included_path| {
      !excluded_cases.any(|excluded_path| excluded_path.eq(included_path))
    }))
  }

  fn normalize_mode_settings(
    file_map: &mut CollectorFileMap,
    meta: &CollectorMetadata,
  ) {
    fn interpret_only_mode(target_mode: &mut CollectorMode) {
      let updated_mode = match *target_mode {
        CollectorMode::Run => CollectorMode::Skip,
        CollectorMode::Only => CollectorMode::Run,
        rest => rest,
      };

      *target_mode = updated_mode;
    }

    let _ = file_map.par_values().map(|file| {
      file.nodes.par_iter().for_each(|node| {
        let mut node = node.lock().unwrap();

        meta.only_mode.then(|| interpret_only_mode(&mut node.mode));

        let scoped_only_mode = false;
        node.tasks.par_iter().for_each_with(
          scoped_only_mode,
          |scoped, task| {
            let mut task = task.lock().unwrap();

            match node.mode {
              CollectorMode::Skip => task.mode = CollectorMode::Skip,
              _ => (),
            };

            match task.mode {
              CollectorMode::Skip => {
                let updated_state = CollectorState::Custom(CollectorMode::Skip);
                task.state = updated_state
              }
              CollectorMode::Only => *scoped = true,
              _ => (),
            };
          },
        );

        scoped_only_mode.then(|| {
          node.tasks.par_iter().for_each(|task| {
            let mut task = task.lock().unwrap();
            interpret_only_mode(&mut task.mode);
          })
        });
      })
    });
  }
}
