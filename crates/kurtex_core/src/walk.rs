use std::path::{Path, PathBuf};
use globwalk;

pub const DEFAULT_EXTENSIONS: [&'static str; 4] = ["ts", "js", "mjs", "cjs"];

#[derive(Clone)]
pub struct Extensions(pub Vec<&'static str>);

impl Default for Extensions {
  fn default() -> Self {
    Self(DEFAULT_EXTENSIONS.to_vec())
  }
}

impl Extensions {
  fn empty() -> Self {
    Self(vec![])
  }
}

impl Into<Vec<&str>> for Extensions {
  fn into(self) -> Vec<&'static str> {
    self.0
  }
}

pub struct Walk<'a> {
  extensions: Extensions,
  paths: Vec<String>,
  root_dir: &'a Path,
}

impl<'a> Walk<'a> {
  pub const WALKER_MAX_DEPTH: i32 = 25;

  pub fn new<S>(paths: &[S], root_dir: &'a Path) -> Self
  where
    S: AsRef<str>,
  {
    Self {
      root_dir,
      extensions: Extensions::empty(),
      paths: paths.iter().map(|p| p.as_ref().to_string()).collect(),
    }
  }

  pub fn with_extensions(mut self, extensions: Extensions) -> Self {
    self.extensions = extensions;
    self
  }

  pub fn build(self) -> Box<dyn Iterator<Item = PathBuf> + Send> {
    if self.paths.is_empty() {
      return Box::new(std::iter::empty());
    }

    let extensions: Vec<&str> = self.extensions.into();
    let paths = if extensions.is_empty() {
      self.paths
    } else {
      let mut updated_paths = Vec::new();
      let mut extensions_iter = extensions.iter();

      while let Some(&ext) = extensions_iter.next() {
        updated_paths.extend(self.paths.iter().map(|path| {
          let updated_path = Path::new(path).with_extension(ext);

          updated_path.to_str().map(|s| s.to_owned()).unwrap()
        }));
      }

      assert!(!updated_paths.is_empty());

      updated_paths
    };

    let walker =
      globwalk::GlobWalkerBuilder::from_patterns(self.root_dir, &paths)
        .max_depth(Self::WALKER_MAX_DEPTH as usize)
        .follow_links(false)
        .build()
        .unwrap()
        .into_iter()
        .filter_map(Result::ok)
        .map(|e| e.into_path());

    Box::new(walker)
  }
}

#[cfg(test)]
mod test {
  use std::path::PathBuf;
  use tokio::fs;

  use tokio::fs::File;

  use crate::util::fs::kurtex_tmp_dir;
  use crate::walk::{Extensions, Walk};

  #[tokio::test]
  async fn test_walker_with_extensions() {
    let mut tmp_dir = kurtex_tmp_dir();
    tmp_dir.join("tests/walker").clone_into(&mut tmp_dir);

    fs::create_dir_all(tmp_dir.clone()).await.unwrap();
    assert!(tmp_dir.exists(), "tmp_dir was not created.");

    let test_files = ["bar", "foo"];

    for file in test_files {
      let file = PathBuf::from(file).with_extension("ts");
      File::create(tmp_dir.join(file)).await.unwrap();
    }

    println!(": {:?}", ["rs, ts"].to_vec());
    let mut paths = Walk::new(&test_files, &tmp_dir)
      .with_extensions(Extensions(["rs", "ts"].to_vec()))
      .build()
      .collect::<Vec<PathBuf>>();

    paths.sort();

    let expected = ["bar.ts", "foo.ts"];
    assert_eq!(paths, expected.map(|f| tmp_dir.join(f)));

    fs::remove_dir_all(kurtex_tmp_dir()).await.unwrap();
  }
}
