//! `curses` helper library.
//!
//! All errors from `curses` will panic, since those errors are effectively
//! unrecoverable.
//!
//! Note that this module doesn't *actually* use `libcurses`, and merely
//! emulates its behavior at a high level in terms of another library.

use std::io;
use std::time::Duration;

use crate::render::texel;
use crate::render::texel::Texel;

/// Returns the current dimensions of the terminal window.
pub fn dims() -> (usize, usize) {
  let (cols, rows) = crossterm::terminal::size().unwrap();
  (rows as _, cols as _)
}

/// A low-level curses context.
pub struct Curses<W: io::Write = io::Stdout> {
  w: W,
}

impl Curses {
  /// Initializes the `curses` environment.
  pub fn init() -> Curses {
    Curses::with(io::stdout())
  }
}

/// Arguments for a draw call.
///
/// See [`Curses::draw()`].
#[allow(missing_docs)]
pub struct DrawCall {
  pub row: usize,
  pub col: usize,
  pub texel: Texel,
}

impl<W: io::Write> Curses<W> {
  /// Initializes the `curses` environment for `w`.
  pub fn with(mut w: W) -> Curses<W> {
    crossterm::execute!(
      w,
      crossterm::terminal::EnterAlternateScreen,
      crossterm::cursor::Hide,
      crossterm::terminal::DisableLineWrap,
    )
    .unwrap();
    crossterm::terminal::enable_raw_mode().unwrap();

    Curses { w }
  }

  /// Draws the character `c` at the given location on the screen.
  pub fn draw(&mut self, call: DrawCall) {
    use crossterm::style::Color;
    use crossterm::style::Colors;
    let fg = match call.texel.fg() {
      texel::Color::Rgb(rgb) => Color::Rgb {
        r: rgb.red,
        g: rgb.green,
        b: rgb.blue,
      },
      _ => Color::Reset,
    };
    let bg = match call.texel.bg() {
      texel::Color::Rgb(rgb) => Color::Rgb {
        r: rgb.red,
        g: rgb.green,
        b: rgb.blue,
      },
      _ => Color::Reset,
    };

    crossterm::queue!(
      self.w,
      crossterm::cursor::MoveTo(call.col as _, call.row as _),
      crossterm::style::SetColors(Colors {
        foreground: Some(fg),
        background: Some(bg),
      }),
      crossterm::style::Print(call.texel.glyph().unwrap_or(' ')),
    )
    .unwrap();
  }

  /// Returns an iterator over currently-buffered keyboard inputs.
  pub fn keys(
    &mut self,
  ) -> impl Iterator<Item = crossterm::event::KeyEvent> + '_ {
    use crossterm::event::*;

    std::iter::from_fn(move || {
      while poll(Duration::default()).unwrap() {
        match read().unwrap() {
          Event::Key(e) => return Some(e),
          _ => continue,
        }
      }
      None
    })
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

impl<W: io::Write> Drop for Curses<W> {
  fn drop(&mut self) {
    self.cleanup();
  }
}
