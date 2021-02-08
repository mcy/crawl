//! Texels, terminal elements.
//!
//! A *texel* is Crawl's abstraction for a cell in a terminal. They're not
//! *quite* like cells, because they carry a little bit more information. See
//! the [`Texel`] type for more info.

pub use palette::named as colors;

/// A color used by a [`Texel`].
pub type Color = palette::Srgb<u8>;

/// A "terminal element", analogous to a pixel or voxel.
///
/// A texel consists of a "glyph" (a printable character), a foreground color,
/// and a background color; colors are optional.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Texel {
  glyph: char,
  fg: Option<Color>,
  bg: Option<Color>,
  // NOTE: there's currently 14 unused bits in this struct.
}

impl Texel {
  /// Creates a new colorless texel with the given glyph.
  #[inline]
  pub fn new(glyph: char) -> Self {
    Self {
      glyph,
      fg: None,
      bg: None,
    }
  }

  /// Creates a new texel with the given glyph and colors.
  #[inline]
  pub fn colored(glyph: char, fg: Option<Color>, bg: Option<Color>) -> Self {
    Self { glyph, fg, bg }
  }

  /// Returns the glyph.
  #[inline]
  pub fn glyph(self) -> char {
    self.glyph
  }

  /// Returns the foreground color, if one is present.
  #[inline]
  pub fn fg(self) -> Option<Color> {
    self.fg
  }

  /// Returns the background color, if one is present.
  #[inline]
  pub fn bg(self) -> Option<Color> {
    self.bg
  }
}
