use std::convert::From;
use std::env;
use std::rc::Rc;

use deno_core::error::AnyError;
use deno_core::v8::{DataError, HandleScope, Local, Value};
use deno_core::{v8, ModuleId};
use serde::{Deserialize, Serialize};

use crate::deno::module_loader::TsModuleLoader;
use crate::deno::ops::ResolverOps;

pub struct EsmModuleResolver {
  pub runtime: deno_core::JsRuntime,
}

#[derive(Default)]
pub struct EsmResolverOptions {
  pub include_bindings: bool,
}

impl EsmModuleResolver {
  pub fn new(runtime_opts: EsmResolverOptions) -> EsmModuleResolver {
    let EsmResolverOptions { include_bindings } = runtime_opts;

    let binary_snapshot = ResolverOps::get_snapshot_binary();

    let startup_snapshot = include_bindings.then(|| binary_snapshot);
    let runtime_extensions =
      include_bindings.then(|| ResolverOps::initialize_extensions());
    let extensions = runtime_extensions.unwrap_or_else(|| vec![]);

    let deno_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
      module_loader: Some(Rc::new(TsModuleLoader)),
      startup_snapshot,
      extensions,
      ..Default::default()
    });

    Self { runtime: deno_runtime }
  }

  pub async fn process_esm_file<S>(
    &mut self,
    file_path: S,
  ) -> Result<ModuleId, AnyError>
  where
    S: AsRef<str>,
  {
    let file_path = file_path.as_ref();

    let module_id = self.resolve_module_id(file_path, true).await?;

    self.runtime.mod_evaluate(module_id).await?;
    self.runtime.run_event_loop(Default::default()).await?;

    Ok(module_id)
  }

  pub async fn extract_file_exports<'a, R, S>(
    &'a mut self,
    module_id: ModuleId,
    exports_specifier: Option<S>,
  ) -> Result<(R, HandleScope<'_>), AnyError>
  where
    S: AsRef<str>,
    R: TryFrom<Local<'a, Value>, Error = DataError>,
  {
    let global = self.runtime.get_module_namespace(module_id)?;
    let mut scope = self.runtime.handle_scope();
    let scope_ref = &mut scope;
    let file_object_mapper = global.open(scope_ref);

    let specifier =
      exports_specifier.as_ref().map(|s| s.as_ref()).unwrap_or("default");
    let default_export = v8::String::new(scope_ref, specifier).unwrap();
    let exported_config =
      file_object_mapper.get(scope_ref, default_export.into()).unwrap();

    Ok((R::try_from(exported_config)?, scope))
  }

  async fn resolve_module_id(
    &mut self,
    file_path: &str,
    is_main_module: bool,
  ) -> Result<ModuleId, AnyError> {
    // NOTE: remove current_dir
    let module_specifier = env::current_dir()
      .map_err(AnyError::from)
      .and_then(|current_dir| {
        deno_core::resolve_path(file_path, current_dir.as_path())
          .map_err(AnyError::from)
      })
      .unwrap();

    if is_main_module {
      self.runtime.load_main_es_module(&module_specifier).await
    } else {
      self.runtime.load_side_es_module(&module_specifier).await
    }
  }
}

#[derive(Default)]
pub struct EsmSerdeResolver;

impl EsmSerdeResolver {
  pub async fn serialize<R>(
    mut scope: HandleScope<'_>,
    v8_object: Local<'_, v8::Object>,
  ) -> Result<R, deno_core::serde_v8::Error>
  where
    R: Serialize + for<'de> Deserialize<'de>,
  {
    Ok(deno_core::serde_v8::from_v8(&mut scope, v8_object.into())?)
  }
}
