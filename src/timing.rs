//! Timing primitives.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::time::Duration;
use std::time::Instant;

/// A timer for maintaining a stable FPS.
pub struct FrameTimer {
  frame_count: u64,
  last_frame: Instant,

  fps: f64,
  last_measurement: Instant,
  last_measurement_frame: u64,
}

impl FrameTimer {
  /// Creates a new `FrameTimer`.
  pub fn new() -> FrameTimer {
    FrameTimer {
      frame_count: 0,
      last_frame: Instant::now(),
      fps: 0.0,
      last_measurement: Instant::now(),
      last_measurement_frame: 0,
    }
  }

  /// Returns the number of frames timed so far.
  pub fn frame_count(&self) -> u64 {
    self.frame_count
  }

  /// Measures the frames per second at the given measurement interval.
  ///
  /// This function should be called once per frame; once the given interval
  /// has elapsed, the FPS will be computed as the average frame time since the
  /// measurement occured.This function caches the framerate between
  /// measurements.
  pub fn measure_fps(&mut self, measurement_interval: Duration) -> f64 {
    if self.last_measurement.elapsed() < measurement_interval {
      return self.fps;
    }

    let frames = (self.frame_count - self.last_measurement_frame) as f64;
    let fps = frames / self.last_measurement.elapsed().as_secs_f64();

    self.fps = fps;
    self.last_measurement = Instant::now();
    self.last_measurement_frame = self.frame_count;

    self.fps
  }

  /// Ends a frame, blocking until the minimum frame length for the given FPS
  /// is reached.
  ///
  /// This function should be called once per frame.
  pub fn end_frame(&mut self, target_fps: u32) {
    let frame_time = Duration::from_secs(1) / target_fps;
    if let Some(time_left) = frame_time.checked_sub(self.last_frame.elapsed()) {
      thread::sleep(time_left);
    }
    self.last_frame = Instant::now();
    self.frame_count += 1;
  }
}

/// A timer for measuring the average time spent on a particular operation,
/// for computing debug timings.
///
/// This timer can keep track of several different operations, each of which is
/// tracked by a string "tag", such as `my_system.foo`.
pub struct SystemTimer {
  table: HashMap<&'static str, Mutex<TimerInner>>,
}

impl SystemTimer {
  /// Creates a new `SystemTimer`.
  pub fn new() -> Self {
    Self {
      table: HashMap::new(),
    }
  }

  /// Adds the tag `system` to the set of tags usable by this timer.
  pub fn register(&mut self, system: &'static str) {
    self.table.insert(system, Mutex::new(TimerInner::new()));
  }

  /// Starts a timing measurement for `system`.
  ///
  /// The measurement is completed when the returned guard value is dropped,
  /// which will then be added to the running total.
  #[must_use]
  pub fn start(&mut self, system: &'static str) -> SystemTimerGuard<'_> {
    match self.table.get(system) {
      Some(lock) => {
        let mut lock = lock.lock().unwrap();
        lock.last_start = Instant::now();
        SystemTimerGuard(Some(lock))
      }
      None => SystemTimerGuard(None),
    }
  }

  /// Returns the total time measured by this timer for `system`.
  pub fn total_time(&self, system: &'static str) -> Duration {
    self
      .table
      .get(system)
      .map(|s| s.lock().unwrap().total_time)
      .unwrap_or_default()
  }

  /// Measures the average measured time since the last sampling interval
  /// for `system`.
  ///
  /// This function performs an averating operation after `measurement_interval`
  /// has elapsed since it was last called, and caches the measurement in
  /// between calls.
  pub fn measure(
    &mut self,
    system: &'static str,
    measurement_interval: Duration,
  ) -> Duration {
    let mut inner = match self.table.get(system) {
      Some(lock) => lock.lock().unwrap(),
      None => return Duration::default(),
    };

    if inner.last_measurement.elapsed() < measurement_interval {
      return inner.timing;
    }

    let timing = inner.raw_time / inner.measurements;
    inner.timing = timing;
    inner.raw_time = Duration::default();
    inner.last_measurement = Instant::now();
    inner.measurements = 0;
    timing
  }
}

struct TimerInner {
  last_start: Instant,
  total_time: Duration,
  raw_time: Duration,
  measurements: u32,

  timing: Duration,
  last_measurement: Instant,
}

impl TimerInner {
  fn new() -> Self {
    Self {
      last_start: Instant::now(),
      total_time: Duration::default(),
      raw_time: Duration::default(),
      measurements: 0,

      timing: Duration::default(),
      last_measurement: Instant::now(),
    }
  }
}

/// A guard for a [`SystemTimer::start()`] call.
pub struct SystemTimerGuard<'a>(Option<MutexGuard<'a, TimerInner>>);

impl Drop for SystemTimerGuard<'_> {
  fn drop(&mut self) {
    if let Some(inner) = &mut self.0 {
      let elapsed = inner.last_start.elapsed();
      inner.total_time += elapsed;
      inner.raw_time += elapsed;
      inner.measurements += 1;
    }
  }
}
