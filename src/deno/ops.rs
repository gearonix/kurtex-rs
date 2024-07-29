use deno_core::error::AnyError;
use deno_core::{extension, op2, v8, Extension};

#[op2(async)]
#[string]
async fn test(callback: &v8::Function) -> Result<(), AnyError> {
  println!("TESTTESTTEST");
  Ok(())
}

pub struct ResolverOps;

impl ResolverOps {
  #[inline]
  #[must_use]
  pub const fn get_snapshot_binary() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/KURTEX_SNAPSHOT.bin"))
  }

  #[inline]
  #[must_use]
  pub fn initialize_extensions() -> Vec<Extension> {
    extension! {
        kurtex,
        ops = [
            test
        ]
    };

    vec![kurtex::init_ops()]
  }
}
