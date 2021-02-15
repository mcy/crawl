//! Geometry and math library.

use std::mem;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Range;

use num::FromPrimitive;
use num::Integer;
use num::Signed;
use num::ToPrimitive;
use num::Zero;

mod impls;

pub mod fov;
pub mod graph;

/// A direction on the plane.
///
/// We use the following convention for coordinates: x increases to the right
/// direction, and y in the downwards direction.
#[allow(missing_docs)]
pub enum Direction {
  Up,
  Down,
  Left,
  Right,
}

/// A two-dimensional point.
///
/// `Point<T>` values may be added and subtracted componentwise.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Point<T = i64>([T; 2]);

impl<T> Point<T> {
  /// Creates a new `Point` with the given coordinates.
  #[inline]
  pub fn new(x: T, y: T) -> Self {
    Self([x, y])
  }

  /// Creates a new `Point` representing the origin.
  #[inline]
  pub fn zero() -> Self
  where
    T: Zero,
  {
    Zero::zero()
  }

  /// Returns this `Point`'s coordinates as an array.
  #[inline]
  pub fn coords(self) -> [T; 2] {
    self.0
  }

  /// Returns the `x` coordinate.
  #[inline]
  pub fn x(self) -> T
  where
    T: Copy,
  {
    self.0[0]
  }

  /// Returns the `y` coordinate.
  #[inline]
  pub fn y(self) -> T
  where
    T: Copy,
  {
    self.0[1]
  }

  /// Computes the dot product of `self` and `other`.
  pub fn dot<U>(self, other: Point<U>) -> <T::Output as Add>::Output
  where
    T: Mul<U> + Copy,
    U: Copy,
    T::Output: Zero,
  {
    let mut total = Zero::zero();
    for i in 0..self.len() {
      total = total + self[i] * other[i];
    }
    total
  }

  /// Computes the Manhattan norm of `self`.
  pub fn manhattan(self) -> T
  where
    T: Signed + Copy,
  {
    self.x().abs() + self.y().abs()
  }

  /// Componentwise orders the coordinates of `self` and `other`.
  ///
  /// Returns a pair of points whose coordinates are the minima and maxima in
  /// each coordinate, respectively.
  #[inline]
  pub fn sort_coords(mut self, mut other: Self) -> (Self, Self)
  where
    T: PartialOrd,
  {
    for i in 0..self.len() {
      if self[i] > other[i] {
        mem::swap(&mut self[i], &mut other[i])
      }
    }
    (self, other)
  }

  /// Returns whether the Euclidean norm of `self` is at most `z`.
  pub fn norm_at_most(self, z: T) -> bool
  where
    T: Zero + Mul<Output = T> + PartialOrd + Copy,
  {
    let dot = self.dot(self);

    dot <= z * z
  }
}

/// A rectangle, represented as a pair of [`Point`] values.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Rect<T = i64>(Point<T>, Point<T>);

// Invariant: rect.0.x <= rect.1.x and rect.0.y <= rect.1.y.
impl<T: Signed> Rect<T> {
  /// Creates a new `Rect` with the given [`Point`] values as opposing corners.
  #[inline]
  pub fn new(p1: Point<T>, p2: Point<T>) -> Self
  where
    T: PartialOrd,
  {
    let (min, max) = Point::sort_coords(p1, p2);
    Self(min, max)
  }

  /// Creates a new `Rect` of the given dimensions with one corner at the
  /// origin.
  #[inline]
  pub fn with_dims(width: T, height: T) -> Self {
    Self(Point::zero(), Point::new(width.abs(), height.abs()))
  }

  /// Returns the upper-left corner of this `Rect`.
  #[inline]
  pub fn upper_left(self) -> Point<T> {
    self.0
  }

  /// Returns the upper-right corner of this `Rect`.
  #[inline]
  pub fn lower_right(self) -> Point<T> {
    self.1
  }

  /// Returns the upper-left and lower-right corners of this `Rect`.
  #[inline]
  pub fn corners(self) -> (Point<T>, Point<T>) {
    (self.0, self.1)
  }

  /// Returns the width of this `Rect`.
  #[inline]
  pub fn width(self) -> T
  where
    T: Copy,
  {
    self.1.x() - self.0.x()
  }

  /// Returns the height of this `Rect`.
  #[inline]
  pub fn height(self) -> T
  where
    T: Copy,
  {
    self.1.y() - self.0.y()
  }

  /// Returns the area of this `Rect`.
  #[inline]
  pub fn area(self) -> T
  where
    T: Copy,
  {
    self.width() * self.height()
  }

  /// Returns the center of this `Rect`.
  #[inline]
  pub fn center(self) -> Point<T>
  where
    T: Copy + FromPrimitive,
  {
    self.0 / T::from_u64(2).unwrap() + self.1 / T::from_u64(2).unwrap()
  }

  /// Returns whether this `Rect` is empty.
  ///
  /// A `Rect` is considered empty if its area is non-positive, i.e., less than
  /// or equal to zero.
  #[inline]
  pub fn is_empty(self) -> bool
  where
    T: Copy,
  {
    !self.area().is_positive()
  }

  /// Returns whether this `Rect` contains a given point.
  ///
  /// Note that the points in a rectangle form an "exclusive" range; points
  /// colinear with the lower-left corner are *not* part of the rectangle.
  #[inline]
  pub fn contains(self, p: Point<T>) -> bool
  where
    T: Copy + PartialOrd,
  {
    for i in 0..p.len() {
      if !(self.0[i]..self.1[i]).contains(&p[i]) {
        return false;
      }
    }
    true
  }

  /// Returns whether this `Rect`'s boundary contains a given point.
  ///
  /// Note that the points in a rectangle form an "exclusive" range; points
  /// colinear with the lower-left corner are *not* part of the rectangle; thus,
  /// the boundary is shifted one unit up and to the left from it.
  #[inline]
  pub fn boundary_contains(self, p: Point<T>) -> bool
  where
    T: Copy + PartialOrd,
  {
    let on_edge = p.x() == self.0.x()
      || p.x() == self.1.x().sub(T::one())
      || p.y() == self.0.y()
      || p.y() == self.1.y().sub(T::one());

    on_edge && self.contains(p)
  }

  /// Translates this `Rect` such that its center is (approximately) at
  /// `center`.
  pub fn centered_on(self, center: Point<T>) -> Self
  where
    T: FromPrimitive + Copy,
  {
    Self::with_dims(self.width(), self.height())
      - Point::new(
        self.width() / T::from_u64(2).unwrap(),
        self.height() / T::from_u64(2).unwrap(),
      )
      + center
  }

  /// Computes the intersection of this `Rect` with `other`.
  ///
  /// Returns `None` if they do not intersect at all.
  pub fn intersect(self, other: Rect<T>) -> Option<Rect<T>>
  where
    T: PartialOrd,
  {
    let (_, p1) = Point::sort_coords(self.0, other.0);
    let (p2, _) = Point::sort_coords(self.1, other.1);

    if p1[0] >= p2[0] || p1[1] >= p2[1] {
      return None;
    }

    Some(Rect(p1, p2))
  }

  /// Returns an iterator over all points in this rectangle.
  ///
  /// Points are traversed in row-major order.
  pub fn points(self) -> impl Iterator<Item = Point<T>>
  where
    T: Copy,
    Range<T>: Iterator<Item = T>,
  {
    let [x1, y1] = self.0.coords();
    let [x2, y2] = self.1.coords();

    (y1..y2)
      .map(move |y| (x1..x2).map(move |x| Point::new(x, y)))
      .flatten()
  }

  /// Returns an iterator over all points in the boundary of this rectangle.
  ///
  /// The boundary is defined
  pub fn boundary(self) -> impl Iterator<Item = Point<T>>
  where
    T: Copy,
    Range<T>: Iterator<Item = T>,
  {
    let [x1, y1] = self.0.coords();
    let [x2, y2] = self.1.coords();

    (x1..x2)
      .map(move |x| Point::new(x, y1))
      .chain(
        (y1.add(T::one())..y2.sub(T::one()))
          .map(move |y| {
            let mut count = 0;
            std::iter::from_fn(move || match count {
              0 => {
                count += 1;
                Some(Point::new(x1, y))
              }
              1 => {
                count += 1;
                Some(Point::new(x2.sub(T::one()), y))
              }
              _ => None,
            })
          })
          .flatten(),
      )
      .chain((x1..x2).map(move |x| Point::new(x, y2.sub(T::one()))))
  }

  /// Returns an iterator over the intersection of `self` with a rectangular
  /// tiling of the plane.
  ///
  /// `tile` is a single cell of the tiling from which the intersections are
  /// to be computed.
  ///
  /// Rectangles are traversed in row-major order.
  ///
  /// # Panics
  ///
  /// Panics if `tile` has zero area.
  pub fn disect(self, tile: Rect<T>) -> impl Iterator<Item = Rect<T>>
  where
    T: Copy + Integer + Signed + ToPrimitive + PartialOrd,
    Range<T>: Iterator<Item = T>,
  {
    assert!(!tile.is_empty());

    // This problem is easier to solve when `tile`'s UL corner is zero, so we'll
    // do everything with respect to that.
    let (tile_start, tile_end) = tile.corners();
    let z_self = self - tile_start;
    let tile_dims = tile_end - tile_start;

    // Now, we need to round `z_self`'s coordinates to be multiples of
    // `tile_dims`. This rounding should occur towards negative infinity.
    let (start, end) = z_self.corners();
    let start = Point::new(
      T::div_floor(&start.x(), &tile_dims.x()) * tile_dims.x(),
      T::div_floor(&start.y(), &tile_dims.y()) * tile_dims.y(),
    );

    (start.y()..end.y())
      .step_by(tile_dims.y().to_usize().unwrap())
      .map(move |y| {
        (start.x()..end.x())
          .step_by(tile_dims.x().to_usize().unwrap())
          .map(move |x| {
            let tile_coords = Point::new(x, y);
            let tile = Self::new(tile_coords, tile_coords + tile_dims);
            self.intersect(tile).unwrap() + tile_start
          })
      })
      .flatten()
  }
}

/// A rectangle with associated data at each point.
// Invariant: self.1.len() == self.0.area()
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct RectVec<T>(Rect<i64>, Box<[T]>);

impl<T: Clone> RectVec<T> {
  /// Creates a new, empty `RectVec` with arbitrary degenerate coordinates.
  pub fn empty() -> Self {
    RectVec(Rect::with_dims(0, 0), Vec::new().into_boxed_slice())
  }

  /// Creates a new `RectVec` with the requested dimensions and filled with the
  /// given value.
  pub fn new(rect: Rect<i64>, val: T) -> Self {
    RectVec(rect, vec![val; rect.area() as usize].into_boxed_slice())
  }

  /// Returns this `RectVec`'s dimensions.
  pub fn dims(&self) -> Rect<i64> {
    self.0
  }

  /// Returns this `RectVec`'s data as a linear slice.
  pub fn data(&self) -> &[T] {
    &self.1
  }

  /// Returns this `RectVec`'s data as a mutable linear slice.
  pub fn data_mut(&mut self) -> &mut [T] {
    &mut self.1
  }

  /// Transforms this `RectVec`'s dimensions to the new rectangle, filling it
  /// with `val` in the process.
  pub fn resize(&mut self, new_rect: Rect<i64>, val: T) {
    if self.0.area() == new_rect.area() {
      self.0 = new_rect;
      for x in self.1.iter_mut() {
        *x = val.clone();
      }
    } else {
      *self = Self::new(new_rect, val);
    }
  }

  /// Gets a reference to the data value associated with `p`.
  ///
  /// Returns `None` if `p` is out-of-bounds.
  pub fn get(&self, p: Point<i64>) -> Option<&T> {
    if !self.dims().contains(p) {
      return None;
    }
    let origin = self.dims().upper_left();
    let rel = p - origin;
    let index = rel.x() + rel.y() * self.dims().width();
    self.1.get(index as usize)
  }

  /// Gets a mutable reference to the data value associated with `p`.
  ///
  /// Returns `None` if `p` is out-of-bounds.
  pub fn get_mut(&mut self, p: Point<i64>) -> Option<&mut T> {
    if !self.dims().contains(p) {
      return None;
    }
    let origin = self.dims().upper_left();
    let rel = p - origin;
    let index = rel.x() + rel.y() * self.dims().width();
    self.1.get_mut(index as usize)
  }

  /// Returns an iterator over the points of this `RectVec` and their associated
  /// values.
  pub fn points(&self) -> impl Iterator<Item = (Point<i64>, &T)> + '_ {
    let dims = self.dims();
    dims.points().enumerate().map(move |(i, p)| (p, &self.1[i]))
  }

  /// Returns an iterator over the points of this `RectVec` and their associated
  /// values.
  pub fn points_mut(
    &mut self,
  ) -> impl Iterator<Item = (Point<i64>, &mut T)> + '_ {
    let dims = self.dims();
    let ptr = self.1.as_mut_ptr();
    // SAFETY: Since this iterator only ever returns disjoint references into
    // `ptr`, and borrows `self`, this is safe (by analogy with e.g.
    // slice::IterMut).
    dims
      .points()
      .enumerate()
      .map(move |(i, p)| (p, unsafe { &mut *ptr.add(i) }))
  }
}
