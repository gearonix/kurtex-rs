use rccell::RcCell;

use crate::collector::NodeCollectorManager;

pub struct CollectorContext {
  collectors: Vec<RcCell<NodeCollectorManager>>,
  current: RcCell<NodeCollectorManager>,
}

#[derive(Default)]
pub struct CollectorMetadata {
  pub(crate) only_mode: bool,
}

impl CollectorContext {
  pub fn register_collector(&mut self, new_node: RcCell<NodeCollectorManager>) {
    self.collectors.push(new_node.clone());
    self.set_current(new_node);
  }

  pub fn set_current(&mut self, new_node: RcCell<NodeCollectorManager>) {
    self.current = new_node
  }

  pub fn get_current(&self) -> RcCell<NodeCollectorManager> {
    self.current.clone()
  }

  pub fn acquire_collectors(&self) -> Vec<RcCell<NodeCollectorManager>> {
    self.collectors.clone()
  }
}

impl Default for CollectorContext {
  fn default() -> Self {
    let node = RcCell::new(NodeCollectorManager::default());
    CollectorContext { collectors: vec![node.clone()], current: node }
  }
}
