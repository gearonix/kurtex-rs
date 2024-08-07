use std::borrow::Cow;
use std::rc::Rc;

use crate::deno::module_resolver::extract_op_state;
use deno_core::anyhow::Context;
use deno_core::error::AnyError;
use deno_core::{v8, OpState};
use mut_rc::MutRc;

use crate::runner::collector::{
  CollectorIdentifier, CollectorRunMode, NodeCollectorManager,
};
use crate::runner::context::CollectorContext;

pub struct BindingsResolver {
  pub bindings: Vec<deno_core::Extension>,
}

impl BindingsResolver {
  #[inline]
  #[must_use]
  pub const fn get_library_snapshot_path() -> &'static [u8] {
    // TODO: move to global variable
    // TODO: test factory
    include_bytes!(concat!(env!("OUT_DIR"), "/KURTEX_SNAPSHOT.bin"))
  }
}

pub struct CollectorRegistryOps;

pub trait OpsLoader {
  fn load(&self) -> deno_core::Extension;
}

impl CollectorRegistryOps {
  pub fn new() -> Self {
    CollectorRegistryOps {}
  }

  #[deno_core::op2]
  fn op_register_collector_task(
    op_state: &mut OpState,
    #[string] identifier: String,
    #[global] callback: v8::Global<v8::Function>,
    #[string] mode: String,
  ) -> Result<(), AnyError> {
    let run_mode = CollectorRunMode::from(mode);

    let collector_ctx = op_state
      .try_borrow_mut::<CollectorContext>()
      .context("error while accessing collector context")?;

    let current_node = collector_ctx.get_current_node();

    current_node
      .with_mut(|node| {
        node.register_task(identifier, callback, run_mode);
      })
      .unwrap();

    Ok(())
  }

  #[deno_core::op2]
  fn op_register_collector_node<'a>(
    op_state: &mut OpState,
    #[string] identifier: String,
    #[global] factory: v8::Global<v8::Function>,
    #[string] mode: String,
  ) -> Result<(), AnyError> {
    let identifier = CollectorIdentifier::Custom(identifier);
    let run_mode = CollectorRunMode::from(mode);

    let node_collector =
      MutRc::new(NodeCollectorManager::new(identifier, run_mode));
    let collector_ctx = extract_op_state::<CollectorContext>(op_state)?;

    collector_ctx.register_node(node_collector);

    // TODO: return type
    Ok(())
  }
}

impl OpsLoader for CollectorRegistryOps {
  fn load(&self) -> deno_core::Extension {
    let provide_state = Box::new(move |op_state: &mut deno_core::OpState| {
      let collector_ctx = CollectorContext::new();

      op_state.put(collector_ctx);
    });

    let collector_registry_ops: Vec<deno_core::OpDecl> = Vec::from([
      Self::op_register_collector_task,
      Self::op_register_collector_node,
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
