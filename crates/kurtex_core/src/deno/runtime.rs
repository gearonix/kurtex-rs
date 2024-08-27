use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use anyhow::{anyhow, bail};
use deno_ast::ModuleSpecifier;
use deno_core::anyhow::Context;
use deno_core::error::AnyError;
use deno_core::v8::{DataError, HandleScope, Local, Value};
use deno_core::{v8, CrossIsolateStore, ModuleId, PollEventLoopOptions};
use deno_graph::{GraphKind, ModuleGraph, WalkOptions};
use rccell::RcCell;
use serde::{Deserialize, Serialize};

use kurtex_binding::ts_module_loader::TypescriptModuleLoader;

use crate::deno::ExtensionLoader;
use crate::AnyResult;

pub struct KurtexRuntime {
  runtime: deno_core::JsRuntime,
  pub(crate) graph: KurtexGraph,
}

#[derive(Default)]
pub struct KurtexRuntimeOptions {
  pub loaders: Vec<Box<dyn ExtensionLoader>>,
  pub snapshot: &'static [u8],
  pub is_main: bool,
}

impl KurtexRuntime {
  pub fn new(options: KurtexRuntimeOptions) -> KurtexRuntime {
    let KurtexRuntimeOptions { loaders, snapshot, is_main } = options;
    let include_snapshot = !loaders.is_empty();

    let startup_snapshot = include_snapshot.then(|| snapshot);
    let extensions =
      loaders.into_iter().map(|loader| loader.load()).collect();
    let module_loader = Rc::new(TypescriptModuleLoader::new());

    let deno_runtime =
      deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        startup_snapshot,
        module_loader: Some(module_loader.clone()),
        extensions,
        is_main,
        extension_transpiler: None,
        shared_array_buffer_store: Some(CrossIsolateStore::default()),
        ..Default::default()
      });
    let graph = KurtexGraph::new(module_loader.clone());

    Self { runtime: deno_runtime, graph }
  }

  pub async fn resolve_module<S>(
    &mut self,
    file_path: S,
  ) -> AnyResult<ModuleId>
  where
    S: AsRef<str>,
  {
    self.resolve_module_inner(file_path, false).await
  }

  pub async fn resolve_main_module<S>(
    &mut self,
    file_path: S,
  ) -> AnyResult<ModuleId>
  where
    S: AsRef<str>,
  {
    self.resolve_module_inner(file_path, true).await
  }

  pub async fn resolve_module_inner<S>(
    &mut self,
    file_path: S,
    // TODO: rewrite
    is_main: bool,
  ) -> AnyResult<ModuleId>
  where
    S: AsRef<str>,
  {
    let file_path = file_path.as_ref();

    let module_id = self.load_es_module(file_path, is_main).await?;
    self.runtime.mod_evaluate(module_id).await?;
    self.runtime.run_event_loop(Default::default()).await?;

    Ok(module_id)
  }

  pub async fn resolve_test_module<S>(
    &mut self,
    file_path: S,
  ) -> AnyResult<ModuleId>
  where
    S: AsRef<str>,
  {
    let file_path_ = file_path.as_ref();
    let module_specifier = ModuleSpecifier::from_file_path(&file_path_);

    match module_specifier {
      Ok(specifier) => self.graph.add_root(specifier),
      Err(_) => {
        bail!("Invalid module path: {}", file_path_)
      }
    }

    Ok(self.resolve_module(file_path).await?)
  }

  pub async fn get_module_exports<'a, R, S>(
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

    let specifier = exports_specifier
      .as_ref()
      .map(|s| s.as_ref())
      .unwrap_or("default");
    let default_export = v8::String::new(scope_ref, specifier).unwrap();
    let exported_config =
      file_object_mapper.get(scope_ref, default_export.into()).unwrap();

    Ok((R::try_from(exported_config)?, scope))
  }

  pub fn op_state(
    &mut self,
  ) -> Result<Rc<RefCell<deno_core::OpState>>, AnyError> {
    Ok(self.runtime.op_state())
  }

  pub fn mutate_state<T, U, R>(&mut self, getter: R) -> AnyResult<U>
  where
    T: 'static,
    R: FnOnce(&mut T) -> U,
  {
    let op_state = self.runtime.op_state();
    let mut op_state = op_state.borrow_mut();

    let generic_state = op_state
      .deref_mut()
      .try_borrow_mut::<T>()
      .context("Error while accessing op_state mutably.")
      .unwrap();

    Ok(getter(generic_state))
  }

  pub fn mutate_state_with<T, U, R, I>(
    &mut self,
    init: I,
    getter: R,
  ) -> AnyResult<U>
  where
    T: 'static,
    R: FnOnce(I, &mut T) -> U,
  {
    Ok(self.mutate_state(|state| getter(init, state))?)
  }

  pub fn get_state_with<T, U, R, I>(
    &mut self,
    init: I,
    getter: R,
  ) -> AnyResult<U>
  where
    T: 'static,
    R: FnOnce(I, &T) -> U,
  {
    Ok(self.get_state(|state| getter(init, state))?)
  }

  pub fn get_state<T, U, R>(&mut self, getter: R) -> AnyResult<U>
  where
    T: 'static,
    R: FnOnce(&T) -> U,
  {
    let op_state = self.runtime.op_state();
    let op_state = op_state.borrow();

    let generic_state = op_state
      .deref()
      .try_borrow::<T>()
      .context("Error while accessing op_state.")
      .unwrap();

    Ok(getter(generic_state))
  }

  async fn load_es_module(
    &mut self,
    file_path: &str,
    is_main: bool,
  ) -> AnyResult<ModuleId> {
    let module_specifier =
      ModuleSpecifier::from_file_path(&file_path)
        .map_err(|_e| anyhow!("Invalid module path: {}", file_path))?;

    if is_main {
      self.runtime.load_main_es_module(&module_specifier).await
    } else {
      self.runtime.load_side_es_module(&module_specifier).await
    }
  }

  pub async fn call_v8_function<'a>(
    &mut self,
    callback: &'a v8::Global<v8::Function>,
  ) -> AnyResult<v8::Global<v8::Value>> {
    let call = self.runtime.call_with_args(callback, &[]);
    self
      .runtime
      .with_event_loop_promise(call, PollEventLoopOptions::default())
      .await
  }

  pub async fn serialize_v8_object<R>(
    mut scope: HandleScope<'_>,
    v8_object: Local<'_, v8::Object>,
  ) -> Result<R, deno_core::serde_v8::Error>
  where
    R: Serialize + for<'de> Deserialize<'de>,
  {
    Ok(deno_core::serde_v8::from_v8(&mut scope, v8_object.into())?)
  }
}

pub struct KurtexGraph {
  roots: Vec<deno_core::ModuleSpecifier>,
  module_loader: Rc<TypescriptModuleLoader>,
  built: RcCell<bool>,
}

impl KurtexGraph {
  pub fn new(module_loader: Rc<TypescriptModuleLoader>) -> Self {
    KurtexGraph {
      roots: vec![],
      module_loader,
      built: Default::default(),
    }
  }

  fn add_root(&mut self, specifier: ModuleSpecifier) {
    self.roots.push(specifier)
  }

  pub async fn build(&self) -> AnyResult<Rc<ModuleGraph>> {
    let mut built = self.built.borrow_mut();

    if *built {
      return Err(anyhow!("The module graph has already been built."));
    }

    let mut roots = self.roots.clone();
    let loader = self.module_loader.graph_loader().borrow();
    let mut graph = ModuleGraph::new(GraphKind::All);

    graph.build(roots.clone(), loader.deref(), Default::default()).await;

    graph
      .walk(
        roots.iter(),
        WalkOptions {
          check_js: true,
          follow_type_only: false,
          follow_dynamic: false,
          prefer_fast_check_graph: false,
        },
      )
      .validate()
      .unwrap();

    *built = true;
    Ok(Rc::new(graph))
  }
}
