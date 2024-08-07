use mut_rc::MutRc;

use crate::runner::collector::{
  CollectorIdentifier, CollectorRunMode, NodeCollectorManager,
};

pub struct CollectorContext {
  nodes: Vec<MutRc<NodeCollectorManager>>,
  current_node: MutRc<NodeCollectorManager>,
  default_node: MutRc<NodeCollectorManager>,
}

impl CollectorContext {
  pub fn new() -> Self {
    let file_node = MutRc::new(NodeCollectorManager::new(
      CollectorIdentifier::File,
      CollectorRunMode::Run,
    ));

    CollectorContext {
      nodes: vec![file_node.clone()],
      current_node: file_node.clone(),
      default_node: file_node,
    }
  }

  pub fn register_node(&mut self, node: MutRc<NodeCollectorManager>) {
    self.nodes.push(node.clone());
    self.current_node = node.clone()
  }

  pub fn get_current_node(&self) -> MutRc<NodeCollectorManager> {
    self.current_node.clone()
  }

  pub fn clear(&mut self) {
    self.nodes.clear();
    self.default_node.with_mut(|t| t.reset_state()).unwrap();
    self.current_node = self.default_node.clone()
  }

  pub fn get_all_nodes(&self) -> Vec<MutRc<NodeCollectorManager>> {
    let mut all_nodes = self.nodes.clone();
    let default_node = self.default_node.clone();
    all_nodes.push(default_node);
    
    all_nodes
  }
}
