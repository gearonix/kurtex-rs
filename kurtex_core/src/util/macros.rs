#[macro_export]
macro_rules! arc {
  ($inner:expr) => {
    ::std::sync::Arc::new($inner)
  };
}

#[macro_export]
macro_rules! arc_mut {
  ($inner:expr) => {
    ::std::sync::Arc::new(::std::sync::Mutex::new($inner))
  };
}
