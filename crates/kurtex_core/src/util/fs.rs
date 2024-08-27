use std::env;
use std::path::{Path, PathBuf};

pub fn kurtex_tmp_dir() -> PathBuf {
  env::temp_dir().join("kurtex-tmp")
}

pub fn add_file_extension<S>(
  path: S,
  extension: impl AsRef<Path>,
) -> PathBuf
where
  S: Into<PathBuf>,
{
  let mut path = path.into();

  match path.extension() {
    None => path.set_extension(extension.as_ref()),
    Some(ext) => {
      let mut ext = ext.to_os_string();
      ext.push(".");
      ext.push(extension.as_ref());
      path.set_extension(ext)
    }
  };

  path
}

#[cfg(test)]
mod tests {
  use crate::fs::add_file_extension;

  #[test]
  fn test_add_file_extension() {
    let path = add_file_extension("/dev/main", "vue");
    let existing_path = add_file_extension("/dev/main.ts", "vue");

    assert_eq!(path.to_str().unwrap(), "/dev/main.vue");
    assert_eq!(existing_path.to_str().unwrap(), "/dev/main.ts.vue");
  }
}
