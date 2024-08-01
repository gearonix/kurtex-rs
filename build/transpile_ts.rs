use std::{io, path::Path};

use swc_common::{
  self, comments::SingleThreadedComments, errors::Handler, sync::Lrc,
  FilePathMapping, Globals, Mark, SourceMap, GLOBALS,
};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_parser::Syntax::Typescript;
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_transforms_base::{
  feature::FeatureFlag, fixer::fixer, hygiene::hygiene, resolver,
};
use swc_ecma_transforms_module::common_js;
use swc_ecma_transforms_module::util::Config as ModuleConfig;
use swc_ecma_transforms_typescript::{typescript, Config as TypescriptConfig};
use swc_ecma_visit::FoldWith;

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

    let program =
      program.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));

    let program = program.fold_with(&mut typescript(
      TypescriptConfig { no_empty_export: true, ..TypescriptConfig::default() },
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

    std::fs::write(output_path, transpiled_result).unwrap();
  })
}
