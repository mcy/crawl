//! Actor components.

use std::collections::HashSet;

use crate::geo::Point;
use crate::gfx::texel::Texel;

pub mod ai;

/// Component: A "player" actor.
pub struct Player;

/// Component: If this entity has a [`Position`], the renderer camera will focus
/// on it.
pub struct HasCamera;

/// Component: An actor with a position.
pub struct Position(pub Point<i64>);

/// Component: A tangile actor (i.e., one with collision).
pub struct Tangible;

/// Component: An actor with a field-of-view.
pub struct Fov {
  /// The radius of the FOV range.
  pub range: Point<i64>,
  /// The set of points that are currently visible.
  pub visible: HashSet<Point<i64>>,
  /// The set of points that have been seen.
  pub seen: HashSet<Point<i64>>,
}

/// Component: An actor with a sprite.
pub struct Sprite(pub Texel);
