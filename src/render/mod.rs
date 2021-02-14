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
  pub fn bake(&mut self, mut scene: Scene, window: &curses::Curses) {
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
  fn draw_scene(&mut self, window: &curses::Curses) {
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

/// An unbaked scene.
///
/// This type can be used for building up a scene to be rendered. The rendering
/// itself is done with the [`Renderer`].
///
/// See [`Renderer::bake()`].
#[derive(Clone, Debug)]
pub struct Scene {
  layers: Vec<(i32, Layer)>,
  debug: Vec<String>,
  camera: Point,
  viewport: Rect,
  debug_mode: bool,
}

#[derive(Clone, Debug)]
enum Layer {
  Image(Vec<RectVec<Texel>>),
}

impl Scene {
  /// Creates a new `Scene` centered on `camera`.
  ///
  /// If `debug_mode` is false, debug strings will not be rendered in this
  /// scene.
  pub fn new(camera: Point, debug_mode: bool) -> Self {
    let (rows, cols) = curses::dims();
    let viewport =
      Rect::with_dims(cols as i64, rows as i64).centered_on(camera);
    Self {
      layers: Vec::new(),
      debug: Vec::new(),
      camera,
      viewport,
      debug_mode,
    }
  }

  /// Returns the location of this `Scene`'s camera.
  pub fn camera(&self) -> Point {
    self.camera
  }

  /// Returns this `Scene`'s viewport, in game coordinates.
  pub fn viewport(&self) -> Rect {
    self.viewport
  }

  /// Returns an RAII builder for adding a new image layer to this scene.
  ///
  /// The layer will have the given z-priority.
  pub fn image_layer(&mut self, priority: i32) -> ImageLayer<'_> {
    ImageLayer {
      scene: self,
      priority,
      images: Vec::new(),
    }
  }

  /// Adds debug information to this scene, which is rendered on top of all
  /// elements.
  pub fn debug(&mut self, data: String) {
    if self.debug_mode {
      self.debug.push(data)
    }
  }
}

/// A scene layer consisting of various images.
///
/// This type can be used to build an image layer in a [`Scene`]; once the layer
/// is complete, call [`finish()`] or drop this value, and it will get added to
/// the scene.
pub struct ImageLayer<'sc> {
  scene: &'sc mut Scene,
  priority: i32,
  images: Vec<RectVec<Texel>>,
}

impl ImageLayer<'_> {
  /// Returns the [`Scene`] associated with this layer.
  pub fn scene(&self) -> &Scene {
    self.scene
  }

  /// Adds a new image to this layer.
  pub fn push(&mut self, image: RectVec<Texel>) {
    self.images.push(image)
  }

  /// Finishes building this layer, and adds it to the owning [`Scene`].
  pub fn finish(self) {}
}

impl Drop for ImageLayer<'_> {
  fn drop(&mut self) {
    self
      .scene
      .layers
      .push((self.priority, Layer::Image(mem::take(&mut self.images))))
  }
}
