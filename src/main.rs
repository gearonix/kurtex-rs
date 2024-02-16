fn main() {
    let (js, sourcemap) = ts_to_js(
        "hello.ts",
        r#"
    interface Args {
        name: string;
     }

    function hello(param: Args) {
        // comment
        console.log(`Hello ${param.name}!`);
    }
    "#,
    );

    println!("{}", js);
    println!("{}", sourcemap);
}

use std::io;

use swc::{config::IsModule, Compiler, PrintArgs};
use swc_common::{errors::Handler, source_map::SourceMap, sync::Lrc, Mark, GLOBALS};
use swc_ecma_ast::EsVersion;
use swc_ecma_parser::Syntax;
use swc_ecma_transforms_typescript::strip;
use swc_ecma_visit::FoldWith;

/// Transforms typescript to javascript. Returns tuple (js string, source map)
pub(crate) fn ts_to_js(filename: &str, ts_code: &str) -> (String, String) {
    let cm = Lrc::new(SourceMap::new(swc_common::FilePathMapping::empty()));

    let compiler = Compiler::new(cm.clone());

    let source = cm.new_source_file(
        swc_common::FileName::Custom(filename.into()),
        ts_code.to_string(),
    );

    let handler = Handler::with_emitter_writer(Box::new(io::stderr()), Some(compiler.cm.clone()));

    return GLOBALS.set(&Default::default(), || {
        let program = compiler
            .parse_js(
                source,
                &handler,
                EsVersion::Es5,
                Syntax::Typescript(Default::default()),
                IsModule::Bool(false),
                Some(compiler.comments()),
            )
            .expect("parse_js failed");

        // Add TypeScript type stripping transform
        let top_level_mark = Mark::new();
        let program = program.fold_with(&mut strip(top_level_mark));

        // https://rustdoc.swc.rs/swc/struct.Compiler.html#method.print
        let ret = compiler
            .print(
                &program, // ast to print
                PrintArgs::default(),
            )
            .expect("print failed");

        return (ret.code, ret.map.expect("no source map"));
    });
}
