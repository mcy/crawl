//! Texels, terminal elements.
//!
//! A *texel* is Crawl's abstraction for a cell in a terminal. They're not
//! *quite* like cells, because they carry a little bit more information. See
//! the [`Texel`] type for more info.

pub use palette::named as colors;

/// An RGB value used by a [`Texel`].
pub type Rgb = palette::Srgb<u8>;

/// A foreground or background color for a [`Texel`].
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Color {
  /// A solid RGB value.
  Rgb(Rgb),

  /// The "default" value, i.e., reset to whatever the terminal's default colors
  /// are.
  Reset,

  /// Inherit whatever color the layer below had; if no such layer is present,
  /// behaves like `Reset`.
  Inherit,
}

impl From<Rgb> for Color {
  fn from(rgb: Rgb) -> Self {
    Self::Rgb(rgb)
  }
}

/// A character weight, ranging from light to bold.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[allow(missing_docs)]
#[rustfmt::skip]
pub enum Weight {
  Normal, Light, Bold, Inherit,
}

/// A "terminal element", analogous to a pixel or voxel.
///
/// A texel consists of a "glyph" (a printable character), a foreground color,
/// and a background color; colors are optional.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Texel {
  glyph: Option<char>,
  fg: Rgb,
  bg: Rgb,
  meta: Meta,
}

bitflags::bitflags! {
  struct Meta: u16 {
    const WEIGHT_BOLD = 1 << 0;
    const WEIGHT_DIM = 1 << 1;
    const WEIGHT_INHERIT = Self::WEIGHT_DIM.bits | Self::WEIGHT_BOLD.bits;

    const ULINE = 1 << 2;

    const BG_RESET = 1 << 8;
    const BG_INHERIT = 1 << 9;
    const FG_RESET = 1 << 10;
    const FG_INHERIT = 1 << 11;
  }
}

impl Texel {
  /// Creates a new invisible texel.
  #[inline]
  pub fn empty() -> Self {
    Self {
      glyph: None,
      fg: colors::BLACK,
      bg: colors::BLACK,
      meta: Meta::FG_INHERIT | Meta::BG_INHERIT,
    }
  }

  /// Creates a new colorless texel with the given glyph.
  #[inline]
  pub fn new(glyph: char) -> Self {
    Self {
      glyph: Some(glyph),
      fg: colors::BLACK,
      bg: colors::BLACK,
      meta: Meta::FG_RESET | Meta::BG_RESET,
    }
  }

  /// Returns this texel's glyph.
  #[inline]
  pub fn glyph(self) -> Option<char> {
    self.glyph
  }

  /// Returns a copy of this texel with the given glyph.
  #[inline]
  pub fn with_glyph(mut self, glyph: impl Into<Option<char>>) -> Self {
    self.glyph = glyph.into();
    self
  }

  /// Returns this texel's foreground color.
  #[inline]
  pub fn fg(self) -> Color {
    if self.meta.contains(Meta::FG_RESET) {
      Color::Reset
    } else if self.meta.contains(Meta::FG_INHERIT) {
      Color::Inherit
    } else {
      self.fg.into()
    }
  }

  /// Returns a copy of this texel with the given foreground color.
  #[inline]
  pub fn with_fg(mut self, color: impl Into<Color>) -> Self {
    self.meta.remove(Meta::FG_RESET);
    self.meta.remove(Meta::FG_INHERIT);
    match color.into() {
      Color::Rgb(rgb) => self.fg = rgb,
      Color::Reset => self.meta |= Meta::FG_RESET,
      Color::Inherit => self.meta |= Meta::FG_INHERIT,
    }
    self
  }

  /// Returns this texel's background color.
  #[inline]
  pub fn bg(self) -> Color {
    if self.meta.contains(Meta::BG_RESET) {
      Color::Reset
    } else if self.meta.contains(Meta::BG_INHERIT) {
      Color::Inherit
    } else {
      self.bg.into()
    }
  }

  /// Returns a copy of this texel with the given background color.
  #[inline]
  pub fn with_bg(mut self, color: impl Into<Color>) -> Self {
    self.meta.remove(Meta::BG_RESET);
    self.meta.remove(Meta::BG_INHERIT);
    match color.into() {
      Color::Rgb(rgb) => self.bg = rgb,
      Color::Reset => self.meta |= Meta::BG_RESET,
      Color::Inherit => self.meta |= Meta::BG_INHERIT,
    }
    self
  }

  /// Returns this texel's weight.
  pub fn weight(self) -> Weight {
    if self.meta.contains(Meta::WEIGHT_INHERIT) {
      Weight::Inherit
    } else if self.meta.contains(Meta::WEIGHT_BOLD) {
      Weight::Bold
    } else if self.meta.contains(Meta::WEIGHT_DIM) {
      Weight::Light
    } else {
      Weight::Normal
    }
  }

  /// Returns a copy of this texel with the given weight.
  #[inline]
  pub fn with_weight(mut self, weight: Weight) -> Self {
    self.meta.remove(Meta::WEIGHT_INHERIT);
    match weight {
      Weight::Normal => {}
      Weight::Bold => self.meta |= Meta::WEIGHT_BOLD,
      Weight::Light => self.meta |= Meta::WEIGHT_DIM,
      Weight::Inherit => self.meta |= Meta::WEIGHT_INHERIT,
    }
    self
  }

  /// Layers `other` over this `Texel`, following any relevant inheritance
  /// rules.
  #[inline]
  pub fn add_layer(mut self, other: Texel) -> Self {
    if let Some(glyph) = other.glyph {
      self.glyph = Some(glyph);
    }
    if other.fg() != Color::Inherit {
      self = self.with_fg(other.fg());
    }
    if other.bg() != Color::Inherit {
      self = self.with_bg(other.bg());
    }
    if other.weight() != Weight::Inherit {
      self = self.with_weight(other.weight());
    }
    self
  }
}
