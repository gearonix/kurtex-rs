use std::borrow::Cow;

use deno_core::error::AnyError;
use deno_core::v8;
use mut_rc::MutRc;

use crate::collector::{
  CollectorContext, CollectorIdentifier, CollectorMetadata, CollectorMode,
  LifetimeHook, NodeCollectorManager,
};

pub trait OpsLoader {
  fn load(&self) -> deno_core::Extension;
}

pub struct CollectorRegistryOps;

impl CollectorRegistryOps {
  pub fn new() -> Self {
    CollectorRegistryOps
  }

  #[deno_core::op2]
  #[meta(sanitizer_details = "register new task unit")]
  #[meta(sanitizer_fix = "awaiting identifier and callback")]
  fn op_register_collector_task(
    #[state] collector_ctx: &CollectorContext,
    #[string] identifier: String,
    #[global] callback: v8::Global<v8::Function>,
    #[from_v8] run_mode: CollectorMode,
  ) -> Result<(), AnyError> {
    let current_node = collector_ctx.get_current_node();
    let current_node = current_node.borrow_mut();

    current_node
      .with_mut(|node| node.register_task(identifier, callback, run_mode))
      .unwrap();

    Ok(())
  }

  #[deno_core::op2]
  #[meta(sanitizer_details = "register new test node (suite)")]
  #[meta(sanitizer_fix = "awaiting identifier and callback")]
  fn op_register_collector_node<'a>(
    #[state] collector_ctx: &CollectorContext,
    #[from_v8] identifier: CollectorIdentifier,
    #[global] factory: v8::Global<v8::Function>,
    #[from_v8] run_mode: CollectorMode,
  ) -> Result<(), AnyError> {
    let node_collector = MutRc::new(NodeCollectorManager::new_with_factory(
      identifier, run_mode, factory,
    ));

    collector_ctx.register_node(node_collector);

    Ok(())
  }

  #[deno_core::op2]
  #[meta(sanitizer_details = "register new test lifetime hook")]
  #[meta(sanitizer_fix = "awaiting lifetime hook type and callback")]
  fn op_register_lifetime_hook<'a>(
    #[state] collector_ctx: &CollectorContext,
    #[from_v8] lifetime_hook: LifetimeHook,
    #[global] callback: v8::Global<v8::Function>,
  ) -> Result<(), AnyError> {
    let current_node = collector_ctx.get_current_node();
    let current_node = current_node.borrow_mut();

    current_node
      .with_mut(|node| node.register_lifetime_hook(lifetime_hook, callback))
      .unwrap();

    Ok(())
  }
}

impl OpsLoader for CollectorRegistryOps {
  fn load(&self) -> deno_core::Extension {
    let provide_state = Box::new(move |op_state: &mut deno_core::OpState| {
      let collector_ctx = CollectorContext::new();
      let collector_meta = CollectorMetadata::default();

      op_state.put(collector_ctx);
      op_state.put(collector_meta)
    });

    let collector_registry_ops: Vec<deno_core::OpDecl> = Vec::from([
      Self::op_register_collector_task,
      Self::op_register_collector_node,
      Self::op_register_lifetime_hook,
    ])
    .iter()
    .map(|cb| cb())
    .collect();

    deno_core::Extension {
      name: "KurtexInternals",
      ops: Cow::Owned(collector_registry_ops),
      op_state_fn: Some(provide_state),
      ..deno_core::Extension::default()
    }
  }
}
