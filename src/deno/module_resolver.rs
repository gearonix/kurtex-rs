use std::cell::RefCell;
use std::convert::From;
use std::env;
use std::ops::Deref;
use std::rc::Rc;

use deno_core::anyhow::Context;
use deno_core::error::AnyError;
use deno_core::v8::{DataError, HandleScope, Local, Value};
use deno_core::{v8, ModuleId};
use serde::{Deserialize, Serialize};

use crate::deno::module_loader::TsModuleLoader;
use crate::runner::ops::{BindingsResolver, OpsLoader};

pub struct EsmModuleResolver {
  pub runtime: deno_core::JsRuntime,
}

#[derive(Default)]
pub struct EsmResolverOptions {
  pub loaders: Vec<Box<dyn OpsLoader>>,
}

// TODO rewrite to better scoped lts

impl EsmModuleResolver {
  pub fn new(runtime_opts: EsmResolverOptions) -> EsmModuleResolver {
    let binary_snapshot = BindingsResolver::get_library_snapshot_path();
    let EsmResolverOptions { loaders: ops_loaders } = runtime_opts;
    let include_snapshot = !ops_loaders.is_empty();

    let startup_snapshot = include_snapshot.then(|| binary_snapshot);
    let extensions =
      ops_loaders.into_iter().map(|loader| loader.load()).collect();

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
    is_main: bool,
  ) -> Result<ModuleId, AnyError>
  where
    S: AsRef<str>,
  {
    let file_path = file_path.as_ref();

    let module_id = self.resolve_module_id(file_path, is_main).await?;

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

  pub fn get_op_state(
    &mut self,
  ) -> Result<Rc<RefCell<deno_core::OpState>>, AnyError> {
    Ok(self.runtime.op_state())
  }

  pub fn extract_op_state<'a, R>(
    &mut self,
    op_state: &'a deno_core::OpState,
  ) -> Result<&'a R, AnyError>
  where
    R: 'static,
  {
    op_state.deref().try_borrow::<R>().context("error while accessing op_state")
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
