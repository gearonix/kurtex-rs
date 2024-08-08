use std::borrow::Cow;

use deno_core::anyhow::Context;
use deno_core::error::AnyError;
use deno_core::{v8, OpState, ResourceId};
use mut_rc::MutRc;

use crate::deno::module_resolver::{extract_op_state, extract_op_state_mut};
use crate::runner::collector::{
  CollectorIdentifier, CollectorMode, NodeCollectorManager,
};
use crate::runner::context::{CollectorContext, CollectorMetadata};

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

// TODO: AsyncRefCell

impl CollectorRegistryOps {
  pub fn new() -> Self {
    CollectorRegistryOps {}
  }

  #[deno_core::op2]
  // TODO
  #[meta(sanitizer_details = "")]
  #[meta(sanitizer_fix = "")]
  fn op_register_collector_task(
    scope: &mut v8::HandleScope,
    op_state: &OpState,
    #[string] identifier: String,
    #[global] callback: v8::Global<v8::Function>,
    #[string] mode: String,
  ) -> Result<(), AnyError> {
    let run_mode = CollectorMode::from(mode);

    let collector_ctx = extract_op_state::<CollectorContext>(op_state)?;
    let current_node = collector_ctx.get_current_node();
    let current_node = current_node.borrow_mut();

    current_node
      .with_mut(|task| task.register_task(identifier, callback, run_mode))
      .unwrap();

    Ok(())
  }

  #[deno_core::op2]
  // TODO
  #[meta(sanitizer_details = "")]
  #[meta(sanitizer_fix = "")]
  fn op_register_collector_node<'a>(
    op_state: &OpState,
    #[string] identifier: String,
    #[global] factory: v8::Global<v8::Function>,
    #[string] mode: String,
  ) -> Result<(), AnyError> {
    let identifier = CollectorIdentifier::Custom(identifier);
    let run_mode = CollectorMode::from(mode);

    let node_collector = MutRc::new(NodeCollectorManager::new_with_factory(
      identifier, run_mode, factory,
    ));
    let collector_ctx = extract_op_state::<CollectorContext>(op_state)?;

    collector_ctx.register_node(node_collector);

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
