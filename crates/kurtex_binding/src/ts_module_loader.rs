use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use deno_ast::{
  EmitOptions, ImportsNotUsedAsValues, MediaType, ParseParams, SourceMapOption,
  TranspileOptions,
};
use deno_core::{
  ModuleLoadResponse, ModuleSource, ModuleSourceCode, ModuleSpecifier,
  ModuleType, RequestedModuleType,
};
use deno_graph::source::{
  MemoryLoader as GraphMemoryLoader, MemoryLoader, Source as GraphSource,
};
use hashbrown::HashMap;
use rccell::RcCell;

type SourceMapStore = RcCell<HashMap<String, Vec<u8>>>;

pub struct TypescriptModuleLoader {
  graph_loader: RcCell<GraphMemoryLoader>,
  source_maps: SourceMapStore,
}

impl TypescriptModuleLoader {
  pub fn new() -> Self {
    let graph_loader = RcCell::new(GraphMemoryLoader::default());
    let source_maps = RcCell::new(HashMap::default());

    TypescriptModuleLoader { graph_loader, source_maps }
  }

  pub fn graph_loader(&self) -> &RcCell<GraphMemoryLoader> {
    &self.graph_loader
  }
}

impl deno_core::ModuleLoader for TypescriptModuleLoader {
  fn resolve(
    &self,
    specifier: &str,
    referrer: &str,
    _kind: deno_core::ResolutionKind,
  ) -> Result<ModuleSpecifier, deno_core::error::AnyError> {
    deno_core::resolve_import(specifier, referrer).map_err(|e| e.into())
  }

  fn load(
    &self,
    module_specifier: &ModuleSpecifier,
    _maybe_referrer: Option<&ModuleSpecifier>,
    _is_dyn_import: bool,
    _requested_module_type: deno_core::RequestedModuleType,
  ) -> ModuleLoadResponse {
    let mut graph_loader = self.graph_loader.borrow_mut();
    let source_maps = self.source_maps.clone();
    let module_specifier = module_specifier.clone();

    fn load_module(
      graph_loader: &mut MemoryLoader,
      source_maps: SourceMapStore,
      module_specifier: &ModuleSpecifier,
      _requested_module_type: RequestedModuleType,
    ) -> Result<ModuleSource, deno_core::error::AnyError> {
      let module_path = module_specifier.to_file_path().unwrap();
      let module_source = module_specifier.to_string();
      let (module_type, should_transpile) =
        get_module_type_from_path(&module_path);

      let media_type = MediaType::from_path(&module_path);
      let source_code = std::fs::read_to_string(&module_path.as_path())
        .with_context(|| {
          format!("Trying to load {module_path:?} for {module_specifier}")
        })?;

      let source_code_ = source_code.clone();
      let mut module_headers = Vec::new();
      let content_type_header =
        get_content_type_header(&module_type, should_transpile);

      if let Some(header) = content_type_header {
        module_headers.push(header)
      }

      graph_loader.add_source(
        module_source.clone(),
        GraphSource::Module {
          specifier: module_source,
          maybe_headers: Some(module_headers),
          content: source_code_,
        },
      );

      let source_code = if should_transpile {
        let parsed = deno_ast::parse_module(ParseParams {
          specifier: module_specifier.clone(),
          text: Arc::from(source_code),
          media_type,
          capture_tokens: false,
          scope_analysis: false,
          maybe_syntax: None,
        })?;

        let source_data = parsed
          .transpile(
            &TranspileOptions {
              // preserve imports to build correct module graph.
              imports_not_used_as_values: ImportsNotUsedAsValues::Preserve,
              ..TranspileOptions::default()
            },
            &EmitOptions {
              source_map: SourceMapOption::Separate,
              remove_comments: true,
              ..EmitOptions::default()
            },
          )?
          .into_source();
        let cm = source_data.source_map.unwrap();
        source_maps.borrow_mut().insert(module_specifier.to_string(), cm);

        String::from_utf8(source_data.source)?
      } else {
        source_code
      };
      let module = deno_core::ModuleSource::new(
        module_type,
        ModuleSourceCode::String(source_code.into()),
        &module_specifier,
        None,
      );

      Ok(module)
    };

    ModuleLoadResponse::Sync(load_module(
      &mut graph_loader,
      source_maps,
      &module_specifier,
      _requested_module_type,
    ))
  }

  fn get_source_map(&self, specifier: &str) -> Option<Vec<u8>> {
    self.source_maps.borrow().get(specifier).cloned()
  }
}

pub fn get_module_type_from_path(module_path: &PathBuf) -> (ModuleType, bool) {
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
    mt @ _ => (ModuleType::Other(Cow::Owned(mt.to_string())), false),
  }
}

pub fn get_content_type_header(
  module_type: &ModuleType,
  should_transpile: bool,
) -> Option<(String, String)> {
  let header_str = match module_type {
    ModuleType::JavaScript => {
      if should_transpile {
        "application/javascript"
      } else {
        "application/typescript"
      }
    }
    ModuleType::Json => "application/json",
    _ => return None,
  };

  Some(("content-type".to_string(), header_str.to_string()))
}
