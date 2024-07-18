use std::env;
use std::path::{Path, PathBuf};

#[must_use]
pub fn find_up_files(
  filenames: &[&str],
  working_dir: Option<&Path>,
) -> Option<PathBuf> {
  let mut current_dir = working_dir
    .map(|dir| dir.to_path_buf())
    .unwrap_or_else(|| env::current_dir().unwrap());

  loop {
    for &filename in filenames {
      let candidate = current_dir.join(filename);

      if candidate.exists() {
        return Some(candidate);
      }
    }

    if let Some(parent) = current_dir.parent() {
      current_dir = parent.to_path_buf()
    } else {
      break;
    }
  }

  None
}
