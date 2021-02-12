//! Widget bars.
//!
//! A widget bar consists of a set of *widgets*, which are variable-length
//! horizontal runs of texels. Widgets can either be of fixed size, or can be
//! *flexible*, allowing them to take on a range of widths. The layout algorithm
//! attempts to fairly distribute space among all widgets.
//!
//! The [`WidgetBar`] type is generic on the actual type of widget. The
//! [`Widget`] trait represents a type of widget (usually, this would be
//! implemented by some `enum`), and the [`Widget::State`] type represents all
//! data that widgets can draw from for rendering. Each frame, the widget state
//! should be updated, after which the bar layout can be recalculated as needed.

use num::integer::div_ceil;

use crate::render::texel::Color;
use crate::render::texel::Texel;

/// A widget type.
///
/// Widgets must implement this trait, which describes the shared state that
/// they draw their information from, and how that state affects the widget's
/// resulting *shape*.
pub trait Widget {
  /// The shared widget state that widgets draw from for rendering.
  type State;

  /// Update a particular widget's shape based on the current state.
  fn update(&self, state: &Self::State, shape: &mut Shape);
}

/// A widget bar, consisting of a list of widgets of various types.
///
/// The type parameter `W` determines the different kinds of widgets, as well
/// as their shared state.
pub struct WidgetBar<W: Widget> {
  state: W::State,
  widgets: Vec<WidgetData<W>>,
  buf: Box<[Texel]>,
  dirty: bool,
}

struct WidgetData<W> {
  priority: i64,
  shape: Shape,
  hint: Hint,
  width: usize,
  ty: W,
}

#[derive(Copy, Clone, Debug)]
enum Hint {
  Flex(usize, Option<usize>),
  Fixed(usize),
  Hidden,
}

impl<W: Widget> WidgetBar<W> {
  /// Creates a new `WidgetBar` with the given initial state.
  pub fn new(state: W::State) -> Self {
    Self {
      state,
      widgets: Vec::new(),
      buf: Box::new([]),
      dirty: true,
    }
  }

  /// Adds a new widget to the bar.
  ///
  /// Priority determines sorting of widgets from left to right: a higher
  /// priority will put it further to the right. Widgets of equal priority are
  /// rendered in an unspecified order.
  ///
  /// This function automatically marks the bar as dirty.
  pub fn push(&mut self, widget: W, priority: i64) {
    self.widgets.push(WidgetData {
      priority,
      shape: Shape::Hidden,
      hint: Hint::Hidden,
      width: 0,
      ty: widget,
    });

    self.mark_dirty();
  }

  /// Returns a reference to the widgets' shared state.
  ///
  /// Note that [`mark_dirty()`] should be called to trigger a layout reflow
  /// if the state changes.
  pub fn state(&self) -> &W::State {
    &self.state
  }

  /// Returns a reference to the widgets' shared state.
  ///
  /// Note that [`mark_dirty()`] should be called to trigger a layout reflow.
  pub fn state_mut(&mut self) -> &mut W::State {
    &mut self.state
  }

  /// Marks the bar as dirty, so that a draw operation will need to re-flow
  /// the widget layout.
  pub fn mark_dirty(&mut self) {
    self.dirty = true;
  }

  /// Recalculates the layout of the widgets on the bar, using the given
  /// overall width.
  fn reflow(&mut self, width: usize) {
    self.widgets.sort_by_key(|w| w.priority);

    // There are three kinds of widgets:
    // - Hidden widgets, which we ignore completely (and mark as hidden).
    // - Fixed-size widgets, which don't need to be reflowed (since they always
    //   take up the same size).
    // - Unbounded widgets, which take up equal portions of whatever the fixed
    //   widgets take up.
    // - Bounded widgets, which are like unbounded widgets except they will only
    //   take up a limited size.
    //
    // Note, however, that an unbounded widget may become fixed, should the
    // remaining portion (after fixed widgets are removed) is too small to hold
    // the unbounded widget. Similarly, if a widget is bounded, and the
    // remaining portion is bigger than requested, it becomes fixed.
    //
    // For now, we do the naive quadratic algorithm, though there's certainly
    // an n log n algorithm we can use instead.

    // First, compute and cache all of the width hints.
    for w in &mut self.widgets {
      w.ty.update(&self.state, &mut w.shape);
      w.hint = w.shape.width_hint();
      w.width = 0;
    }

    // Next, subtract from the available space all of the fixed hints.
    let mut available = width;
    for w in &mut self.widgets {
      if let Hint::Fixed(n) = w.hint {
        w.width = n;
        available = match available.checked_sub(n) {
          Some(n) => n,
          None => {
            // We ran out of space. This is a pathological result that we're
            // just going to hope doesn't happen...
            return;
          }
        };
      }
    }

    // Now, see if any unbounded widgets happen to become fixed, and adjust the
    // available width to compensate. We need to run the whole widget vector
    // until this converges (which is guaranteed: we either converge or run out
    // of space).
    let mut unboundeds = self
      .widgets
      .iter()
      .filter(|w| matches!(w.hint, Hint::Flex(..)))
      .count();
    loop {
      if unboundeds == 0 {
        // Nothing to do; we're out of unbounded widgets to reflow.
        return;
      }
      // This is the width-per-widget, rounded *up*. If a bounded widget can't
      // fit in this space, it needs to be changed to a fixed widget.
      let width_per = div_ceil(available, unboundeds);
      let mut had_change = false;
      for w in &mut self.widgets {
        if let Hint::Flex(min, max) = w.hint {
          let max = max.unwrap_or(width);
          if min <= width_per && width_per < max {
            continue;
          } else if min > width_per {
            had_change = true;
            w.hint = Hint::Fixed(min);
            w.width = min;
            unboundeds -= 1;
            available = match available.checked_sub(min) {
              Some(n) => n,
              None => {
                // We ran out of space. This is a pathological result that we're
                // just going to hope doesn't happen...
                return;
              }
            };
          } else {
            had_change = true;
            w.hint = Hint::Fixed(max);
            w.width = max;
            unboundeds -= 1;
            available = match available.checked_sub(max) {
              Some(n) => n,
              None => {
                // We ran out of space. This is a pathological result that we're
                // just going to hope doesn't happen...
                return;
              }
            };
          }
        }
      }

      if !had_change {
        break;
      }
    }

    // Having found *every* necesarilly fixed widget, we can distribute the
    // remaining space among the remaining unbounded widgets. We give each
    // widget `width_per`, if that much is left; otherwise, we give it the rest
    // of the space and finish there.
    let width_per = div_ceil(available, unboundeds);
    for w in &mut self.widgets {
      if let Hint::Flex(..) = w.hint {
        if width_per > available {
          w.width = available;
          return;
        }
        w.width = width_per;
        available -= width_per;
      }
    }
  }

  /// Draws the widget bar, producing an array of texels that can be blitted
  /// into a scene.
  pub fn draw(&mut self, width: usize) -> &[Texel] {
    if self.buf.len() != width {
      self.buf = vec![Texel::new('\0'); width].into_boxed_slice();
      self.dirty = true;
    }

    if self.dirty {
      self.reflow(width);
      let mut i = 0;
      for w in &self.widgets {
        if w.width == 0 {
          continue;
        }

        let texels = &mut self.buf[i..i + w.width];
        i += w.width;
        w.shape.draw(texels);
      }
      self.dirty = false;
    }

    &self.buf
  }
}

/// A generic widget shape.
///
/// `Shapes` can be used to specify generic widgets, for easy re-use of widget
/// rendering and hinting logic.
pub enum Shape {
  /// A "bar", such as a health bar, which depicts a fraction, a bar, and
  /// possibly a label.
  /// ```text
  /// HP[|||||20/200]
  /// ```
  Bar {
    /// An optional label before the bar.
    label: Option<String>,
    /// The color to fill the "active" portion of the bar with.
    fill_color: Color,
    /// The fraction depicted on the bar.
    value_range: (i32, i32),
    /// The minium and maximum "prefered" sizes for the bar; the bar
    /// may wind up being bigger to fit all of the data.
    width_range: (usize, usize),
  },

  /// A single scalar, such as a floor number or a money amount.
  /// ```text
  /// x: 12
  /// ```
  Scalar {
    /// An optional label before the value.
    label: Option<String>,
    /// The color to use for the value.
    color: Color,
    /// The value itself.
    value: i32,
  },

  /// Fills as much space as possible with the given texel.
  Fill(Texel),

  /// Renders nothing; useful for hiding a widget based on the game state.
  Hidden,
}

impl Shape {
  /// Provides a hint for the layout of this shape.
  fn width_hint(&self) -> Hint {
    match self {
      Self::Bar {
        label,
        value_range: (cur, max),
        width_range,
        ..
      } => {
        let label_len = label.as_ref().map(String::len).unwrap_or(0);
        let cur_len = estimate_num_chars(*cur);
        let max_len = estimate_num_chars(*max);

        // The 3 is for the brackets and the slash.
        let minimum = label_len + cur_len + max_len + 3;

        let min = minimum.max(width_range.0);
        let max = minimum.max(width_range.1);
        Hint::Flex(min, Some(max))
      }
      Self::Scalar { label, value, .. } => {
        let label_len = label.as_ref().map(String::len).unwrap_or(0);
        let int_len = estimate_num_chars(*value);
        Hint::Fixed(label_len + int_len)
      }
      Self::Fill(_) => Hint::Flex(0, None),
      Self::Hidden => Hint::Hidden,
    }
  }

  /// Draws this shape onto `buf`.
  fn draw(&self, mut buf: &mut [Texel]) {
    match self {
      Self::Bar {
        label,
        fill_color: color,
        value_range: (cur, max),
        ..
      } => {
        let label_len = label.as_ref().map(String::len).unwrap_or(0);
        let cur_len = estimate_num_chars(*cur);
        let max_len = estimate_num_chars(*max);

        // 1 below is the slash; 2 is the brackets.
        let bar_nums = cur_len + max_len + 1;
        let minimum = label_len + 2 + bar_nums;
        let extra = buf.len().saturating_sub(minimum);
        let mut filled = (bar_nums + extra) * *cur as usize / *max as usize;

        for c in label.as_ref().map(String::as_str).unwrap_or("").chars() {
          if !push_texel(Texel::new(c), &mut buf) {
            return;
          }
        }

        if !push_texel(Texel::new('['), &mut buf) {
          return;
        }
        for _ in 0..extra {
          let color = if filled > 0 {
            filled -= 1;
            *color
          } else {
            Color::Reset
          };
          if !push_texel(Texel::new('|').with_fg(color), &mut buf) {
            return;
          }
        }
        for c in format!("{}", cur).chars() {
          let color = if filled > 0 {
            filled -= 1;
            *color
          } else {
            Color::Reset
          };
          if !push_texel(Texel::new(c).with_fg(color), &mut buf) {
            return;
          }
        }
        {
          let color = if filled > 0 {
            filled -= 1;
            *color
          } else {
            Color::Reset
          };
          if !push_texel(Texel::new('/').with_fg(color), &mut buf) {
            return;
          }
        }
        for c in format!("{}", max).chars() {
          let color = if filled > 0 {
            filled -= 1;
            *color
          } else {
            Color::Reset
          };
          if !push_texel(Texel::new(c).with_fg(color), &mut buf) {
            return;
          }
        }
        if !push_texel(Texel::new(']'), &mut buf) {
          return;
        }
      }
      Self::Scalar {
        label,
        color,
        value,
      } => {
        for c in label.as_ref().map(String::as_str).unwrap_or("").chars() {
          if !push_texel(Texel::new(c), &mut buf) {
            return;
          }
        }

        let num = format!("{}", value);
        for c in num.chars() {
          if !push_texel(Texel::new(c).with_fg(*color), &mut buf) {
            return;
          }
        }
      }
      Self::Fill(t) => {
        for tx in buf {
          *tx = *t;
        }
      }
      Self::Hidden => {}
    }
  }
}

/// Estimates the number of characters needed to print `num`.
fn estimate_num_chars(mut num: i32) -> usize {
  if num == 0 {
    return 1;
  }
  let mut chars = (num < 0) as usize;
  while num != 0 {
    num /= 10;
    chars += 1;
  }
  return chars;
}

/// Pushes `tx` onto `buf`, returning the remaining part of `buf` and whether
/// the push succeeded.
fn push_texel(tx: Texel, buf: &mut &mut [Texel]) -> bool {
  if buf.is_empty() {
    return false;
  }
  buf[0] = tx;
  let mut tmp = &mut [][..];
  std::mem::swap(buf, &mut tmp);
  tmp = &mut tmp[1..];
  std::mem::swap(buf, &mut tmp);
  return true;
}
