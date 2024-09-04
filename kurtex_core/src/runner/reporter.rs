use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, time};

use anyhow::{anyhow, bail};
use log::debug;
use nu_ansi_term::Color::{Blue, LightBlue, LightGray, LightGreen, LightYellow, Red, White};
use nu_ansi_term::{Color, Style};
use rccell::RcCell;

use crate::runner::collector::RunnerCollectorContext;
use crate::{
  CollectorFile, CollectorMode, CollectorNode, CollectorStatus,
  CollectorTask,
};

// TODO: listr
pub struct KurtexDefaultReporter {
  start_time: time::Instant,
}

pub trait Reporter {
  fn paint(&self, color: Color, msg: String) {
    println!("{}", color.paint(msg))
  }

  fn paint_if<T>(&self, failed: &Vec<T>, color: Color, msg: String) {
    if !failed.is_empty() {
      println!("{}", color.paint(msg))
    }
  }

  fn start(&self) {}
  fn report_collected(&mut self);
  fn report_finished(&self, ctx: &RunnerCollectorContext);
  fn begin_file(&self, file: Arc<CollectorFile>) {}
  fn end_file(&self, file: Arc<CollectorFile>) {}
  fn begin_node(&self, node: Arc<Mutex<CollectorNode>>) {}
  fn end_node(&self, node: Arc<Mutex<CollectorNode>>) {}
  fn begin_task(&self, task: Arc<Mutex<CollectorTask>>) {}
  fn end_task(&self, task: Arc<Mutex<CollectorTask>>) {}
  fn watcher_started(&self, ctx: &RunnerCollectorContext) {}
  fn watcher_rerun(&self, files: &Vec<PathBuf>, file: PathBuf) {}
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

  fn report_finished(&self, ctx: &RunnerCollectorContext) {
    let end_time = self.start_time.elapsed();
    let milliseconds = end_time.as_micros() as f64 / 1_000.0;

    debug!("Reporter: finished with {}.", end_time.as_millis());
    println!();

    let mut failed_files = ctx
      .files
      .iter()
      .filter(|file| file.error.is_some())
      .collect::<Vec<&Arc<CollectorFile>>>();

    let mut failed = task_vec![];
    let mut passed = task_vec![];
    let mut runnable = task_vec![];
    let mut skipped = task_vec![];
    let mut todo = task_vec![];

    for task_rc in ctx.tasks.iter() {
      let task = task_rc.lock().unwrap();
      let is_runnable = matches!(
        task.status,
        CollectorStatus::Pass | CollectorStatus::Fail
      );

      match task.status {
        CollectorStatus::Fail => failed.push(task_rc.clone()),
        CollectorStatus::Pass => passed.push(task_rc.clone()),
        CollectorStatus::Custom(CollectorMode::Skip) => {
          skipped.push(task_rc.clone())
        }
        CollectorStatus::Custom(CollectorMode::Todo) => {
          todo.push(task_rc.clone())
        }
        _ => {}
      };

      if is_runnable {
        runnable.push(task_rc.clone());
      }
    }

    let print_failed_files = || {
      let has_failed = !failed_files.is_empty();

      has_failed.then(|| {
        println!(
          "{}",
          format!("Failed to parse {} files", failed_files.len())
        );

        failed_files.iter().for_each(|file| {
          let file_path = file.file_path.display().to_string();
          let error = file.error.as_ref().map(|e| e.to_string());

          self.paint(Red, format!("\n {}", file_path));
          eprintln!("{}", error.unwrap());
          println!();
        });
      })
    };

    let print_failed_tasks = || {
      let has_failed = !failed.is_empty();

      has_failed.then(|| {
        println!("{}", format!("Failed tests ({})", failed.len()));

        failed.iter().for_each(|task| {
          let task = task.lock().unwrap();
          let error = task.error.as_ref().map(|e| e.to_string());

          let bold_red = Style::new().bold().on(Red);
          let fail_mark = format!(" {} ", bold_red.paint("FAIL"));

          println!("\n {} {}", fail_mark, task.name);
          eprintln!("{}", error.unwrap());
          println!();
        });
      })
    };

    print_failed_files();
    print_failed_tasks();

    self.paint_if(
      &failed_files,
      White,
      format!("Failed to parse {} files", failed_files.len()),
    );

    self.paint_if(
      &failed,
      Red,
      format!("Failed {} / {}", failed.len(), runnable.len()),
    );

    self.paint(
      LightGreen,
      format!("Passed {} / {}", passed.len(), runnable.len()),
    );

    self.paint_if(
      &skipped,
      LightYellow,
      format!("Skipped  {}", skipped.len()),
    );

    self.paint_if(&todo, White, format!("Todo  {} ", todo.len()));

    println!("Time {}ms", milliseconds);
  }

  fn watcher_started(&self, ctx: &RunnerCollectorContext) {
    let has_failed = ctx.tasks.iter().any(|task| {
      let task = task.lock().unwrap();
      task.status == CollectorStatus::Fail
    });
    let has_failed_files =
      ctx.files.iter().any(|file| file.error.is_some());

    if has_failed | has_failed_files {
      self.paint(
        Red,
        "\n Tests failed. Watching for file changes...".to_string(),
      )
    } else {
      self
        .paint(LightGreen, "\n Watching for file changes...".to_string())
    }
  }

  fn watcher_rerun(&self, files: &Vec<PathBuf>, trigger: PathBuf) {
    let path =
      trigger.strip_prefix(env::current_dir().unwrap()).unwrap();

    self.paint(
      Blue,
      format!("File {} changed, re-running tests...", path.display()),
    );
  }
}

impl Default for KurtexDefaultReporter {
  fn default() -> Self {
    KurtexDefaultReporter::new()
  }
}

#[macro_export]
macro_rules! create_task_vector {
  ($($tt:tt)*) => {{
    use crate::CollectorTask;

    let result: Vec<
      ::std::sync::Arc<::std::sync::Mutex<CollectorTask>>,
    > = Vec::new();
    result
  }};
}

pub use create_task_vector as task_vec;
