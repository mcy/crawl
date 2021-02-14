//! Input processing utilties.

use std::collections::HashSet;
use std::time::Duration;

use crate::timing::SystemTimer;

pub use crossterm::event::KeyCode;
pub use crossterm::event::KeyEvent;
pub use crossterm::event::KeyModifiers;

/// A tracker for a frame's key presses.
///
/// Due to the nature of teletype terminals, the only inputs we can really
/// capture are key-presses as recorded by the VT100 emulator. This struct
/// tracks all inputs for a particular frame, which can be querried by different
/// systems throughout the frame.
///
/// At the begining of each frame [`start_frame()`] should be called to load up
/// that frame's inputs from `stdin`.
pub struct UserInput {
  keys: HashSet<KeyEvent>,
}

impl UserInput {
  /// Creates a new `UserInput`.
  pub fn new() -> Self {
    Self {
      keys: HashSet::new(),
    }
  }

  /// Checks whether `code` was pressed this frame.
  pub fn has_key(&self, code: KeyCode) -> bool {
    self
      .keys
      .contains(&KeyEvent::new(code, KeyModifiers::empty()))
  }

  /// Clears internal buffers and collects new inputs from `stdin`.
  ///
  /// This function should be called at the start of each frame, so that systems
  /// downstream of it can query it for inputs.
  pub fn start_frame(&mut self) {
    use crossterm::event;

    self.keys.clear();
    while event::poll(Duration::default()).unwrap() {
      match event::read().unwrap() {
        event::Event::Key(e) => self.keys.insert(e),
        _ => continue,
      };
    }
  }
}

/// System for loading inputs from from `stdin` into a [`UserInput`] for
/// processing on a given frame.
#[legion::system]
pub fn start_frame(
  #[resource] input: &mut UserInput,
  #[resource] timer: &SystemTimer,
) {
  let _t = timer.start("input::start_frame()");
  input.start_frame();
}
