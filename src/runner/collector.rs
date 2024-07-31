use std::cell::RefCell;
use std::collections::HashMap;
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
}

impl NodeCollectorManager {
  pub fn new(identifier: String, mode: CollectorRunMode) -> Self {
    let task_queue: Vec<Rc<CollectorTask>> = Vec::new();
    let collector_node =
      Rc::new(CollectorNode { identifier, mode, ..CollectorNode::default() });

    NodeCollectorManager { collector_node, task_queue, has_collected: false }
  }

  pub fn process_task<C>(
    &mut self,
    name: String,
    callback: C,
    mode: CollectorRunMode,
  ) where
    C: Fn() -> () + 'static,
  {
    let created_task = Rc::new(CollectorTask::new(name, callback, mode));
    self.task_queue.push(created_task);
  }

  #[inline]
  #[must_use]
  fn should_collect(&self) -> bool {
    !self.has_collected
  }

  #[must_use]
  pub fn collect_node(
    &self,
    collector_file: Rc<CollectorFile>,
  ) -> Option<Rc<CollectorNode>> {
    self.should_collect().then(|| {
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

  pub fn set_lifetime_hook(
    &mut self,
    hook_key: LifetimeHook,
    callback: Callback,
  ) {
    let hook_manager = &self.collector_node.hook_manager;
    hook_manager.borrow_mut().add_hook(hook_key, callback)
  }

  pub fn clear_task_queue(&mut self) {
    self.task_queue.clear();
  }
}

pub struct CollectorFile {
  file_path: PathBuf,
  collected: bool,
  nodes: RefCell<Vec<Rc<CollectorNode>>>,
}

#[derive(Default)]
pub struct CollectorNode {
  identifier: String,
  mode: CollectorRunMode,
  tasks: RefCell<Vec<Rc<CollectorTask>>>,
  file: RefCell<Weak<CollectorFile>>,
  status: Option<CollectorSuiteState>,
  hook_manager: RefCell<LifetimeHookManager>,
}

// TODO temporary
type Callback = Box<dyn Fn() -> ()>; // TODO error

pub struct CollectorTask {
  name: String,
  mode: CollectorRunMode,
  node: RefCell<Weak<CollectorNode>>,
  file: RefCell<Weak<CollectorFile>>,
  state: CollectorSuiteState,
  callback: Callback,
}

impl CollectorTask {
  pub fn new<C>(name: String, callback: C, mode: CollectorRunMode) -> Self
  where
    C: Fn() -> () + 'static,
  {
    CollectorTask {
      name,
      mode,
      file: RefCell::new(Weak::new()),
      node: RefCell::new(Weak::new()),
      state: CollectorSuiteState::Running(mode),
      callback: Box::new(callback),
    }
  }
}

#[derive(Default)]
pub struct LifetimeHookManager {
  hooks: HashMap<LifetimeHook, Vec<Callback>>,
}

impl LifetimeHookManager {
  pub fn new() -> Self {
    let mut hooks: HashMap<LifetimeHook, Vec<Callback>> = HashMap::new();

    hooks.insert(LifetimeHook::BeforeAll, Vec::new());
    hooks.insert(LifetimeHook::AfterAll, Vec::new());
    hooks.insert(LifetimeHook::BeforeEach, Vec::new());
    hooks.insert(LifetimeHook::AfterEach, Vec::new());

    LifetimeHookManager { hooks }
  }

  pub fn add_hook(&mut self, hook_key: LifetimeHook, callback: Callback) {
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
