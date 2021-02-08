//! Primitive scene-rendering engine.

use std::mem;

use crate::geo::Point;
use crate::geo::Rect;
use crate::geo::RectVec;
use crate::render::texel::Texel;

pub mod curses;
pub mod texel;

/// A global rendering context.
///
/// This type is used to render scenes, possibly caching them in between frames.
pub struct Renderer {
  baked: RectVec<Texel>,
  scratch: RectVec<Texel>,
}

impl Renderer {
  /// Creates a new `Renderer`.
  pub fn new() -> Self {
    Self {
      baked: RectVec::empty(),
      scratch: RectVec::empty(),
    }
  }

  /// Bakes a scene, rendering it onto the given `window`.
  pub fn bake(&mut self, mut scene: Scene, window: &mut curses::Curses) {
    let (rows, cols) = window.dims();
    let viewport =
      Rect::with_dims(cols as i64, rows as i64).centered_on(scene.camera);

    self.scratch.resize(viewport, scene.void);

    scene.elements.sort_by_key(|e| e.priority);
    for element in scene.elements {
      match element.kind {
        ElementKind::Image(data) => self.draw_image(&data),
        ElementKind::Shader(f) => self.apply_shader(&f),
      }
    }

    for (i, msg) in scene.debug.into_iter().enumerate() {
      if i >= viewport.height() as usize {
        break;
      }
      let chars = msg
        .chars()
        .map(|c| Texel::colored(c, Some(palette::named::RED), None))
        .take(viewport.width() as usize)
        .collect::<Vec<_>>();

      let stride = viewport.width() as usize * i;
      self.scratch.data_mut()[stride..stride + chars.len()]
        .copy_from_slice(&chars);
    }

    self.draw_scene(window);
  }

  /// Draws `data` onto the scratch buffer, taking into mind `data`'s absolute
  /// dimensions.
  fn draw_image(&mut self, data: &RectVec<Texel>) {
    let intersection = match data.dims().intersect(self.scratch.dims()) {
      Some(x) => x,
      None => return,
    };

    for p in intersection.points() {
      let tx = *data.get(p).unwrap();
      if tx.glyph() == '\0' {
        continue;
      }

      *self.scratch.get_mut(p).unwrap() = tx;
    }
  }

  /// Applies `shader` to the scratch buffer.
  ///
  /// TODO(mcyoung): Replace this entire functionality with blitting masks
  /// instead.
  fn apply_shader(
    &mut self,
    shader: &(dyn Fn(Point<i64>, Texel) -> Texel + '_),
  ) {
    for (p, tx) in self.scratch.points_mut() {
      *tx = shader(p, *tx);
    }
  }

  /// Draws the baked scene currently in `self.scratch`.
  fn draw_scene(&mut self, window: &mut curses::Curses) {
    let origin = self.scratch.dims().upper_left();
    let same_area = self.scratch.dims().area() == self.baked.dims().area();

    for (i, (p, new_tx)) in self.scratch.points().enumerate() {
      if same_area && self.baked.data()[i] == *new_tx {
        // TODO(mcyoung): This should be used to intelligently cache which draw
        // calls need to be done to `window` but that seems to not be working
        // quite right yet.
        //
        // continue
      }

      let rel = p - origin;
      window.draw(curses::DrawCall {
        row: rel.y() as usize,
        col: rel.x() as usize,
        texel: *new_tx,
      })
    }

    mem::swap(&mut self.scratch, &mut self.baked);
  }
}

/// An unbaked scene.
///
/// This type can be used for building up a scene to be rendered. The rendering
/// itself is done with the [`Renderer`].
///
/// See [`Renderer::bake()`].
pub struct Scene<'a> {
  /// A list of scene elements to render.
  pub elements: Vec<Element<'a>>,

  /// A list of debug statements to print on top of the rendered scene.
  pub debug: Vec<String>,

  ///
  pub camera: Point,
  ///
  pub void: Texel,
}

/// A scene element.
///
/// Scene elements can just be blocks of raw texels, or they be more complex
/// mappings of existing texels.
pub struct Element<'a> {
  /// The z-priority for this element.
  pub priority: i32,
  /// The kind of element, e.g., rendering strategy, for this element.
  pub kind: ElementKind<'a>,
}

/// A kind of [`Element`].
pub enum ElementKind<'a> {
  /// A plain buffer of texels to draw.
  Image(RectVec<Texel>),

  /// A dynamic texel shader.
  ///
  /// Very slow, probably going away.
  Shader(Box<dyn Fn(Point<i64>, Texel) -> Texel + 'a>),
}
