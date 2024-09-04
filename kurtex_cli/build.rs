use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::{env, fs};

use deno_core::v8::OneByteConst;
use deno_core::{anyhow, FastStaticString, RuntimeOptions};
use swc_common::{
  self, comments::SingleThreadedComments, errors::Handler, sync::Lrc,
  FilePathMapping, Globals, Mark, SourceMap, GLOBALS,
};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_transforms_base::{
  feature::FeatureFlag, fixer::fixer, hygiene::hygiene, resolver,
};
use swc_ecma_transforms_module::common_js;
use swc_ecma_transforms_module::util::Config as ModuleConfig;
use swc_ecma_transforms_typescript::{
  typescript, Config as TypescriptConfig,
};
use swc_ecma_visit::FoldWith;

use kurtex_core::util::fs::kurtex_tmp_dir;

const EXTENSION_IDENTIFIER: &'static str = "KurtexInternals";

fn main() -> Result<(), anyhow::Error> {
  let target_snapshot_path = get_out_dir().join("KURTEX_SNAPSHOT.bin");

  let binding_dir = get_kurtex_binding_dir();
  let typescript_entry = binding_dir.join("ops/init.ts");
  let esm_entrypoint_path = kurtex_tmp_dir().join("bindgen/kurtex.mjs");

  transpile_typescript_file(&typescript_entry, &esm_entrypoint_path);

  assert!(
    esm_entrypoint_path.exists(),
    "Failed to transpile typescript entrypoint file."
  );

  let entrypoint_const =
    create_entrypoint_onebyte_const(&esm_entrypoint_path);
  let file_source_specifier = FastStaticString::new(&entrypoint_const);
  let esm_identifier: String = format!(
    "ext:{}/{}",
    EXTENSION_IDENTIFIER,
    esm_entrypoint_path.display()
  );
  let esm_identifier: &'static str = esm_identifier.leak();

  let esm_file_source = deno_core::ExtensionFileSource::new(
    &esm_identifier,
    file_source_specifier,
  );

  let runtime_extension = deno_core::Extension {
    name: EXTENSION_IDENTIFIER,
    esm_files: Cow::Owned(vec![esm_file_source]),
    esm_entry_point: Some(&esm_identifier),
    enabled: true,
    ..deno_core::Extension::default()
  };

  let js_runtime =
    deno_core::JsRuntimeForSnapshot::new(RuntimeOptions {
      extensions: vec![runtime_extension],
      ..RuntimeOptions::default()
    });
  let snapshot_output = js_runtime.snapshot();

  fs::remove_dir_all(kurtex_tmp_dir())
    .expect("Failed to remove kurtex-tmp directory.");

  Ok(fs::write(target_snapshot_path, snapshot_output)?)
}

pub fn get_out_dir() -> PathBuf {
  env::var_os("OUT_DIR").expect("OUT_DIR variable is not set").into()
}

pub fn get_kurtex_binding_dir() -> PathBuf {
  let workspace_dir: PathBuf =
    env::var("CARGO_WORKSPACE_DIR").unwrap().into();
  workspace_dir.join("kurtex_binding")
}

pub fn transpile_typescript_file<S, O>(file_path: &S, output_path: &O)
where
  S: AsRef<Path>,
  O: AsRef<Path>,
{
  let file_path = file_path.as_ref();
  let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
  let dst = Box::new(io::stderr());

  let handler = Handler::with_emitter_writer(dst, Some(cm.clone()));
  let fm = Lrc::clone(&cm)
    .load_file(&file_path)
    .expect("failed to load input typescript file");

  let comments = SingleThreadedComments::default();
  let mut parser = Parser::new(
    Syntax::Typescript(TsSyntax::default()),
    StringInput::from(&*fm),
    Some(&comments),
  );

  for e in parser.take_errors() {
    e.into_diagnostic(&handler).emit();
  }

  let program = parser
    .parse_program()
    .map_err(|e| e.into_diagnostic(&handler).emit())
    .expect("failed to parse module.");

  let globals = Globals::default();

  GLOBALS.set(&globals, || {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();

    let program = program.fold_with(&mut resolver(
      unresolved_mark,
      top_level_mark,
      true,
    ));

    let program = program.fold_with(&mut typescript(
      TypescriptConfig {
        no_empty_export: true,
        ..TypescriptConfig::default()
      },
      unresolved_mark,
      top_level_mark,
    ));

    let program = program.fold_with(&mut hygiene());
    let program = program.fold_with(&mut fixer(Some(&comments)));
    let program = program.fold_with(&mut common_js(
      unresolved_mark,
      ModuleConfig::default(),
      FeatureFlag::default(),
      Some(&comments),
    ));

    let mut buf = vec![];
    {
      let mut emitter = Emitter {
        cfg: swc_ecma_codegen::Config::default(),
        cm: cm.clone(),
        comments: Some(&comments),
        wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
      };

      emitter.emit_program(&program).unwrap();
    }

    let transpiled_result = String::from_utf8(buf).expect("non-utf8");
    let output_path = output_path.as_ref();

    fs::create_dir_all(output_path.parent().unwrap())
      .expect("Failed to ensure parent directory.");

    std::fs::write(output_path, transpiled_result)
      .expect("Failed to write transpiled result.");
  })
}

fn create_entrypoint_onebyte_const(
  esm_entrypoint_path: &PathBuf,
) -> &'static OneByteConst {
  static INIT: Once = Once::new();
  // deno_core::v8::OneByteConst is not Send + Sync
  static mut ONE_BYTE_CONST: Option<OneByteConst> = None;

  INIT.call_once(|| {
    let entrypoint_contents = fs::read(esm_entrypoint_path).unwrap();
    unsafe {
      ONE_BYTE_CONST =
        Some(FastStaticString::create_external_onebyte_const(
          Box::leak(entrypoint_contents.into_boxed_slice()),
        ));
    }
  });

  unsafe { ONE_BYTE_CONST.as_ref().unwrap() }
}
