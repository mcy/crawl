//! Scene rendering.

use std::mem;

use crate::geo::RectVec;
use crate::gfx::curses::Curses;
use crate::gfx::scene::Layer;
use crate::gfx::texel;
use crate::gfx::texel::Texel;
use crate::gfx::Scene;

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
  pub fn bake(&mut self, mut scene: Scene, window: &Curses) {
    let viewport = scene.viewport();
    self.scratch.resize(viewport, Texel::new('?'));

    scene.layers.sort_by_key(|(p, _)| *p);
    for (_, layer) in scene.layers {
      match layer {
        Layer::Image(images) => {
          for data in images {
            let intersection = match data.dims().intersect(self.scratch.dims())
            {
              Some(x) => x,
              None => continue,
            };

            for p in intersection.points() {
              let new = data.get(p).unwrap();
              let old = self.scratch.get_mut(p).unwrap();
              *old = old.add_layer(*new);
            }
          }
        }
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
  fn draw_scene(&mut self, window: &Curses) {
    let origin = self.scratch.dims().upper_left();
    let same_area = self.scratch.dims().area() == self.baked.dims().area();

    let mut session = window.draw_session();
    for (i, (p, new_tx)) in self.scratch.points().enumerate() {
      if same_area && self.baked.data()[i] == *new_tx {
        // TODO(mcyoung): This should be used to intelligently cache which draw
        // calls need to be done to `window` but that seems to not be working
        // quite right yet.
        //
        // continue
      }

      let rel = p - origin;
      session.draw((rel.y() as usize, rel.x() as usize), *new_tx);
    }

    mem::swap(&mut self.scratch, &mut self.baked);
  }
}
