use deno_core::extension;
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let target_snapshot_path =
    PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("KURTEX_SNAPSHOT.bin");
  
  // TODO: rewrite
  extension!(
    KurtexInternals,
    js = ["../../packages/kurtex/dist/deno-bindings.mjs"],
    docs = "Kurtex internal bindings"
  );

  let snapshot = deno_core::snapshot::create_snapshot(
    deno_core::snapshot::CreateSnapshotOptions {
      cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
      startup_snapshot: None,
      skip_op_registration: false,
      extensions: vec![KurtexInternals::init_ops_and_esm()],
      with_runtime_cb: None,
      extension_transpiler: None,
    },
    None,
  )
  .unwrap();

  Ok(std::fs::write(target_snapshot_path, snapshot.output)?)
}
