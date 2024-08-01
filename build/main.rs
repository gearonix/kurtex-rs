use deno_core::extension;
use std::env;
use std::path::PathBuf;

mod transpile_ts;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let runtime_binding_path = PathBuf::from(
    env::current_dir()?.join("packages/kurtex/src/deno-bindings.ts"),
  );

  let output_bindings_path =
    PathBuf::from(env::temp_dir()).join("kurtex-tmp/kurtex_deno_bindings.js");

  let output_bindings_path =
    PathBuf::from(env::current_dir()?.join("dev/tmp/kurtex_deno_bindings.js"));

  let target_snapshot_path =
    PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("KURTEX_SNAPSHOT.bin");

  transpile_ts::transpile_typescript_file(
    &runtime_binding_path,
    &output_bindings_path,
  );
  
  assert!(output_bindings_path.exists());

  let extension_path = output_bindings_path.to_str().unwrap();

  extension!(
    KurtexInternals,
    js = ["dev/tmp/kurtex_deno_bindings.js"],
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
