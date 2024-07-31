use deno_core::{extension, Extension, op2};
use deno_core::error::AnyError;

#[op2(async)]
async fn op_test() -> Result<(), AnyError> {
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
            op_test
        ]
    };

    vec![kurtex::init_ops()]
  }
}
