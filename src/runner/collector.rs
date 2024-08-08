use deno_core::v8;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
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

#[derive(Clone)]
pub struct NodeCollectorManager {
  task_queue: Vec<Rc<CollectorTask>>,
  collector_node: Rc<CollectorNode>,
  has_collected: bool,
  node_factory: Option<TestCallback>,
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
    node_factory: Option<TestCallback>,
  ) -> Self {
    let task_queue: Vec<Rc<CollectorTask>> = Vec::new();
    let collector_node = Rc::new(CollectorNode {
      identifier,
      mode: RefCell::new(mode),
      ..CollectorNode::default()
    });

    NodeCollectorManager {
      collector_node,
      task_queue,
      has_collected: false,
      node_factory,
    }
  }

  pub fn new_with_factory(
    identifier: CollectorIdentifier,
    mode: CollectorRunMode,
    factory: TestCallback,
  ) -> Self {
    Self::new(identifier, mode, Some(factory))
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

  pub fn reset_state(&mut self) {
    self.task_queue.clear();
    self.has_collected = false;
  }

  pub fn get_node_factory(&self) -> &Option<TestCallback> {
    &self.node_factory
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
      .field("file", &self.file_path)
      .field("collected", &self.collected.borrow())
      .field("nodes", &self.nodes.borrow().iter().map(|n| n))
      .finish()
  }
}

#[derive(Default)]
pub struct CollectorNode {
  pub(crate) identifier: CollectorIdentifier,
  pub(crate) mode: RefCell<CollectorRunMode>,
  tasks: RefCell<Vec<Rc<CollectorTask>>>,
  file: RefCell<Weak<CollectorFile>>,
  status: Option<CollectorSuiteState>,
  hook_manager: RefCell<LifetimeHookManager>,
}

impl std::fmt::Debug for CollectorNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let identifier = match &self.identifier {
      CollectorIdentifier::Custom(e) => &e,
      CollectorIdentifier::File => "file",
    };

    f.debug_struct("CollectorNode")
      .field("name", &identifier)
      .field("tasks", &self.tasks.borrow().iter().map(|n| n))
      .finish()
  }
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

impl std::fmt::Debug for CollectorTask {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorTask")
      .field("name", &self.name)
      .field("mode", &self.mode)
      .finish()
  }
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
