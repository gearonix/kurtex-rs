use std::cell::RefCell;

use mut_rc::MutRc;

use crate::runner::collector::{
  CollectorIdentifier, CollectorRunMode, NodeCollectorManager,
};

pub struct CollectorContext {
  nodes: RefCell<Vec<MutRc<NodeCollectorManager>>>,
  current_node: RefCell<MutRc<NodeCollectorManager>>,
  default_node: MutRc<NodeCollectorManager>,
}

impl CollectorContext {
  pub fn new() -> Self {
    let file_node = MutRc::new(NodeCollectorManager::new(
      CollectorIdentifier::File,
      CollectorRunMode::Run,
      None,
    ));

    CollectorContext {
      nodes: RefCell::new(vec![file_node.clone()]),
      current_node: RefCell::new(file_node.clone()),
      default_node: file_node,
    }
  }

  pub fn register_node(&self, new_node: MutRc<NodeCollectorManager>) {
    self.nodes.borrow_mut().push(new_node.clone());
    *self.current_node.borrow_mut() = new_node.clone()
  }

  pub fn get_current_node(&self) -> RefCell<MutRc<NodeCollectorManager>> {
    self.current_node.clone()
  }

  pub fn clear(&mut self) {
    self.nodes.borrow_mut().clear();
    self.default_node.with_mut(|t| t.reset_state()).unwrap();
    *self.current_node.borrow_mut() = self.default_node.clone()
  }

  pub fn get_all_collectors(&self) -> RefCell<Vec<MutRc<NodeCollectorManager>>> {
    let all_nodes = self.nodes.clone();
    all_nodes.borrow_mut().push(self.default_node.clone());

    all_nodes
  }
}
