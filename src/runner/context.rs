use crate::runner::collector::{
  CollectorIdentifier, CollectorRunMode, NodeCollectorManager,
};
use std::rc::Rc;

pub struct CollectorContext {
  nodes: Vec<Rc<NodeCollectorManager>>,
  current_node: Option<Rc<NodeCollectorManager>>,
  file_node: NodeCollectorManager,
}

impl CollectorContext {
  pub fn new() -> Self {
    let file_node = NodeCollectorManager::new(
      CollectorIdentifier::File,
      CollectorRunMode::Run,
    );

    CollectorContext { nodes: Vec::new(), current_node: None, file_node }
  }

  pub fn clear(&mut self) {
    self.nodes.clear();
    self.current_node = None
  }

  pub fn switch_node(&mut self, node: Rc<NodeCollectorManager>) {
    self.nodes.push(Rc::clone(&node));
    self.current_node = Some(Rc::clone(&node))
  }

  pub fn get_current_node(&self) -> &NodeCollectorManager {
    // TODO: rewrite
    if let Some(node) = &self.current_node {
      node.as_ref()
    } else {
      &self.file_node
    }
  }
}
