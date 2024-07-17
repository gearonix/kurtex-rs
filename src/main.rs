#![feature(try_blocks)]

use std::env;
use std::rc::Rc;

use anyhow::Error;
use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::error::AnyError;
use deno_core::v8::Local;
use deno_core::ModuleSourceCode;
use deno_core::{anyhow, ModuleLoadResponse};
use deno_core::{extension, v8};
use deno_core::{op2, ModuleId};
use serde::{Deserialize, Serialize};

mod exports;

#[op2(async)]
#[string]
async fn op_read_file(#[string] path: String) -> Result<String, AnyError> {
    let contents = tokio::fs::read_to_string(path).await?;
    Ok(contents)
}

#[op2(async)]
#[string]
async fn op_write_file(#[string] path: String, #[string] contents: String) -> Result<(), AnyError> {
    tokio::fs::write(path, contents).await?;
    Ok(())
}

#[op2(async)]
#[string]
async fn op_fetch(#[string] url: String) -> Result<String, AnyError> {
    let body = reqwest::get(url).await?.text().await?;
    Ok(body)
}

#[op2(async)]
async fn op_set_timeout(delay: f64) -> Result<(), AnyError> {
    tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
    Ok(())
}

#[op2(fast)]
fn op_remove_file(#[string] path: String) -> Result<(), AnyError> {
    std::fs::remove_file(path)?;
    Ok(())
}

struct TsModuleLoader;

impl deno_core::ModuleLoader for TsModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Result<deno_core::ModuleSpecifier, AnyError> {
        deno_core::resolve_import(specifier, referrer).map_err(|e| e.into())
    }

    fn load(
        &self,
        module_specifier: &deno_core::ModuleSpecifier,
        _maybe_referrer: Option<&reqwest::Url>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> ModuleLoadResponse {
        println!("module_specifier: {:?}", module_specifier);
        println!("_maybe_referrer: {:?}", _maybe_referrer);
        println!("_requested_module_type: {:?}", _requested_module_type);

        let module_specifier = module_specifier.clone();

        let module_load = Box::pin(async move {
            let path = module_specifier.to_file_path().unwrap();

            let media_type = MediaType::from_path(&path);
            let (module_type, should_transpile) = match &media_type {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (deno_core::ModuleType::JavaScript, false)
                }
                MediaType::Jsx => (deno_core::ModuleType::JavaScript, true),
                MediaType::TypeScript
                | MediaType::Mts
                | MediaType::Cts
                | MediaType::Dts
                | MediaType::Dmts
                | MediaType::Dcts
                | MediaType::Tsx => (deno_core::ModuleType::JavaScript, true),
                MediaType::Json => (deno_core::ModuleType::Json, false),
                _ => panic!("Unknown extension {:?}", path.extension()),
            };

            let code = std::fs::read_to_string(&path)?;
            let code = if should_transpile {
                let parsed = deno_ast::parse_module(ParseParams {
                    specifier: module_specifier.clone(),
                    text_info: SourceTextInfo::from_string(code),
                    media_type,
                    capture_tokens: false,
                    scope_analysis: false,
                    maybe_syntax: None,
                })?;
                parsed
                    .transpile(&Default::default(), &Default::default())?
                    .into_source()
                    .text
            } else {
                code
            };
            let module = deno_core::ModuleSource::new(
                module_type,
                ModuleSourceCode::String(code.into()),
                &module_specifier,
                None,
            );
            Ok(module)
        });

        ModuleLoadResponse::Async(module_load)
    }
}

static RUNTIME_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/RUNJS_SNAPSHOT.bin"));

extension! {
    runjs,
    ops = [
        op_read_file,
        op_write_file,
        op_remove_file,
        op_fetch,
        op_set_timeout,
    ]
}

async fn resolve_module_id(
    deno_runtime: &mut deno_core::JsRuntime,
    file_path: &str,
    is_main_module: bool,
) -> Result<ModuleId, Error> {
    let module_specifier = env::current_dir()
        .map_err(Error::from)
        .and_then(|current_dir| {
            deno_core::resolve_path(file_path, current_dir.as_path()).map_err(Error::from)
        })
        .unwrap();

    if is_main_module {
        deno_runtime.load_main_es_module(&module_specifier).await
    } else {
        deno_runtime.load_side_es_module(&module_specifier).await
    }
}

async fn run_js(file_path: &str) -> Result<(), AnyError> {
    let mut deno_runtime = &mut deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(TsModuleLoader)),
        startup_snapshot: Some(RUNTIME_SNAPSHOT),
        extensions: vec![runjs::init_ops()],
        ..Default::default()
    });

    let mod_id = resolve_module_id(deno_runtime, file_path, true).await?;

    let result = deno_runtime.mod_evaluate(mod_id);
    deno_runtime.run_event_loop(Default::default()).await?;

    let sum_js_mod_id = resolve_module_id(&mut deno_runtime, "src/sum.js", false).await?;

    let global = deno_runtime.get_module_namespace(sum_js_mod_id)?;

    let scope = &mut deno_runtime.handle_scope();

    let glb_open = global.open(scope);
    let glb_local: v8::Local<'_, v8::Object> = v8::Local::new(scope, global.clone());

    let func_key = v8::String::new(scope, "sum").unwrap();
    let func = glb_open.get(scope, func_key.into()).unwrap();
    let func = v8::Local::<v8::Function>::try_from(func).unwrap();

    let a = v8::Integer::new(scope, 5).into();
    let b = v8::Integer::new(scope, 2).into();

    let func_args: &[Local<v8::Value>] = &[a, b];
    let func_res = func.call(scope, glb_local.into(), func_args).unwrap();

    let func_res = func_res
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);

    println!("Function returned: {:?}", func_res);

    result.await
}

#[derive(Serialize, Deserialize, Debug)]
struct Foo {
    hello: String,
}

async fn run_cfg_js() -> Result<(), AnyError> {
    let mut deno_runtime = &mut deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(TsModuleLoader)),
        startup_snapshot: Some(RUNTIME_SNAPSHOT),
        extensions: vec![runjs::init_ops()],
        ..Default::default()
    });

    let mod_id = resolve_module_id(deno_runtime, "src/config.ts", true).await?;

    let result = deno_runtime.mod_evaluate(mod_id);
    deno_runtime.run_event_loop(Default::default()).await?;

    let global = deno_runtime.get_module_namespace(mod_id)?;
    let scope = &mut deno_runtime.handle_scope();
    let glb_open = global.open(scope);
    let glb_local: v8::Local<'_, v8::Object> = v8::Local::new(scope, global.clone());

    let default_key = v8::String::new(scope, "default").unwrap();

    let default_obj = glb_open
        .get(scope, default_key.into())
        .ok_or_else(|| anyhow::anyhow!("no default"))?;
    let default_obj = v8::Local::<v8::Object>::try_from(default_obj).unwrap();

    let obj: Foo = deno_core::serde_v8::from_v8(scope, default_obj.into())?;

    println!("glb_open: {:?}", glb_open);
    println!("test_func: {:?}", default_obj);
    println!("obj: {:?}", obj);
    // let global = deno_runtime.get_module_namespace(config_mod_id)?;
    // let scope = &mut deno_runtime.handle_scope();
    // let glb_open = global.open(scope);

    result.await
}

fn main() {
    let args = &env::args().collect::<Vec<String>>()[1..];

    if args.is_empty() {
        eprintln!("Usage: runjs <file>");
        std::process::exit(1);
    }

    let file_path = &args[0];

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // if let Err(error) = runtime.block_on(run_js(file_path)) {
    //     eprintln!("error: {error}");
    // }

    if let Err(error) = runtime.block_on(run_cfg_js()) {
        eprintln!("error: {error}");
    }
}
