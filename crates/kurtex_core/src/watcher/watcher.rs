use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::path::{Path, PathBuf};
use std::time;

use anyhow::anyhow;
use deno_core::futures;
use deno_core::futures::channel::mpsc;
use deno_core::futures::channel::mpsc::channel;
use deno_core::futures::SinkExt;
use hashbrown::HashMap;
use notify::{EventKind, INotifyWatcher, RecursiveMode, Watcher};
use tokio::time::timeout as recv_timeout;
use tokio_stream::StreamExt;

use crate::AnyResult;

pub const DEBOUNCER_CHANNEL_BUFFER: usize = 100;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DebouncedEventKind {
  Update,
  Insert,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DebouncedEvent {
  pub path: PathBuf,
  pub kind: DebouncedEventKind,
}

#[derive(Debug)]
struct EventData {
  insert: time::Instant,
  update: time::Instant,
}

impl EventData {
  #[inline(always)]
  fn new(time: time::Instant) -> Self {
    Self { insert: time, update: time }
  }
}

struct DebouncerDataInner {
  event_map: HashMap<PathBuf, EventData>,
  timeout: time::Duration,
  debounce_deadline: Option<time::Instant>,
}

impl DebouncerDataInner {
  pub fn new(timeout: time::Duration) -> Self {
    DebouncerDataInner {
      event_map: HashMap::default(),
      timeout,
      debounce_deadline: None,
    }
  }

  #[inline]
  pub fn next_tick(&self) -> Option<time::Duration> {
    let now = time::Instant::now();
    self
      .debounce_deadline
      .map(|deadline| deadline.saturating_duration_since(now))
  }

  pub fn extract_debounced_events(&mut self) -> Vec<DebouncedEvent> {
    let mut events_expired: Vec<DebouncedEvent> =
      Vec::with_capacity(self.event_map.len());
    let mut data_back = HashMap::with_capacity(self.event_map.len());
    self.debounce_deadline = None;

    for (path, event) in self.event_map.drain() {
      if event.update.elapsed() >= self.timeout {
        events_expired
          .push(DebouncedEvent::new(path.clone(), DebouncedEventKind::Update));
      } else if event.insert.elapsed() >= self.timeout {
        Self::update_deadline(
          self.timeout,
          &mut self.debounce_deadline,
          &event,
        );

        data_back.insert(path.clone(), event);
        events_expired
          .push(DebouncedEvent::new(path.clone(), DebouncedEventKind::Insert));
      } else {
        Self::update_deadline(
          self.timeout,
          &mut self.debounce_deadline,
          &event,
        );

        data_back.insert(path.clone(), event);
      }
    }

    self.event_map = data_back;
    events_expired
  }

  fn update_deadline(
    timeout: time::Duration,
    debounce_deadline: &mut Option<time::Instant>,
    event: &EventData,
  ) {
    let deadline_candidate = event.update + timeout;
    match debounce_deadline {
      Some(current_deadline) => {
        if *current_deadline > deadline_candidate {
          *debounce_deadline = Some(deadline_candidate);
        }
      }
      None => *debounce_deadline = Some(deadline_candidate),
    }
  }

  #[inline(always)]
  fn register_event(&mut self, event: notify::Event) {
    let time = time::Instant::now();
    let deadline_candidate = time + self.timeout;

    if self.debounce_deadline.is_none() {
      self.debounce_deadline = Some(deadline_candidate)
    }

    if let EventKind::Modify(_) = event.kind {
      event.paths.iter().for_each(|path| {
        let has_tilde = path.to_string_lossy().ends_with("~");

        if has_tilde {
          return;
        }

        if let Some(v) = self.event_map.get_mut(path) {
          v.update = time;
        } else {
          self.event_map.insert(path.clone(), EventData::new(time));
        }
      })
    }
  }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DebouncerConfig {
  timeout: time::Duration,
  inner: notify::Config,
}

impl DebouncerConfig {
  pub fn new(timeout: time::Duration) -> Self {
    let notify_config = notify::Config::default().with_poll_interval(timeout);

    DebouncerConfig { timeout, inner: notify_config }
  }
}

impl DebouncedEvent {
  pub fn new(path: PathBuf, kind: DebouncedEventKind) -> Self {
    Self { path, kind }
  }
}

pub type DebounceEventResult = AnyResult<Vec<DebouncedEvent>>;

pub trait DebounceEventHandler: Send + 'static {
  async fn send_event(&mut self, event: DebounceEventResult);
}

impl<F> DebounceEventHandler for F
where
  F: FnMut(DebounceEventResult) + Send + 'static,
{
  async fn send_event(&mut self, event: DebounceEventResult) {
    (self)(event)
  }
}

impl DebounceEventHandler for mpsc::Sender<DebounceEventResult> {
  async fn send_event(&mut self, event: DebounceEventResult) {
    let _ = self.send(event).await;
  }
}

pub struct AsyncWatcherDebouncer<T: Watcher = INotifyWatcher> {
  pub(crate) watcher: T,
  inner_tx: mpsc::Sender<InnerEvent>,
}

#[derive(Debug)]
pub enum InnerEvent {
  NotifyEvent(AnyResult<notify::Event>),
  Shutdown,
}

impl<T: Watcher> AsyncWatcherDebouncer<T> {
  pub fn new<F>(timeout: time::Duration, mut event_handler: F) -> Self
  where
    F: DebounceEventHandler,
    T: notify::Watcher,
  {
    Self::new_inner(timeout, event_handler).unwrap()
  }

  fn new_inner<F>(
    timeout: time::Duration,
    mut event_handler: F,
  ) -> AnyResult<AsyncWatcherDebouncer<T>>
  where
    F: DebounceEventHandler,
    T: notify::Watcher,
  {
    let debouncer_config = DebouncerConfig::new(timeout);
    let (mut inner_tx, mut inner_rx) =
      channel::<InnerEvent>(DEBOUNCER_CHANNEL_BUFFER);

    deno_core::unsync::spawn(async move {
      let mut debouncer = DebouncerDataInner::new(timeout);

      'outer: loop {
        match debouncer.next_tick() {
          Some(timeout) => {
            let timeout_result = recv_timeout(timeout, inner_rx.next()).await;

            match timeout_result {
              Ok(Some(InnerEvent::NotifyEvent(ev))) => match ev {
                Ok(ev) => debouncer.register_event(ev),
                Err(e) => event_handler.send_event(Err(e)).await,
              },
              Ok(Some(InnerEvent::Shutdown)) => break 'outer,
              Err(e) => {
                if let IoErrorKind::TimedOut = IoError::from(e).kind() {
                  let send_data = debouncer.extract_debounced_events();

                  if !send_data.is_empty() {
                    event_handler.send_event(Ok(send_data)).await;
                  }
                }
              }
              _ => unreachable!(),
            }
          }
          None => match inner_rx.next().await {
            Some(InnerEvent::NotifyEvent(ev)) => match ev {
              Ok(ev) => debouncer.register_event(ev),
              Err(e) => event_handler.send_event(Err(e)).await,
            },
            Some(InnerEvent::Shutdown) => break 'outer,
            None => break 'outer,
          },
        }
      }
    });

    let mut inner_tx_c = inner_tx.clone();
    let watcher = T::new(
      move |event: Result<notify::Event, notify::Error>| {
        futures::executor::block_on(async {
          let _ = inner_tx_c
            .send(InnerEvent::NotifyEvent(event.map_err(|e| {
              anyhow!("Notify internal error occurred. {}", e.to_string())
            })))
            .await
            .unwrap();
        });
      },
      debouncer_config.inner,
    )?;

    let guard = AsyncWatcherDebouncer { watcher, inner_tx };

    Ok(guard)
  }

  pub fn close(&mut self) {
    futures::executor::block_on(async {
      let _ = self.inner_tx.send(InnerEvent::Shutdown).await.unwrap();
    });
  }

  pub fn watch(&mut self, path: &Path) {
    self.watcher.watch(path, RecursiveMode::Recursive).unwrap();
  }
}
