use log::{Metadata, Record};
use nu_ansi_term::{AnsiString, Color};
use std::ffi::OsStr;

// Taken from
// https://github.com/eza-community/eza/blob/main/src/logger.rs
pub fn configure<T: AsRef<OsStr>>(env: Option<T>) {
  let Some(env) = env else { return };

  let env_var = env.as_ref();
  if env_var.is_empty() {
    return;
  }

  if env_var == "trace" {
    log::set_max_level(log::LevelFilter::Trace)
  } else {
    log::set_max_level(log::LevelFilter::Debug)
  }

  if let Err(e) = log::set_logger(LOGGER) {
    eprintln!("Failed to initialize logger: {e}")
  }
}

const LOGGER: &'static Logger = &Logger::default();

#[derive(Debug, Default)]
struct Logger;

impl log::Log for Logger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    true
  }

  fn log(&self, record: &Record) {
    let open = Color::Fixed(243).paint("[");
    let level = level(record.level());
    let level = level.to_string().to_ascii_lowercase();
    let close = Color::Fixed(243).paint("]");

    eprintln!(
      "{}{} {}{} {}",
      open,
      level,
      record.target(),
      close,
      record.args()
    );
  }

  fn flush(&self) {}
}

fn level(level: log::Level) -> AnsiString<'static> {
  #[rustfmt::skip]
    return match level {
        log::Level::Error => Color::Red.paint("ERROR"),
        log::Level::Warn  => Color::Yellow.paint("WARN"),
        log::Level::Info  => Color::Cyan.paint("INFO"),
        log::Level::Debug => Color::Blue.paint("DEBUG"),
        log::Level::Trace => Color::Fixed(245).paint("TRACE"),
    };
}
