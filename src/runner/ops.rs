use std::borrow::Cow;

use deno_core::error::AnyError;
use deno_core::v8;

use crate::runner::collector::CollectorRunMode;
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
  
  #[deno_core::op2(fast)]
  fn op_register_collector_task<'a>(
    #[string] identifier: String,
    callback: v8::Local<'a, v8::Function>,
    #[string] mode: String,
  ) -> Result<(), AnyError> {
    let run_mode = CollectorRunMode::from(mode);

    Ok(())
  }

  #[deno_core::op2(fast)]
  fn op_register_collector_node<'a>(
    #[string] identifier: String,
    factory: v8::Local<'a, v8::Function>,
    #[string] mode: String,
  ) -> Result<(), AnyError> {
    let run_mode = CollectorRunMode::from(mode);

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
