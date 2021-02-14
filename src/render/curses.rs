//! `curses` helper library.
//!
//! All errors from `curses` will panic, since those errors are effectively
//! unrecoverable.
//!
//! Note that this module doesn't *actually* use `libcurses`, and merely
//! emulates its behavior at a high level in terms of another library.

use std::io;
use std::io::Write as _;

use crate::render::texel;
use crate::render::texel::Texel;

/// Returns the current dimensions of the terminal window.
pub fn dims() -> (usize, usize) {
  let (cols, rows) = crossterm::terminal::size().unwrap();
  (rows as _, cols as _)
}

/// A low-level curses context.
pub struct Curses {
  w: io::Stdout,
}

impl Curses {
  /// Initializes the `curses` environment.
  pub fn init() -> Curses {
    let mut c = Curses { w: io::stdout() };

    crossterm::execute!(
      c.w,
      crossterm::terminal::EnterAlternateScreen,
      crossterm::cursor::Hide,
      crossterm::terminal::DisableLineWrap,
    )
    .unwrap();
    crossterm::terminal::enable_raw_mode().unwrap();

    c
  }

  /// Starts a new drawing session, taking a lock on `stdout`.
  ///
  /// The returned value can be used to draw individual cells of the terminal,
  /// though they will not be commited until the returned RAII object is
  /// dropped.
  pub fn draw_session(&self) -> Session<'_> {
    Session(self.w.lock())
  }

  /// Clean up whatever mess the terminal made.
  fn cleanup(&mut self) {
    crossterm::execute!(
      self.w,
      crossterm::terminal::LeaveAlternateScreen,
      crossterm::cursor::Show,
      crossterm::terminal::EnableLineWrap,
    )
    .unwrap();
    crossterm::terminal::disable_raw_mode().unwrap();
    self.w.flush().unwrap();
  }

  /// Destroys the `curses` environment, taking the process along with it.
  pub fn die(&mut self, exit: i32) -> ! {
    self.cleanup();
    std::process::exit(exit);
  }
}

impl Drop for Curses {
  fn drop(&mut self) {
    self.cleanup();
  }
}

/// RAII wrapper for a `stdout` lock, which can be used to perform a long
/// sequence of draw calls without having to hit the `stdout` lock on each one.
pub struct Session<'a>(io::StdoutLock<'a>);

impl Session<'_> {
  /// Draws a texel at the given row-column on the screen.
  pub fn draw(&mut self, rc: (usize, usize), tx: Texel) {
    use crossterm::style::Color;
    use crossterm::style::Colors;
    let fg = match tx.fg() {
      texel::Color::Rgb(rgb) => Color::Rgb {
        r: rgb.red,
        g: rgb.green,
        b: rgb.blue,
      },
      _ => Color::Reset,
    };
    let bg = match tx.bg() {
      texel::Color::Rgb(rgb) => Color::Rgb {
        r: rgb.red,
        g: rgb.green,
        b: rgb.blue,
      },
      _ => Color::Reset,
    };

    let (r, c) = rc;
    crossterm::queue!(
      self.0,
      crossterm::cursor::MoveTo(c as _, r as _),
      crossterm::style::SetColors(Colors {
        foreground: Some(fg),
        background: Some(bg),
      }),
      crossterm::style::Print(tx.glyph().unwrap_or(' ')),
    )
    .unwrap();
  }
}
