use std::process::ExitCode;

#[derive(Debug)]
pub enum CliResult {
  None,
}

// TODO: expand impl
impl std::process::Termination for CliResult {
  fn report(self) -> std::process::ExitCode {
    ExitCode::from(0)
  }
}
