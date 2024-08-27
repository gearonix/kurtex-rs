use rayon::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;

use crate::AnyResult;
use deno_graph::ModuleSpecifier;
use deno_graph::{GraphKind, ModuleGraph};

pub struct WatcherResolver {
  // TODO: change Specifier to something lighter + BTreeSet
  cached_files: Vec<ModuleSpecifier>,
  changed_files: Vec<ModuleSpecifier>,
  module_graph: Rc<ModuleGraph>,
}

impl WatcherResolver {
  pub fn new(graph: Rc<ModuleGraph>) -> Self {
    assert_eq!(
      graph.graph_kind(),
      GraphKind::All,
      "Watcher can only operate on GraphKind::All."
    );

    WatcherResolver {
      changed_files: vec![],
      cached_files: vec![],
      module_graph: graph,
    }
  }

  pub fn resolve_dependency_tests(
    &mut self,
    file_path: PathBuf,
  ) -> Vec<PathBuf> {
    self.changed_files.clear();
    self.cached_files.clear();

    let specifier = ModuleSpecifier::from_file_path(file_path).unwrap();
    Self::resolve_dependencies(
      &mut self.cached_files,
      &mut self.changed_files,
      &self.module_graph,
      specifier,
    );

    self.changed_files.iter().cloned().map(|url| url.path().into()).collect()
  }

  fn resolve_dependencies(
    cached_files: &mut Vec<ModuleSpecifier>,
    changed_files: &mut Vec<ModuleSpecifier>,
    module_graph: &ModuleGraph,
    specifier: ModuleSpecifier,
  ) {
    let was_cached = cached_files.contains(&specifier);
    let was_changed = changed_files.contains(&specifier);
    let not_in_graph = !module_graph.contains(&specifier);
    let is_graph_root = module_graph.roots.contains(&specifier);

    if was_cached || was_changed || not_in_graph {
      return;
    }

    cached_files.push(specifier.clone());
    if is_graph_root {
      return changed_files.push(specifier.clone());
    }

    let modules = module_graph.modules().filter_map(|module| module.js());
    modules.for_each(|module| {
      for dependency in module.dependencies.values() {
        let is_importer =
          dependency.get_code().filter(|&dep| dep.eq(&specifier)).is_some();

        is_importer.then(|| {
          Self::resolve_dependencies(
            cached_files,
            changed_files,
            module_graph,
            module.specifier.clone(),
          );
        });
      }
    })
  }
}
