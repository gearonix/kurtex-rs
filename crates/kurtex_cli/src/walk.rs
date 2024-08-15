use crate::module_specifier::VALID_EXTENSIONS;
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Extensions(pub Vec<&'static str>);

impl Default for Extensions {
  fn default() -> Self {
    Self(VALID_EXTENSIONS.to_vec())
  }
}

impl Into<Vec<&str>> for Extensions {
  fn into(self) -> Vec<&'static str> {
    self.0
  }
}

pub struct Walk {
  extensions: Extensions,
  paths: Vec<&'static str>,
  root_dir: PathBuf,
}

impl Walk {
  pub const WALKER_MAX_DEPTH: i32 = 25;

  pub fn new<S>(paths: &[S], root_dir: PathBuf) -> Self
  where
    S: AsRef<str>,
  {
    assert!(
      !paths.is_empty(),
      "At least one path must be provided to Walk::new"
    );

    Self { root_dir, extensions: Extensions::default(), paths: paths.to_vec() }
  }

  pub fn with_extensions(mut self, extensions: Extensions) -> Self {
    self.extensions = extensions;
    self
  }

  pub fn build(self) -> impl Iterator<Item = PathBuf> + Send {
    let extensions: Vec<&str> = self.extensions.into();

    let paths = if extensions.is_empty() {
      self.paths
    } else {
      let mut updated_paths = Vec::new();

      while let Some(&ext) = extensions.iter().next() {
        updated_paths.extend(self.paths.iter().map(|path| {
          let updated_path = Path::new(path).with_extension(ext);

          updated_path.to_str().unwrap()
        }));
      }
      assert!(!updated_paths.is_empty());

      updated_paths
    };

    let walker = globwalk::GlobWalkerBuilder::from_patterns(
      self.root_dir,
      paths.iter().map(|f| f.as_os_str()).collect(),
    )
    .max_depth(Self::WALKER_MAX_DEPTH as usize)
    .follow_links(false)
    .build()
    .unwrap()
    .into_iter()
    .filter_map(Result::ok)
    .map(|e| e.into_path());

    walker
  }
}

pub const fn kurtex_tmp_dir() -> PathBuf {
  env::temp_dir().join("kurtex-tmp")
}

#[cfg(test)]
mod test {
  use crate::walk::{kurtex_tmp_dir, Extensions, Walk};
  use std::path::PathBuf;
  use tokio::fs::File;

  #[test]
  async fn test_walker_with_extensions() {
    let mut tmp_dir = kurtex_tmp_dir();
    tmp_dir.join("tests/walker").clone_into(&mut tmp_dir);
    let test_files = ["bar", "foo"];

    for file in test_files {
      let file = PathBuf::from(file).with_extension("ts");
      File::create(file).await?;
    }

    let mut paths = Walk::new(&test_files, tmp_dir)
      .with_extensions(Extensions(["rs, ts"].to_vec()))
      .build()
      .collect();

    paths.sort();

    assert_eq!(paths, vec!["bar.ts", "foo.ts"])
  }
}