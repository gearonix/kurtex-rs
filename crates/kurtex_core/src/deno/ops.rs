use rccell::RcCell;
use std::borrow::Cow;
use std::rc::Rc;

use crate::collector::{
  CollectorContext, CollectorIdentifier, CollectorMetadata, CollectorMode,
  LifetimeHook, NodeCollectorManager,
};
use crate::TestCallback;

pub trait ExtensionLoader {
  fn load(&self) -> deno_core::Extension;
}

pub struct CollectorRegistryExt;

impl CollectorRegistryExt {
  pub fn new() -> Self {
    CollectorRegistryExt
  }

  #[deno_core::op2]
  #[meta(sanitizer_details = "register new task unit")]
  #[meta(sanitizer_fix = "awaiting identifier and callback")]
  fn op_register_collector_task(
    #[state] collector_ctx: &CollectorContext,
    #[string] identifier: String,
    #[from_v8] callback: TestCallback,
    #[from_v8] run_mode: CollectorMode,
  ) {
    collector_ctx
      .get_current()
      .borrow_mut()
      .register_task(identifier, callback, run_mode)
  }

  #[deno_core::op2]
  #[meta(sanitizer_details = "register new test node (suite)")]
  #[meta(sanitizer_fix = "awaiting identifier and callback")]
  fn op_register_collector_node(
    #[state] collector_ctx: &mut CollectorContext,
    #[from_v8] identifier: CollectorIdentifier,
    #[from_v8] factory: TestCallback,
    #[from_v8] run_mode: CollectorMode,
  ) {
    collector_ctx.register_collector(RcCell::new(
      NodeCollectorManager::new_with_factory(identifier, run_mode, factory),
    ));
  }

  #[deno_core::op2]
  #[meta(sanitizer_details = "register new test lifetime hook")]
  #[meta(sanitizer_fix = "awaiting lifetime hook type and callback")]
  fn op_register_lifetime_hook(
    #[state] collector_ctx: &CollectorContext,
    #[from_v8] lifetime_hook: LifetimeHook,
    #[from_v8] callback: TestCallback,
  ) {
    collector_ctx
      .get_current()
      .borrow_mut()
      .register_lifetime_hook(lifetime_hook, callback);
  }
}

impl ExtensionLoader for CollectorRegistryExt {
  fn load(&self) -> deno_core::Extension {
    const EXTENSION_IDENTIFIER: &'static str = "KurtexInternals";

    let provide_state = Box::new(move |op_state: &mut deno_core::OpState| {
      let collector_ctx = CollectorContext::default();
      let collector_meta = CollectorMetadata::default();

      op_state.put(collector_ctx);
      op_state.put(collector_meta)
    });

    let collector_registry_ops: Vec<deno_core::OpDecl> = vec![
      Self::op_register_collector_task,
      Self::op_register_collector_node,
      Self::op_register_lifetime_hook,
    ]
    .iter()
    .map(|cb| cb())
    .collect();

    deno_core::Extension {
      name: EXTENSION_IDENTIFIER,
      ops: Cow::Owned(collector_registry_ops),
      op_state_fn: Some(provide_state),
      ..deno_core::Extension::default()
    }
  }
}
