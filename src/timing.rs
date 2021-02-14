//! Timing primitives.

use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use chashmap::CHashMap;

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
  table: CHashMap<&'static str, TimerInner>,
  keys: Mutex<Vec<&'static str>>,
}

impl SystemTimer {
  /// Creates a new `SystemTimer`.
  pub fn new() -> Self {
    Self {
      table: CHashMap::new(),
      keys: Mutex::new(Vec::new()),
    }
  }

  /// Starts a timing measurement for `system`.
  ///
  /// The measurement is completed when the returned guard value is dropped,
  /// which will then be added to the running total.
  #[must_use]
  pub fn start(&self, system: &'static str) -> SystemTimerGuard<'_> {
    let keys = &self.keys;
    self.table.upsert(
      system,
      move || {
        keys.lock().unwrap().push(system);
        TimerInner::new()
      },
      |v| v.last_start = Instant::now(),
    );
    SystemTimerGuard(self, system)
  }

  /// Returns the total time measured by this timer for `system`.
  pub fn total_time(&self, system: &'static str) -> Duration {
    self
      .table
      .get(system)
      .map(|s| s.total_time)
      .unwrap_or_default()
  }

  /// Measures the average measured time since the last sampling interval
  /// for `system`.
  ///
  /// This function performs an averating operation after `measurement_interval`
  /// has elapsed since it was last called, and caches the measurement in
  /// between calls.
  pub fn measure(
    &self,
    system: &'static str,
    measurement_interval: Duration,
  ) -> Duration {
    match self.table.get_mut(system) {
      Some(mut inner) => inner.measure(measurement_interval, Instant::now()),
      None => Duration::default(),
    }
  }

  /// Measure the average measure time since the last sampling interval for
  /// every system tracked by `self`.
  ///
  /// This function performs an averating operation after `measurement_interval`
  /// has elapsed since it was last called, and caches the measurement in
  /// between calls.
  pub fn measure_all(
    &self,
    measurement_interval: Duration,
  ) -> impl Iterator<Item = (&'static str, Duration)> + '_ {
    let now = Instant::now();
    let table = &self.table;
    let keys = self.keys.lock().unwrap();
    let mut idx = 0;
    std::iter::from_fn(move || {
      let &system = keys.get(idx)?;
      idx += 1;

      let m = table.get_mut(system)?.measure(measurement_interval, now);
      Some((system, m))
    })
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

  fn measure(&mut self, interval: Duration, now: Instant) -> Duration {
    if now - self.last_measurement < interval {
      return self.timing;
    }

    let timing = self.raw_time / self.measurements;
    self.timing = timing;
    self.raw_time = Duration::default();
    self.last_measurement = now;
    self.measurements = 0;
    timing
  }
}

/// A guard for a [`SystemTimer::start()`] call.
pub struct SystemTimerGuard<'a>(&'a SystemTimer, &'static str);

impl SystemTimerGuard<'_> {
  /// Finishes a timing early.
  pub fn finish(self) {}
}

impl Drop for SystemTimerGuard<'_> {
  fn drop(&mut self) {
    if let Some(mut inner) = self.0.table.get_mut(self.1) {
      let elapsed = inner.last_start.elapsed();
      inner.total_time += elapsed;
      inner.raw_time += elapsed;
      inner.measurements += 1;
    }
  }
}
