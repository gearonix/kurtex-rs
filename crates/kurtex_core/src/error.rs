use std::borrow::Cow;
use std::fmt::Formatter;

pub type AnyResult<T = ()> = Result<T, anyhow::Error>;
pub type AnyError = anyhow::Error;

struct KurtexError {
  message: Cow<'static, str>,
}

impl std::fmt::Debug for KurtexError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(&format!("{}: {}", "Kurtex", &self.message))
  }
}

impl std::fmt::Display for KurtexError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(&format!("{}: {}", "Kurtex", &self.message))
  }
}

impl std::error::Error for KurtexError {}

pub fn generic_error<M>(message: M) -> AnyError
where
  M: Into<Cow<'static, str>>,
{
  let ktx_error = KurtexError { message: message.into() };

  ktx_error.into()
}
