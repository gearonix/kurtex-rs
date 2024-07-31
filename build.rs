use deno_core::extension;
use std::env;
use std::path::PathBuf;

fn main() {
  // TODO: improve
  extension!(kurtex, js = ["bindings/kurtex.mjs"], docs = "kurtex runtime");

  let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
  // TODO: move to global variable or env var
  let snapshot_path = out_dir.join("KURTEX_SNAPSHOT.bin");

  println!("snapshot_path: {:?}", snapshot_path);

  let snapshot = deno_core::snapshot::create_snapshot(
    deno_core::snapshot::CreateSnapshotOptions {
      cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
      startup_snapshot: None,
      skip_op_registration: false,
      extensions: vec![kurtex::init_ops_and_esm()],
      with_runtime_cb: None,
      extension_transpiler: None,
    },
    None,
  )
  .unwrap();

  std::fs::write(snapshot_path, snapshot.output).unwrap()
}
