use std::sync::{Arc, Mutex};
use std::time;

use crate::{CollectorNode, CollectorTask};
use log::debug;
use nu_ansi_term::Color;

pub struct KurtexDefaultReporter {
  start_time: time::Instant,
}

// TODO: better namings
pub trait Reporter {
  fn start(&self) {}
  fn report_collected(&mut self);
  fn report_finished(&self);
  fn begin_file(&self) {}
  fn end_file(&self) {}
  fn begin_node(&self, node: Arc<Mutex<CollectorNode>>) {}
  fn end_node(&self, node: Arc<Mutex<CollectorNode>>) {}
  fn begin_task(&self, task: Arc<Mutex<CollectorTask>>) {}
  fn end_task(&self, task: Arc<Mutex<CollectorTask>>) {}
}

impl KurtexDefaultReporter {
  pub fn new() -> Self {
    let start_time = time::Instant::now();

    KurtexDefaultReporter { start_time }
  }
}

impl Reporter for KurtexDefaultReporter {
  fn report_collected(&mut self) {
    self.start_time = time::Instant::now();

    debug!("Reporter: collected test files.");
  }

  fn report_finished(&self) {
    let end_time = self.start_time.elapsed();

    // TODO: onFinished
    debug!("Reporter: finished with {}.", end_time.as_millis());
    println!();

    let passed_tests = 0;
    let runnable_tests = 0;

    println!(
      "{}",
      Color::Green.paint(format!("Passed {passed_tests} / {runnable_tests}"))
    );
    println!("Time {}ms", end_time.as_millis());
  }
}

impl Default for KurtexDefaultReporter {
  fn default() -> Self {
    KurtexDefaultReporter::new()
  }
}
