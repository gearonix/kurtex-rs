use deno_core::extension;

pub struct DenoOpsResolver {
  pub extensions: Vec<deno_core::Extension>,
}

impl DenoOpsResolver {
  #[inline]
  #[must_use]
  pub const fn get_library_snapshot_path() -> &'static [u8] {
    // TODO: move to global variable
    // TODO: test factory
    include_bytes!(concat!(env!("OUT_DIR"), "/KURTEX_SNAPSHOT.bin"))
  }

  pub fn new() -> Self {
    let mut extensions: Vec<deno_core::Extension> = Vec::new();

    extension! {
        KurtexInternalOps,
        ops = [
        node_collector_triggers::op_register_collector_task,
        node_collector_triggers::op_register_collector_node
      ]
    };

    extensions.push(KurtexInternalOps::init_ops());

    DenoOpsResolver { extensions }
  }
}

mod node_collector_triggers {
  use crate::runner::collector::CollectorRunMode;
  use deno_core::error::AnyError;
  use deno_core::{op2, v8};

  #[op2(fast)]
  pub fn op_register_collector_task<'a>(
    #[string] identifier: String,
    callback: v8::Local<'a, v8::Function>,
    #[string] mode: String,
  ) -> Result<(), AnyError> {
    let run_mode = CollectorRunMode::from(mode);

    Ok(())
  }

  #[op2(fast)]
  pub fn op_register_collector_node<'a>(
    #[string] identifier: String,
    factory: v8::Local<'a, v8::Function>,
    #[string] mode: String,
  ) -> Result<(), AnyError> {
    let run_mode = CollectorRunMode::from(mode);

    Ok(())
  }
}
