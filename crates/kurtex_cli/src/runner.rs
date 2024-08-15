/// A trait for exposing functionality to the CLI.
pub trait Runner {
  type Options;

  fn new(matches: Self::Options) -> Self;
  fn run(self) -> ();
}
