use std::fmt::Formatter;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use deno_core::v8;
use hashbrown::HashMap;

use crate::error::AnyError;

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum CollectorMode {
  #[default]
  Run,
  Skip,
  Only,
  Todo,
}

impl<'a> deno_core::FromV8<'a> for CollectorMode {
  type Error = deno_core::error::StdAnyError;

  fn from_v8(
    scope: &mut v8::HandleScope<'a>,
    value: v8::Local<'a, v8::Value>,
  ) -> Result<Self, Self::Error> {
    let owned_string = deno_core::_ops::to_string(scope, &value);

    CollectorMode::from_str(&owned_string)
      .map_err(|e| deno_core::error::StdAnyError::from(e))
  }
}

impl FromStr for CollectorMode {
  type Err = AnyError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "run" => Ok(CollectorMode::Run),
      "skip" => Ok(CollectorMode::Skip),
      "only" => Ok(CollectorMode::Only),
      "todo" => Ok(CollectorMode::Todo),
      _ => Err(anyhow!("Invalid CollectorRunMode variant: '{}'", s)),
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum CollectorStatus {
  Custom(CollectorMode),
  Fail,
  #[default]
  Pass,
}

#[derive(Default, Clone)]
pub enum CollectorIdentifier {
  #[default]
  File,
  Custom(String),
}

impl<'a> deno_core::FromV8<'a> for CollectorIdentifier {
  type Error = deno_core::error::StdAnyError;

  fn from_v8(
    scope: &mut v8::HandleScope<'a>,
    value: v8::Local<'a, v8::Value>,
  ) -> Result<Self, Self::Error> {
    let identifier = CollectorIdentifier::Custom(
      deno_core::_ops::to_string(scope, &value),
    );

    Ok(identifier)
  }
}

impl std::fmt::Debug for CollectorIdentifier {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    const FILE_IDENT: &'static str = "$$file";

    let identifier = match self {
      CollectorIdentifier::Custom(e) => e,
      CollectorIdentifier::File => FILE_IDENT,
    };

    write!(f, "{identifier:?}")
  }
}

#[derive(Default)]
pub struct CollectorFile {
  pub(crate) file_path: PathBuf,
  pub(crate) collected: bool,
  pub(crate) error: Option<AnyError>,
  pub(crate) nodes: Vec<Arc<Mutex<CollectorNode>>>,
}

impl CollectorFile {
  pub fn from_path(file_path: PathBuf) -> Self {
    CollectorFile { file_path, ..CollectorFile::default() }
  }
}

// temporary
impl std::fmt::Debug for CollectorFile {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorFile")
      .field("file", &self.file_path)
      .field("collected", &self.collected)
      .field("nodes", &self.nodes)
      .finish()
  }
}

#[derive(Default)]
pub struct CollectorNode {
  pub(crate) identifier: CollectorIdentifier,
  pub(crate) mode: CollectorMode,
  pub(crate) tasks: Vec<Arc<Mutex<CollectorTask>>>,
  pub(crate) status: CollectorStatus,
  pub(crate) error: Option<AnyError>,
  pub(crate) hook_manager: LifetimeHookManager,
}

impl std::fmt::Debug for CollectorNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorNode")
      .field("name", &self.identifier)
      .field("mode", &self.mode)
      .field("tasks", &self.tasks)
      .finish()
  }
}

// Kurtex generic callback.
#[derive(Debug, Clone)]
pub struct TestCallback(v8::Global<v8::Function>);

unsafe impl Send for TestCallback {}
unsafe impl Sync for TestCallback {}

static_assertions::assert_impl_any!(TestCallback: Send);
static_assertions::assert_impl_any!(TestCallback: Sync);

impl From<v8::Global<v8::Function>> for TestCallback {
  fn from(value: v8::Global<v8::Function>) -> Self {
    TestCallback(value)
  }
}

impl Deref for TestCallback {
  type Target = v8::Global<v8::Function>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<'a> deno_core::FromV8<'a> for TestCallback {
  type Error = deno_core::error::StdAnyError;

  fn from_v8(
    scope: &mut v8::HandleScope<'a>,
    value: v8::Local<'a, v8::Value>,
  ) -> Result<Self, Self::Error> {
    let Ok(local_cb) =
      deno_core::_ops::v8_try_convert::<v8::Function>(value)
    else {
      panic!("Failed to convert parameter (deno_core::ops::v8_try_convert). Expected function.");
    };
    let global_cb = v8::Global::new(scope, local_cb);

    Ok(TestCallback(global_cb))
  }
}

pub struct CollectorTask {
  pub(crate) name: String,
  pub(crate) mode: CollectorMode,
  pub(crate) error: Option<AnyError>,
  pub(crate) status: CollectorStatus,
  pub(crate) callback: TestCallback,
}

impl std::fmt::Debug for CollectorTask {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorTask")
      .field("name", &self.name)
      .field("mode", &self.mode)
      .finish()
  }
}

impl CollectorTask {
  pub fn new(
    name: String,
    callback: TestCallback,
    mode: CollectorMode,
  ) -> Self {
    CollectorTask {
      name,
      mode,
      error: None,
      status: CollectorStatus::Custom(mode),
      callback,
    }
  }
}

#[derive(Clone)]
pub struct LifetimeHookManager {
  data: HashMap<LifetimeHook, Vec<TestCallback>>,
}

impl LifetimeHookManager {
  pub fn new() -> Self {
    let mut hooks: HashMap<_, _> = HashMap::new();

    hooks.insert(LifetimeHook::BeforeAll, Vec::new());
    hooks.insert(LifetimeHook::AfterAll, Vec::new());
    hooks.insert(LifetimeHook::BeforeEach, Vec::new());
    hooks.insert(LifetimeHook::AfterEach, Vec::new());

    LifetimeHookManager { data: hooks }
  }

  pub fn add_hook(
    &mut self,
    hook_key: LifetimeHook,
    callback: TestCallback,
  ) {
    self
      .data
      .get_mut(&hook_key)
      .and_then(|partition| {
        partition.push(callback);
        Some(partition)
      })
      .unwrap_or_else(|| panic!("Wrong lifetime hook partition."));
  }

  pub fn get_by(&self, hook_key: LifetimeHook) -> &Vec<TestCallback> {
    self
      .data
      .get(&hook_key)
      .unwrap_or_else(|| panic!("Wrong lifetime hook partition."))
  }
}

impl Default for LifetimeHookManager {
  fn default() -> Self {
    LifetimeHookManager::new()
  }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub enum LifetimeHook {
  BeforeAll,
  AfterAll,
  BeforeEach,
  AfterEach,
}

impl<'a> deno_core::FromV8<'a> for LifetimeHook {
  type Error = deno_core::error::StdAnyError;

  fn from_v8(
    scope: &mut v8::HandleScope<'a>,
    value: v8::Local<'a, v8::Value>,
  ) -> Result<Self, Self::Error> {
    let owned_string = deno_core::_ops::to_string(scope, &value);

    LifetimeHook::from_str(&owned_string)
      .map_err(|e| deno_core::error::StdAnyError::from(e))
  }
}

impl FromStr for LifetimeHook {
  type Err = AnyError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let lifetime_hook = match s {
      "beforeAll" => LifetimeHook::BeforeAll,
      "afterAll" => LifetimeHook::AfterAll,
      "beforeEach" => LifetimeHook::BeforeEach,
      "afterEach" => LifetimeHook::AfterEach,
      _ => unreachable!(),
    };

    Ok(lifetime_hook)
  }
}
