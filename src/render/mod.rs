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
    for Element { data, .. } in scene.elements {
      let intersection = match data.dims().intersect(self.scratch.dims()) {
        Some(x) => x,
        None => continue,
      };

      for p in intersection.points() {
        let new = data.get(p).unwrap();
        let old = self.scratch.get_mut(p).unwrap();
        *old = old.add_layer(*new);
      }
    }

    for (i, msg) in scene.debug.into_iter().enumerate() {
      if i >= viewport.height() as usize {
        break;
      }
      let chars = msg
        .chars()
        .map(|c| Texel::new(c).with_fg(texel::colors::RED))
        .take(viewport.width() as usize)
        .collect::<Vec<_>>();

      let stride = viewport.width() as usize * i;
      self.scratch.data_mut()[stride..stride + chars.len()]
        .copy_from_slice(&chars);
    }

    self.draw_scene(window);
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
pub struct Scene {
  /// A list of scene elements to render.
  pub elements: Vec<Element>,

  /// A list of debug statements to print on top of the rendered scene.
  pub debug: Vec<String>,

  ///
  pub camera: Point,
  ///
  pub void: Texel,
}

/// A scene element.
///
/// Scene elements are essentially just blocks of raw texels.
pub struct Element {
  /// The z-priority for this element.
  pub priority: i32,
  ///
  pub data: RectVec<Texel>,
}
