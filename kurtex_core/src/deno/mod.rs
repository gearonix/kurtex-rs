pub mod ops;
pub mod runtime;

// V8 -> Rust extension loader.
pub trait ExtensionLoader {
  fn load(&self) -> deno_core::Extension;
}
