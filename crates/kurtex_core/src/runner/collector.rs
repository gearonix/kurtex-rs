use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use deno_core::error::AnyError;
use hashbrown::HashMap;
use log::debug;
use rayon::prelude::*;
use rccell::RcCell;

use crate::collector::context::{CollectorContext, CollectorMetadata};
use crate::collector::structures::{
  CollectorFile, CollectorMode, CollectorNode, CollectorStatus,
  CollectorTask,
};
use crate::deno::runtime::KurtexRuntime;
use crate::error::AnyResult;
use crate::reporter::{KurtexDefaultReporter, Reporter};
use crate::walk::Walk;
use crate::{
  arc, arc_mut, concurrently, map_pinned_futures, KurtexConfig,
};

#[derive(Default, Debug)]
pub struct TestRunnerConfig {
  pub watch: bool,
  pub globals: bool,
  pub parallel: bool,
  pub config_path: PathBuf,
  pub root_dir: PathBuf,
  pub includes: Vec<String>,
  pub excludes: Vec<String>,
}

impl TestRunnerConfig {
  pub fn adjust_config_file(&mut self, config: KurtexConfig) {
    config.parallel.map(|par| self.parallel = par);
    config.watch.map(|watch| self.watch = watch);

    self.includes = config.includes;
    self.excludes = config.excludes;
  }
}

pub type CollectorFileMap = HashMap<PathBuf, Arc<CollectorFile>>;

#[derive(Default)]
pub enum RunnerContextState {
  #[default]
  Inactive,
  Ready,
}

// TODO: Mutex performance cost
#[derive(Default)]
pub struct RunnerCollectorContext {
  pub files: Vec<Arc<CollectorFile>>,
  pub file_map: CollectorFileMap,
  pub nodes: Vec<Arc<Mutex<CollectorNode>>>,
  pub tasks: Vec<Arc<Mutex<CollectorTask>>>,
  pub reporter: KurtexDefaultReporter,
  pub state: RunnerContextState,
}

impl RunnerCollectorContext {
  pub fn set_ready(&mut self) {
    self.state = RunnerContextState::Ready
  }

  pub fn is_ready(&self) -> bool {
    matches!(self.state, RunnerContextState::Ready)
  }
}

pub struct FileCollector {
  config: Rc<TestRunnerConfig>,
  runtime: RcCell<KurtexRuntime>,
}

#[derive(Default)]
pub struct FileCollectorOptions {
  pub(crate) existing_paths: Vec<PathBuf>,
}

impl FileCollector {
  pub fn new(
    config: Rc<TestRunnerConfig>,
    runtime: RcCell<KurtexRuntime>,
  ) -> Self {
    FileCollector { config, runtime }
  }

  pub async fn run(
    &self,
    opts: FileCollectorOptions,
  ) -> AnyResult<RcCell<RunnerCollectorContext>> {
    async fn process_test_file(
      file_path: PathBuf,
      runtime: RcCell<KurtexRuntime>,
      collector_ctx: RcCell<RunnerCollectorContext>,
    ) -> AnyResult<Arc<CollectorFile>> {
      let obtained_collectors = {
        let mut runtime = runtime.borrow_mut();

        runtime.mutate_state(|ctx: &mut CollectorContext| {
          *ctx = Default::default();
        })?;

        #[allow(unused)]
        let _ = runtime
          .resolve_test_module(file_path.display().to_string())
          .await;

        runtime
          .get_state(|ctx: &CollectorContext| ctx.acquire_collectors())?
      };

      let mut collector_file = CollectorFile::from_path(file_path);

      for collector in obtained_collectors {
        let mut runtime = runtime.borrow_mut();
        let mut collector_ctx = collector_ctx.borrow_mut();

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
          if let Err(e) = runtime.call_v8_function(&factory).await {
            debug!(
              "Got error on file {}.",
              collector_file.file_path.display()
            );

            collector_file.error = Some(e)
          }
        }
        let collected_node_rc = collector.borrow_mut().collect_node();
        let collected_node = collected_node_rc.lock().unwrap();

        runtime.mutate_state_with(
          &collected_node,
          |inner_node, meta: &mut CollectorMetadata| match inner_node
            .mode
          {
            CollectorMode::Only => meta.only_mode = true,
            _ => (),
          },
        )?;

        let runner_tasks =
          collected_node.tasks.iter().map(|m| m.clone());
        collector_ctx.tasks.extend(runner_tasks);

        drop(collected_node);

        collector_ctx.nodes.push(collected_node_rc.clone());
        collector_file.nodes.push(collected_node_rc);
      }

      collector_file.collected = true;
      let collector_file = arc!(collector_file);

      let mut runner_ctx = collector_ctx.borrow_mut();
      runner_ctx.files.push(collector_file.clone());

      Ok(collector_file)
    }
    let collector_ctx = RcCell::new(RunnerCollectorContext::default());

    let target_files = if opts.existing_paths.is_empty() {
      Self::collect_test_files(&self.config)
    } else {
      opts.existing_paths
    };
    let processed_files = map_pinned_futures!(
      target_files,
      process_test_file(runtime, collector_ctx),
      {
        runtime = self.runtime.clone()
        collector_ctx = collector_ctx.clone()
      }
    );

    collector_ctx.borrow_mut().reporter.start();

    let mut file_map: CollectorFileMap = if self.config.parallel {
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

    let mut runtime = self.runtime.borrow_mut();
    runtime.get_state_with(
      &mut file_map,
      |fm, meta: &CollectorMetadata| {
        Self::normalize_mode_settings(fm, &meta);
      },
    )?;

    {
      let mut context = collector_ctx.borrow_mut();
      context.file_map = file_map;
      context.set_ready();
    }

    Ok(collector_ctx)
  }

  fn collect_test_files(opts: &TestRunnerConfig) -> Vec<PathBuf> {
    let TestRunnerConfig { root_dir, includes, excludes, .. } = opts;
    let included_cases = Walk::new(&includes, root_dir).build();
    let mut excluded_cases = Walk::new(&excludes, root_dir).build();

    // TODO: rewrite: **/node_modules/**, parallel
    // TODO: build times
    included_cases
      .filter(move |included_path| {
        !excluded_cases
          .any(|excluded_path| excluded_path.eq(included_path))
      })
      .collect()
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

    let _ = file_map.par_values().for_each(|file| {
      file.nodes.par_iter().for_each(|node| {
        let mut node = node.lock().unwrap();

        meta.only_mode.then(|| interpret_only_mode(&mut node.mode));
        let mut scoped_only_mode = false;

        for task in &node.tasks {
          let mut task = task.lock().unwrap();

          match node.mode {
            CollectorMode::Skip => task.mode = CollectorMode::Skip,
            _ => (),
          };

          match task.mode {
            CollectorMode::Skip => {
              task.status = CollectorStatus::Custom(CollectorMode::Skip);
            }
            CollectorMode::Only => scoped_only_mode = true,
            _ => (),
          };
        }

        scoped_only_mode.then(|| {
          node.tasks.iter().for_each(|task| {
            let mut task = task.lock().unwrap();
            interpret_only_mode(&mut task.mode);
          })
        });
      })
    });
  }
}
