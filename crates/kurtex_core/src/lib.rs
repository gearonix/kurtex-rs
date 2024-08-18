pub mod collector;
pub mod config;
pub mod deno;
pub mod runner;
pub mod util;
pub mod walk;

pub type AnyResult<T = ()> = Result<T, anyhow::Error>;
pub type AnyError = anyhow::Error;
