use std::cell::RefCell;

use mut_rc::MutRc;

use crate::runner::collector::NodeCollectorManager;

pub struct CollectorContext {
  nodes: RefCell<Vec<MutRc<NodeCollectorManager>>>,
  current_node: RefCell<MutRc<NodeCollectorManager>>,
  default_node: MutRc<NodeCollectorManager>,
}

#[derive(Default)]
pub struct CollectorMetadata {
  pub(crate) only_mode: RefCell<bool>,
}

impl CollectorContext {
  pub fn new() -> Self {
    let file_node = MutRc::new(NodeCollectorManager::new_with_file());
    let nodes = RefCell::new(Vec::new());

    CollectorContext {
      nodes,
      current_node: RefCell::new(file_node.clone()),
      default_node: file_node,
    }
  }

  pub fn register_node(&self, new_node: MutRc<NodeCollectorManager>) {
    self.nodes.borrow_mut().push(new_node.clone());
    self.set_current_node(new_node);
  }

  pub fn set_current_node(&self, new_node: MutRc<NodeCollectorManager>) {
    *self.current_node.borrow_mut() = new_node
  }

  pub fn get_current_node(&self) -> RefCell<MutRc<NodeCollectorManager>> {
    self.current_node.clone()
  }

  pub fn clear(&mut self) {
    self.nodes.borrow_mut().clear();
    self.default_node.with_mut(|t| t.reset_state()).unwrap();
    self.set_current_node(self.default_node.clone())
  }

  pub fn get_all_collectors(
    &self,
  ) -> RefCell<Vec<MutRc<NodeCollectorManager>>> {
    let all_nodes = self.nodes.clone();
    all_nodes.borrow_mut().push(self.default_node.clone());

    all_nodes
  }
}
