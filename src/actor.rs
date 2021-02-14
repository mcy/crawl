//! Actor components.

use std::collections::HashSet;

use crate::geo::Point;
use crate::gfx::texel::Texel;

/// A "player" actor.
pub struct Player;

/// An actor currently holding the camera's focus.
pub struct HasCamera;

/// An actor with a position.
pub struct Position(pub Point<i64>);

/// A tangile actor (i.e., one with collision).
pub struct Tangible;

/// An actor with a field-of-view.
pub struct Fov {
  /// The radius of the FOV range.
  pub range: Point<i64>,
  /// The set of points that are currently visible.
  pub visible: HashSet<Point<i64>>,
  /// The set of points that have been seen.
  pub seen: HashSet<Point<i64>>,
}

/// An actor with a sprite.
pub struct Sprite(pub Texel);
