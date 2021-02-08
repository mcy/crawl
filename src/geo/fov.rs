//! Field-of-view algorithms.

use std::cmp::Ordering;

use crate::geo::Point;

/// Compute the field-of-view from a given point using Milazzo's algorithm.
///
/// The FoV is computed relative to an observer at `origin` with a view range
/// given by `range` (which gives the semi-major axes of an ellipse)
///
/// `is_opaque` returns `true` if a point represents an obstruction
/// (i.e. an opaque tile). `ignite` will be called on all points in the FoV.
///   
/// See http://www.adammil.net/blog/v125_Roguelike_Vision_Algorithms.html#mycode
pub fn milazzo(
  origin: Point<i64>,
  range: Point<i64>,
  is_opaque: &mut dyn FnMut(Point<i64>) -> bool,
  ignite: &mut dyn FnMut(Point<i64>),
) {
  /// A slope in the plane, represented as a rational number y/x.
  ///
  /// This is used instead of `Rational64` for conciseness and performance
  /// (since `Rational64` performs GCD reduction).
  #[derive(Copy, Clone)]
  struct Slope {
    x: i64,
    y: i64,
  }

  impl PartialEq for Slope {
    fn eq(&self, other: &Self) -> bool {
      self.y * other.x == other.y * self.x
    }
  }
  impl PartialOrd for Slope {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      (self.y * other.x).partial_cmp(&(other.y * self.x))
    }
  }

  impl PartialEq<Point<i64>> for Slope {
    fn eq(&self, other: &Point<i64>) -> bool {
      self
        == &Slope {
          x: other.x(),
          y: other.y(),
        }
    }
  }
  impl PartialOrd<Point<i64>> for Slope {
    fn partial_cmp(&self, other: &Point<i64>) -> Option<Ordering> {
      self.partial_cmp(&Slope {
        x: other.x(),
        y: other.y(),
      })
    }
  }

  // Tile geometry helpers.
  //
  // A tile's coordinates refer to its center; the following diagram shows
  // different parts of a tile; this type can be used to quickly compute them.
  // ```text
  //     g
  //  a------b    a: top_left        i: inner_top_right
  //  |  /\  |    b: top_right       j: inner_top_left
  //  |i/__\j|    c: bottom_left     k: inner_bottome_left
  // e|/|  |\|f   d: bottom_right    m: immer_bottom_right
  //  |\|__|/|
  //  |k\  /m|    e: middle_left
  //  |  \/  |    f: middle_right    +--> x
  //  c------d    g: middle_top      |
  //     h        h: middle_bottom   v y
  // ```
  #[derive(Copy, Clone)]
  struct Tile(Point<i64>);

  macro_rules! tile_fns {
    ($($ident:ident: $expr:expr,)*) => {
      #[allow(unused)]
      impl Tile {$(
        fn $ident(self) -> Point<i64> {
          $expr(self.0)
        }
      )*}
    }
  }

  tile_fns! {
    top_left:     |p| p * 2 + Point::new(-1, 1),
    top_right:    |p| p * 2 + Point::new(1, 1),
    bottom_left:  |p| p * 2 + Point::new(-1, -1),
    bottom_right: |p| p * 2 + Point::new(1, -1),

    middle_left:   |p| p * 2 + Point::new(-1, 0),
    middle_right:  |p| p * 2 + Point::new(1, 0),
    middle_top:    |p| p * 2 + Point::new(0, 1),
    middle_bottom: |p| p * 2 + Point::new(0, -1),

    inner_top_left:     |p| p * 4 + Point::new(-1, 1),
    inner_top_right:    |p| p * 4 + Point::new(1, 1),
    inner_bottom_left:  |p| p * 4 + Point::new(-1, -1),
    inner_bottom_right: |p| p * 4 + Point::new(1, -1),
  }

  /// State for the algorithm that is not tracked by recursion frames.
  struct State<Opaque, Ignite> {
    origin: Point<i64>,
    range: Point<i64>,
    is_opaque: Opaque,
    ignite: Ignite,
    octant: u8,
  }

  ignite(origin);
  #[rustfmt::skip]
  let mut state = State { origin, range, is_opaque, ignite, octant: 0 };

  for octant in 0..8 {
    state.octant = octant;
    state.recurse(1, Slope { y: 1, x: 1 }, Slope { y: 0, x: 1 });
  }

  impl<O, I> State<O, I>
  where
    O: FnMut(Point<i64>) -> bool,
    I: FnMut(Point<i64>),
  {
    /// Transform octant coordinates into map coordinates.
    ///
    /// The algrithm breaks up the plane (with respect to the origin) into eight
    /// octants. The core recursive algorithm operates on octant coordinates
    /// (x, y) which are such that `y <= x`. The octants (in the convention
    /// where right and down are positive x and y) look like this:
    ///
    /// ```text
    /// \ 2|1 /
    ///  \ | /
    /// 3 \|/ 0
    /// ---o---   +-> x
    /// 4 /|\ 7   |
    ///  / | \    v y
    /// / 5|6 \
    /// ```
    ///
    /// The assignment of numbers to octants is irrelevant, other than that
    /// they be consistent.
    ///
    /// Note that octants 1, 2, 5, and 6 swap the coordinates, which is relevant
    /// for computing the range ellipse
    fn oct2map(&self, p: Point<i64>) -> Point<i64> {
      let [px, py] = p.coords();
      let py = -py; // Y is flipped in the map coordinate system.
      let [ox, oy] = self.origin.coords();
      match self.octant {
        0 => Point::new(ox + px, oy - py),
        1 => Point::new(ox + py, oy - px),
        2 => Point::new(ox - py, oy - px),
        3 => Point::new(ox - px, oy - py),
        4 => Point::new(ox - px, oy + py),
        5 => Point::new(ox - py, oy + px),
        6 => Point::new(ox + py, oy + px),
        _ => Point::new(ox + px, oy + py),
      }
    }

    #[inline(always)]
    fn is_opaque(&mut self, p: Point<i64>) -> bool {
      let p = self.oct2map(p);
      (self.is_opaque)(p)
    }

    #[inline(always)]
    fn ignite(&mut self, p: Point<i64>) {
      let p = self.oct2map(p);
      (self.ignite)(p)
    }

    /// Performs one recursion of Milazzo's algorithm.
    fn recurse(
      &mut self,
      x_start: i64,
      mut sector_top: Slope,
      mut sector_bottom: Slope,
    ) {
      let range = if [1, 2, 5, 6].contains(&self.octant) {
        Point::new(self.range.y(), self.range.x())
      } else {
        self.range
      };

      for x in x_start..=range.x() {
        // Compute the y coordinates of this columnd of the sector.

        let sector_top_y = match sector_top.x {
          // If top == d/1, then d = 1 (since slopes are <= 1), so y = x. This
          // is such a common case (since it occurs as long as the upper vector
          // doesn't hit a wall) that we special case it for efficiency.d
          1 => x,
          _ => {
            // Get the tile that the top vector enters from the left. This is
            // will be the tile marked by a *:
            //
            //       /
            //      *
            //     /^
            //    / |
            //   o------->
            //      ^
            //      x
            //
            // which we can find by multiplying the slope by `x`. We offset by
            // 1/2, since we want the coordinates of the left entry point.
            let Slope { x: sx, y: sy } = sector_top;
            let y = ((x * 2 - 1) * sy + sx) / (sx * 2);
            let p = Point::new(x, y);

            // If the vector passes from the left side up into the tile above,
            // before exiting the column, so we may need to increment y.
            if self.is_opaque(p) {
              // If the tile entered blocks light, whether it passes into the
              // tile above depends on the shape of the wall tile:
              //
              // - If the top-left corner is beveled, then it is blocked; the
              //   corner is not vebeled if the tiles above and to the left
              //   are not opaque. However, the left tile must have been
              //   transparent for the vector to hit, so we need only check
              //   the top.
              //
              // Thus, if the corner is beveled, the slope of the vector must
              // be greater than the slope of the top-left corner of the wall,
              // i.e., the slope from the middle_top to the origin.
              if sector_top >= Tile(p).middle_top()
                && self.is_opaque(p + Point::new(0, 1))
              {
                y + 1
              } else {
                y
              }
            } else {
              // If it doesn't block light, then it passes to the tile above.
              // The vector does so if it has a greater slope than that at the
              // bottom-right corner of the tile above.
              //
              // However, later code assumes that opaque tiles are visible
              // (since we want rays stopped by walls to illuminate them), so
              // there are three cases:
              // 1) The tile above is transparent. The vector must be above the
              //    bottom-right corner of the tile above.
              // 2) The tile above is opaque, and does not have a beveled
              //    bottom-right corner. The vector must again be above the
              //    bottom-right corner.
              // 3) The tile above is opaque and has a beveled bottom-right
              //    corner. The vector must be above the bottom-center of the
              //    tile above.
              //
              // We merge cases (1) and (2) into a single check. If the tile
              // above and to the right is opaque, then the vector must be above
              // the bottom-right corner. Otherwise, it must be above the
              // bottom-center. This works because if the tile above and to the
              // right is opaque, then we have two cases:
              // 1) The tile above is opaque, in which case we must check the
              //    bottom-right corner.
              // 2) The tile above is transparent, so the vector passes into it
              //    if it is above the bottom-right corner.
              //
              // If the tile above and to the right is transparent, we have two
              // further cases:
              // 1) The tile above is opaque with a beveled edge, in which case
              //    we must check against the bottom center.
              // 2) The tile above is transparent, in which case it is only
              //    visible if light strikes the inner square, which is
              //    guaranteed to fit in a wall diamond. If it would not strike,
              //    it would not strike the wall diamond, so we don't bother
              //    incrementing if light passes through the corner of the tile
              //    above.
              //
              // Thus, we use the bottom corner for both cases.
              //
              // TODO(mcyoung): elaborate with diagrams.
              let slope = if self.is_opaque(p + Point::new(1, 1)) {
                Tile(p).top_right()
              } else {
                Tile(p).middle_top()
              };

              if sector_top > slope {
                y + 1
              } else {
                y
              }
            }
          }
        };

        let sector_bottom_y = match sector_bottom.y {
          // If bottom == 0/d == 0, then it's the x axis, so it's hitting the
          // tile at y = 0 dead center. Much like the top == d/1 case, this is
          // the common initial condition.
          0 => 0,
          _ => {
            // We use the same formula as above for the point hit by the vector.
            let Slope { x: sx, y: sy } = sector_bottom;
            let y = ((x * 2 - 1) * sy + sx) / (sx * 2);
            let p = Point::new(x, y);

            // As above, we assume that if a tile is opaque then it is visible.
            //
            // If the tile is opaque, we must ensure that the bottom vector
            // actually hits the wall shape. It misses if the top-left corner
            // is beveled and if the bottom vector above the top center; the
            // corner is beveled if the tiles to the left and right are clear.
            //
            // As before, we can safely assume the tile to the left is clear.
            if sector_bottom >= Tile(p).middle_bottom()
              && self.is_opaque(p)
              && !self.is_opaque(p + Point::new(0, 1))
            {
              y + 1
            } else {
              y
            }
          }
        };

        // Now, we walk through the column, lighting them as necessary. If we
        // hit obstructions, we may need to recurse.
        let mut was_opaque = None;
        for y in (sector_bottom_y..=sector_top_y).rev() {
          let p = Point::new(x, y);

          // Given semi-major axes a and b, the formula for an ellipse is
          // x^2 / a^2 + y^2 / b^2 < 1
          //
          // This earanges into:
          // b^2 x^2 + a^2 y^2 < (ab)^2
          let [a, b] = range.coords();
          let norm = x * x * b * b + y * y * a * a;
          if norm >= a * a * b * b {
            continue;
          }

          let is_opaque = self.is_opaque(p);
          // Every tile in the range of sector_bottom_y+1..sector_top_y is
          // guaranteed to be visible. As noted above, we assume that a tile
          // is visible if it is opaque, so that we can illuminate walls.
          //
          // Thus, we only do extra work for y at the endpoints, when the tile
          // is transparent.
          // 1) If y == top_y, then we want the top vector above the
          //    bottom-right inner corner.
          // 2) If y == bottom_y, the same logic applies for the top-right
          // inner corner.
          let t = Tile(p);
          let is_visible = is_opaque
            || ((y != sector_top_y || sector_top > t.inner_bottom_right())
              && (y != sector_bottom_y || sector_bottom < t.inner_top_left()));
          if is_visible {
            self.ignite(p);
          }

          if x == self.range.x() {
            continue;
          }

          // Recurse if a sector is split by an obstruction. The top sector is
          // recursed into, whereas the bottom sector is just handled directly
          // in this function, potentially recursing further as we hit more
          // obstacles.
          //
          // Note that iteration is from the top of the sector down.
          if is_opaque && was_opaque == Some(false) {
            // If we transitioned from transparent to opaque, then we're done
            // with this column, so we adjust the bottom vector up and and
            // continue processing.
            //
            // If the opaque tile has a beveled top-left corner, so we move the
            // bottom vector to the top left; this is the case if the tiles up
            // and to the left are clear, but we can assume the left tile has
            // already been visited.
            let [nx, ny] = if self.is_opaque(p + Point::new(0, 1)) {
              t.top_left()
            } else {
              t.middle_top()
            }
            .coords();
            let slope = Slope { y: ny, x: nx };

            // We can only adjust if the new slope would still be below the
            // top of the sector.
            if sector_top > slope {
              // If this is the last tile in the column, we don't bother
              // recursing, since nothing more would have been illuminated
              // anyway.
              if y == sector_bottom_y {
                sector_bottom = slope;
                break;
              } else {
                // Recurse to fill out the top of the split sector.
                self.recurse(x + 1, sector_top, slope);
              }
            } else {
              // The new sector is empty. Nothing to do!

              // If this is the bottom, we can just ignore it and mvoec on.
              if y == sector_bottom_y {
                return;
              }
            }
          } else if !is_opaque && was_opaque == Some(true) {
            // If we're transitioning from opaque to transparent, we need to
            // adjust the top vector downwards. Analogous logic applies here
            // to the section above.
            let [nx, ny] = if self.is_opaque(p + Point::new(1, 1)) {
              t.top_right()
            } else {
              t.middle_top()
            }
            .coords();
            let slope = Slope { y: ny, x: nx };

            // We maintain the invariant that top > bottom. If this does not
            // hold the sector is empty and we're don.e
            if slope > sector_bottom {
              sector_top = slope;
            } else {
              return;
            }
          }

          was_opaque = Some(is_opaque);
        }

        // If the column does not end in a clear tile, then there's no reason to
        // keep going. There are two cases:
        // 1) was_opaque == None, implying that the sector is degenerate.
        // 2) was_opaque == Some(true), implying that we found a transition from
        //    clear to opaque and never went back, so there's nothing to do that
        //    the recursive call didn't already do.
        if was_opaque != Some(false) {
          break;
        }
      }
    }
  }
}
