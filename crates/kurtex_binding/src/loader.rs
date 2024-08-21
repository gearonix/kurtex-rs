use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use deno_ast::{EmitOptions, MediaType, ParseParams, SourceMapOption};
use deno_core::{ModuleLoadResponse, ModuleSourceCode, ModuleType};

pub(crate) fn get_module_type_from_path<P>(
  module_path: &mut PathBuf,
  on_failure: Option<P>,
) -> (ModuleType, bool)
where
  P: FnOnce(&mut PathBuf) -> (ModuleType, bool) + 'static,
{
  let media_type = MediaType::from_path(module_path);

  match &media_type {
    MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
      (ModuleType::JavaScript, false)
    }
    MediaType::Jsx => (ModuleType::JavaScript, true),
    MediaType::TypeScript
    | MediaType::Mts
    | MediaType::Cts
    | MediaType::Dts
    | MediaType::Dmts
    | MediaType::Dcts
    | MediaType::Tsx => (ModuleType::JavaScript, true),
    MediaType::Json => (ModuleType::Json, false),
    _ => {
      let unknown_ext = format!(
        "Unknown module type: {}",
        module_path
          .extension()
          .and_then(|ext| ext.to_str())
          .unwrap_or("unknown")
      );

      on_failure.map_or_else(
        || (ModuleType::Other(Cow::Owned(unknown_ext)), false),
        |cb| cb(module_path),
      )
    }
  }
}

#[macro_export]
macro_rules! pin {
    ($($tt:tt)*) => {
        ::std::boxed::Box::pin(async move { $($tt)* })
    };
}

#[derive(Default)]
pub struct TsModuleLoader {}

impl deno_core::ModuleLoader for TsModuleLoader {
  fn resolve(
    &self,
    specifier: &str,
    referrer: &str,
    _kind: deno_core::ResolutionKind,
  ) -> Result<deno_core::ModuleSpecifier, deno_core::error::AnyError> {
    deno_core::resolve_import(specifier, referrer).map_err(|e| e.into())
  }

  fn load(
    &self,
    module_specifier: &deno_core::ModuleSpecifier,
    _maybe_referrer: Option<&reqwest::Url>,
    _is_dyn_import: bool,
    _requested_module_type: deno_core::RequestedModuleType,
  ) -> ModuleLoadResponse {
    let module_specifier = module_specifier.clone();
    let maybe_referrer = _maybe_referrer.map(|url| url.clone());

    let module_load = pin! {
      let mut module_path = module_specifier.to_file_path().unwrap();

      let get_referrer_extension =
        |module_path: &mut PathBuf| -> (ModuleType, bool) {
          match maybe_referrer.map(|r| r.to_file_path().unwrap()) {
            Some(referrer_url) => {
              let mut referrer_path = referrer_url;

            let module_ext = module_path.extension().unwrap();
            let referrer_ext = referrer_path.extension().unwrap();
            let mut new_ext = module_ext.to_os_string();

            new_ext.push(".");
            new_ext.push(referrer_ext);

            module_path.set_extension(new_ext);

            get_module_type_from_path(
              &mut referrer_path,
              None::<fn(&mut PathBuf) -> (ModuleType, bool)>,
            )
            }
            None => {
              panic!("Unknown extension. File {module_path:?}");
            }
          }
        };

      let (module_type, should_transpile) = get_module_type_from_path(
        &mut module_path,
        Some(get_referrer_extension),
      );

      let media_type = MediaType::from_path(&module_path);
      let code =
        std::fs::read_to_string(&module_path.as_path()).with_context(|| {
          format!("Trying to load {module_path:?} for {module_specifier}")
        })?;

      let code = if should_transpile {
        let parsed = deno_ast::parse_module(ParseParams {
          specifier: module_specifier.clone(),
          text: Arc::from(code),
          media_type,
          capture_tokens: false,
          scope_analysis: false,
          maybe_syntax: None,
        })?;

        let source_bytes = parsed
          .transpile(
            &Default::default(),
            &EmitOptions {
              source_map: SourceMapOption::Inline,
              ..EmitOptions::default()
            },
          )?.into_source();

        String::from_utf8(source_bytes.source)?
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
    };

    ModuleLoadResponse::Async(module_load)
  }
}
