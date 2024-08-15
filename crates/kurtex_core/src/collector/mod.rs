use std::cell::RefCell;
use std::rc::Rc;



pub mod structures;
pub mod context;

pub use structures::*;
pub use context::*;

#[derive(Clone)]
pub struct NodeCollectorManager {
  task_queue: Vec<Rc<CollectorTask>>,
  collector_node: Rc<CollectorNode>,
  has_collected: bool,
  node_factory: Option<TestCallback>,
  on_file_level: bool,
}

impl NodeCollectorManager {
  pub fn new_with_file() -> Self {
    NodeCollectorManager {
      on_file_level: true,
      ..Self::new(CollectorIdentifier::File, CollectorMode::Run, None)
    }
  }

  pub fn new(
    identifier: CollectorIdentifier,
    mode: CollectorMode,
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
      on_file_level: false,
      node_factory,
    }
  }

  pub fn new_with_factory(
    identifier: CollectorIdentifier,
    mode: CollectorMode,
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
    mode: CollectorMode,
  ) {
    let created_task = Rc::new(CollectorTask::new(name, callback, mode));
    self.task_queue.push(created_task);
  }

  pub fn register_lifetime_hook(
    &mut self,
    hook_key: LifetimeHook,
    callback: TestCallback,
  ) {
    let hook_manager = &self.collector_node.hook_manager;
    hook_manager.borrow_mut().add_hook(hook_key, callback)
  }

  pub fn reset_state(&mut self) {
    self
      .on_file_level
      .then(|| {
        self.task_queue.clear();
        self.has_collected = false;

        let node = &self.collector_node;
        let identifier = node.identifier.clone();
        let mode = node.mode.clone();

        self.collector_node = Rc::new(CollectorNode {
          identifier,
          mode,
          ..CollectorNode::default()
        });
      })
      .unwrap_or_else(|| {
        panic!("Resetting state is only allowed when on_file_level is true.")
      })
  }

  pub fn get_node_factory(&self) -> &Option<TestCallback> {
    &self.node_factory
  }
}
