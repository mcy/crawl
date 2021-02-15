//! Scene-building.
//!
//! A [`Scene`] represents all the data necessary to build and render an
//! ASCII-art frame of the game.

use std::mem;

use crate::geo::Point;
use crate::geo::Rect;
use crate::geo::RectVec;
use crate::gfx::curses;
use crate::gfx::texel::Texel;

/// An unbaked scene.
///
/// This type can be used for building up a scene to be rendered. The rendering
/// itself is done with the [`gfx::Renderer`].
///
/// See [`gfx::Renderer::bake()`].
#[derive(Clone, Debug)]
pub struct Scene {
  pub(in crate::gfx) layers: Vec<(i32, Layer)>,
  pub(in crate::gfx) debug: Vec<String>,
  camera: Point,
  pub(in crate::gfx) viewport: Rect,
  debug_mode: bool,
}

#[derive(Clone, Debug)]
pub(in crate::gfx) enum Layer {
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
