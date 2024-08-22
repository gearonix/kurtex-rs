use std::rc::Rc;
use std::sync::{Arc, Mutex};

use rayon::prelude::*;
use rccell::RcCell;
use serde::ser::Error;

use crate::reporter::Reporter;
use crate::runner::collector::{RunnerCollectorContext, TestRunnerConfig};
use crate::runtime::KurtexRuntime;
use crate::{
  concurrently, map_pinned_futures, AnyResult, CollectorFile, CollectorMode,
  CollectorNode, CollectorStatus, CollectorTask, LifetimeHook,
};

pub struct TestRunner {
  context: RcCell<RunnerCollectorContext>,
  config: Rc<TestRunnerConfig>,
  runtime: RcCell<KurtexRuntime>,
}

trait CallbackInvoker {
  async fn invoke_lifetime_hook(
    &self,
    node_rc: Arc<Mutex<CollectorNode>>,
    hook_key: LifetimeHook,
  ) -> AnyResult;
  async fn invoke_task(&self, task_rc: Arc<Mutex<CollectorTask>>) -> AnyResult;
}

impl TestRunner {
  pub fn new(
    context: RcCell<RunnerCollectorContext>,
    config: Rc<TestRunnerConfig>,
    runtime: RcCell<KurtexRuntime>,
  ) -> Self {
    TestRunner { context, config, runtime }
  }

  pub async fn run_files(&self) {
    let mut ctx = self.context.borrow_mut();

    ctx.reporter.report_collected();

    for file in ctx.file_map.values() {
      self.run_file(file.clone()).await;
    }
  }

  async fn run_file(&self, file: Arc<CollectorFile>) {
    let mut ctx = self.context.borrow_mut();

    let runnable_nodes = file
      .nodes
      .par_iter()
      .filter(|node| {
        let node = node.lock().unwrap();
        matches!(node.mode, CollectorMode::Run)
      })
      .collect::<Vec<_>>();

    if runnable_nodes.is_empty() {
      return;
    }

    ctx.reporter.begin_file();
    let mut file_nodes = file.nodes.iter();

    // TODO: parallel
    while let Some(node) = file_nodes.next() {
      let node = node.clone();
      self.run_node(node).await;
    }
  }

  async fn run_node(&self, node_rc: Arc<Mutex<CollectorNode>>) {
    let mut ctx = self.context.borrow_mut();
    ctx.reporter.begin_node(node_rc.clone());

    let mut node = node_rc.lock().unwrap();

    match node.mode {
      CollectorMode::Skip | CollectorMode::Todo => {
        node.status = CollectorStatus::Custom(node.mode);
      }
      _ => {}
    }

    let invoked_result: Result<(), anyhow::Error> = try {
      self
        .invoke_lifetime_hook(node_rc.clone(), LifetimeHook::BeforeAll)
        .await?;

      for task in &node.tasks {
        self.run_task(task.clone(), node_rc.clone()).await
      }

      self
        .invoke_lifetime_hook(node_rc.clone(), LifetimeHook::AfterAll)
        .await?;
    };

    if invoked_result.is_err() {
      node.status = CollectorStatus::Fail
    }

    ctx.reporter.end_node(node_rc.clone());
  }

  async fn run_task(
    &self,
    task_rc: Arc<Mutex<CollectorTask>>,
    parent_rc: Arc<Mutex<CollectorNode>>,
  ) {
    let mut ctx = self.context.borrow_mut();
    let mut task = task_rc.lock().unwrap();

    ctx.reporter.begin_task(task_rc.clone());

    if task.mode != CollectorMode::Run {
      ctx.reporter.end_task(task_rc.clone());
      return;
    }

    let invoked_result: AnyResult = try {
      self
        .invoke_lifetime_hook(parent_rc.clone(), LifetimeHook::BeforeEach)
        .await?;
      self.invoke_task(task_rc.clone()).await?;

      task.status = CollectorStatus::Pass;
    };

    if invoked_result.is_err() {
      task.status = CollectorStatus::Fail;
    }

    let after_each = self
      .invoke_lifetime_hook(parent_rc.clone(), LifetimeHook::AfterEach)
      .await;

    if after_each.is_err() {
      task.status = CollectorStatus::Fail;
    }

    ctx.reporter.end_task(task_rc.clone());
  }
}

impl CallbackInvoker for TestRunner {
  // TODO: arguments
  async fn invoke_lifetime_hook(
    &self,
    node_rc: Arc<Mutex<CollectorNode>>,
    hook_key: LifetimeHook,
  ) -> AnyResult {
    let mut rt = self.runtime.borrow_mut();
    let mut node = node_rc.lock().unwrap();
    let hooks_partition = node.hook_manager.get_by(hook_key);

    for hook_fn in hooks_partition {
      if let Err(err) = rt.call_v8_function(hook_fn).await {
        return Err(err.into());
      }
    }

    Ok(())
  }

  async fn invoke_task(&self, task_rc: Arc<Mutex<CollectorTask>>) -> AnyResult {
    let mut rt = self.runtime.borrow_mut();
    let mut task = task_rc.lock().unwrap();
    let task_fn = &task.callback;

    if let Err(e) = rt.call_v8_function(&task_fn).await {
      return Err(e.into());
    }

    Ok(())
  }
}