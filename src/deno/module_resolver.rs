use std::convert::From;
use std::env;
use std::rc::Rc;

use deno_core::error::AnyError;
use deno_core::v8::{DataError, Local, Value};
use deno_core::{v8, ModuleId};
use serde::{Deserialize, Serialize};

use crate::deno::module_loader::TsModuleLoader;

pub struct EsmModuleResolver {
  pub runtime: deno_core::JsRuntime,
}

impl EsmModuleResolver {
  pub fn new() -> EsmModuleResolver {
    let deno_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
      module_loader: Some(Rc::new(TsModuleLoader)),
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
    &mut self,
    module_id: ModuleId,
    exports_specifier: Option<S>,
  ) -> Result<R, DataError>
  where
    S: AsRef<str>,
    R: TryFrom<Local<'a, Value>, Error = DataError>,
    <R as TryFrom<Local<'a, Value>>>::Error: Send + Sync,
  {
    let global = self.runtime.get_module_namespace(module_id)?;
    let scope = &mut self.runtime.handle_scope();
    let file_object_mapper = global.open(scope);

    let specifier = exports_specifier.map(|s| s.as_ref()).unwrap_or("default");
    let default_export = v8::String::new(scope, specifier).unwrap();
    let exported_config =
      file_object_mapper.get(scope, default_export.into()).unwrap();

    R::try_from(exported_config)
  }

  pub async fn serialize_v8_object<R>(
    &mut self,
    v8_object: Local<'_, v8::Object>,
  ) -> Result<R, deno_core::serde_v8::Error>
  where
    R: Serialize + for<'de> Deserialize<'de>,
  {
    let mut scope = self.runtime.handle_scope();

    Ok(deno_core::serde_v8::from_v8(&mut scope, v8_object.into())?)
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
