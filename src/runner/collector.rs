use deno_core::v8;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, Default)]
pub enum CollectorRunMode {
  #[default]
  Run,
  Skip,
  Only,
  Todo,
}

#[derive(Clone, Copy)]
pub enum CollectorSuiteState {
  Running(CollectorRunMode),
  Skip,
  Pass,
}

impl From<String> for CollectorRunMode {
  fn from(value: String) -> Self {
    match value.as_str() {
      "run" => CollectorRunMode::Run,
      "skip" => CollectorRunMode::Skip,
      "only" => CollectorRunMode::Only,
      "todo" => CollectorRunMode::Todo,
      _ => {
        panic!("Invalid CollectorRunMode variant: '{}'", value)
      }
    }
  }
}

pub struct NodeCollectorManager {
  task_queue: Vec<Rc<CollectorTask>>,
  collector_node: Rc<CollectorNode>,
  has_collected: bool,
  // node_factory: TestCallback,
}

#[derive(Default)]
pub enum CollectorIdentifier {
  #[default]
  File,
  Custom(String),
}

impl NodeCollectorManager {
  pub fn new(
    identifier: CollectorIdentifier,
    mode: CollectorRunMode,
    // factory: TestCallback,
  ) -> Self {
    let task_queue: Vec<Rc<CollectorTask>> = Vec::new();
    let collector_node =
      Rc::new(CollectorNode { identifier, mode, ..CollectorNode::default() });

    NodeCollectorManager {
      collector_node,
      task_queue,
      has_collected: false,
      // node_factory: factory,
    }
  }

  #[inline]
  #[must_use]
  fn should_collect(&self) -> bool {
    !self.has_collected
  }

  #[must_use]
  pub fn collect_node(
    &mut self,
    collector_file: Rc<CollectorFile>,
  ) -> Option<Rc<CollectorNode>> {
    self.should_collect().then(|| {
      self.has_collected = true;
      // TODO: decide what to do here with node factories
      // self.node_factory.;

      *self.collector_node.file.borrow_mut() = Rc::downgrade(&collector_file);
      let tasks_queue = self.task_queue.clone();

      let tasks = tasks_queue
        .into_iter()
        .map(|task| {
          *task.node.borrow_mut() = Rc::downgrade(&self.collector_node);
          *task.file.borrow_mut() = Rc::downgrade(&collector_file);

          task
        })
        .collect();

      *self.collector_node.tasks.borrow_mut() = tasks;

      Rc::clone(&self.collector_node)
    })
  }

  pub fn register_task(
    &mut self,
    name: String,
    callback: TestCallback,
    mode: CollectorRunMode,
  ) {
    let created_task = Rc::new(CollectorTask::new(name, callback, mode));
    self.task_queue.push(created_task);
  }

  pub fn set_lifetime_hook(
    &mut self,
    hook_key: LifetimeHook,
    callback: TestCallback,
  ) {
    let hook_manager = &self.collector_node.hook_manager;
    hook_manager.borrow_mut().add_hook(hook_key, callback)
  }

  pub fn clear_task_queue(&mut self) {
    self.task_queue.clear();
  }
}

#[derive(Default)]
pub struct CollectorFile {
  pub file_path: PathBuf,
  pub collected: RefCell<bool>,
  pub nodes: RefCell<Vec<Rc<CollectorNode>>>,
}

// temporary
impl std::fmt::Debug for CollectorFile {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorFile")
      .field("collected", &self.collected.borrow())
      .finish()
  }
}

#[derive(Default)]
pub struct CollectorNode {
  identifier: CollectorIdentifier,
  mode: CollectorRunMode,
  tasks: RefCell<Vec<Rc<CollectorTask>>>,
  file: RefCell<Weak<CollectorFile>>,
  status: Option<CollectorSuiteState>,
  hook_manager: RefCell<LifetimeHookManager>,
}

type TestCallback = v8::Global<v8::Function>;

pub struct CollectorTask {
  name: String,
  mode: CollectorRunMode,
  node: RefCell<Weak<CollectorNode>>,
  file: RefCell<Weak<CollectorFile>>,
  state: CollectorSuiteState,
  callback: TestCallback,
}

impl CollectorTask {
  pub fn new(
    name: String,
    callback: TestCallback,
    mode: CollectorRunMode,
  ) -> Self {
    CollectorTask {
      name,
      mode,
      file: RefCell::new(Weak::new()),
      node: RefCell::new(Weak::new()),
      state: CollectorSuiteState::Running(mode),
      callback,
    }
  }
}

#[derive(Default)]
pub struct LifetimeHookManager {
  hooks: HashMap<LifetimeHook, Vec<TestCallback>>,
}

impl LifetimeHookManager {
  pub fn new() -> Self {
    let mut hooks: HashMap<LifetimeHook, Vec<TestCallback>> = HashMap::new();

    hooks.insert(LifetimeHook::BeforeAll, Vec::new());
    hooks.insert(LifetimeHook::AfterAll, Vec::new());
    hooks.insert(LifetimeHook::BeforeEach, Vec::new());
    hooks.insert(LifetimeHook::AfterEach, Vec::new());

    LifetimeHookManager { hooks }
  }

  pub fn add_hook(&mut self, hook_key: LifetimeHook, callback: TestCallback) {
    self
      .hooks
      .get_mut(&hook_key)
      .and_then(|partition| {
        partition.push(callback);
        Some(partition)
      })
      .unwrap_or_else(|| panic!("wrong lifetime hook method"));
  }
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum LifetimeHook {
  BeforeAll,
  AfterAll,
  BeforeEach,
  AfterEach,
}
