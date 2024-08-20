use std::cell::RefCell;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::str::FromStr;

use anyhow::anyhow;
use deno_core::convert::Smi;
use deno_core::v8;
use deno_core::v8::{HandleScope, Local, Value};
use hashbrown::HashMap;

use crate::error::AnyError;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum CollectorMode {
  #[default]
  Run,
  Skip,
  Only,
  Todo,
}

#[derive(Clone, Copy)]
pub enum CollectorState {
  Custom(CollectorMode),
  Fail,
  Pass,
}

impl<'a> deno_core::FromV8<'a> for CollectorMode {
  type Error = deno_core::error::StdAnyError;

  fn from_v8(
    scope: &mut HandleScope<'a>,
    value: Local<'a, Value>,
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

#[derive(Default, Clone)]
pub enum CollectorIdentifier {
  #[default]
  File,
  Custom(String),
}

impl<'a> deno_core::FromV8<'a> for CollectorIdentifier {
  type Error = deno_core::error::StdAnyError;

  fn from_v8(
    scope: &mut HandleScope<'a>,
    value: Local<'a, Value>,
  ) -> Result<Self, Self::Error> {
    let identifier =
      CollectorIdentifier::Custom(deno_core::_ops::to_string(scope, &value));

    Ok(identifier)
  }
}

impl std::fmt::Debug for CollectorIdentifier {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    const FILE_IDENT: &'static str = "$$file";

    let identifier = match self {
      CollectorIdentifier::Custom(e) => &e,
      CollectorIdentifier::File => FILE_IDENT,
    };

    write!(f, "{identifier:?}")
  }
}

#[derive(Default)]
pub struct CollectorFile {
  pub file_path: PathBuf,
  pub collected: RefCell<bool>,
  pub nodes: RefCell<Vec<Rc<CollectorNode>>>,
}

// temporary
impl std::fmt::Debug for CollectorFile {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorFile")
      .field("file", &self.file_path)
      .field("collected", &self.collected.borrow())
      .field("nodes", &self.nodes.borrow().iter().map(|n| n))
      .finish()
  }
}

#[derive(Default)]
pub struct CollectorNode {
  pub(crate) identifier: CollectorIdentifier,
  pub(crate) mode: RefCell<CollectorMode>,
  pub(crate) tasks: RefCell<Vec<Rc<CollectorTask>>>,
  pub(crate) file: RefCell<Weak<CollectorFile>>,
  pub(crate) status: Option<CollectorState>,
  pub(crate) hook_manager: RefCell<LifetimeHookManager>,
}

impl std::fmt::Debug for CollectorNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CollectorNode")
      .field("name", &self.identifier)
      .field("mode", &self.mode.borrow())
      .field("tasks", &self.tasks.borrow().iter().map(|n| n))
      .finish()
  }
}

pub type TestCallback = v8::Global<v8::Function>;

// TODO think about making whole struct
// RefCell instead of fields
pub struct CollectorTask {
  pub(crate) name: String,
  pub(crate) mode: RefCell<CollectorMode>,
  pub(crate) state: RefCell<CollectorState>,
  pub(crate) node: RefCell<Weak<CollectorNode>>,
  pub(crate) file: RefCell<Weak<CollectorFile>>,
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
      mode: RefCell::new(mode),
      file: RefCell::new(Weak::new()),
      node: RefCell::new(Weak::new()),
      state: RefCell::new(CollectorState::Custom(mode)),
      callback,
    }
  }
}

pub struct LifetimeHookManager {
  data: HashMap<LifetimeHook, Vec<TestCallback>>,
}

impl LifetimeHookManager {
  pub fn new() -> Self {
    let mut hooks: HashMap<LifetimeHook, Vec<TestCallback>> = HashMap::new();

    hooks.insert(LifetimeHook::BeforeAll, Vec::new());
    hooks.insert(LifetimeHook::AfterAll, Vec::new());
    hooks.insert(LifetimeHook::BeforeEach, Vec::new());
    hooks.insert(LifetimeHook::AfterEach, Vec::new());

    LifetimeHookManager { data: hooks }
  }

  pub fn add_hook(&mut self, hook_key: LifetimeHook, callback: TestCallback) {
    self
      .data
      .get_mut(&hook_key)
      .and_then(|partition| {
        partition.push(callback);
        Some(partition)
      })
      .unwrap_or_else(|| panic!("Wrong lifetime hook partition."));
  }
}

impl Default for LifetimeHookManager {
  fn default() -> Self {
    LifetimeHookManager::new()
  }
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum LifetimeHook {
  BeforeAll,
  AfterAll,
  BeforeEach,
  AfterEach,
}

impl<'a> deno_core::FromV8<'a> for LifetimeHook {
  type Error = deno_core::error::StdAnyError;

  fn from_v8(
    scope: &mut HandleScope<'a>,
    value: Local<'a, Value>,
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
