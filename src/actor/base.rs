//! Basic actor components.

use crate::geo::Point;
use crate::geo::Dir;
use crate::gfx::texel::Texel;

/// Component: If this entity has a [`Position`], the renderer camera will focus
/// on it.
pub struct HasCamera;

/// Component: An actor with a position.
pub struct Position(pub Point<i64>);

/// Component: An actor with an orientation.
pub struct Oriented(pub Dir);

/// Component: A tangile actor (i.e., one with collision).
pub struct Tangible;

/// Component: An actor with a sprite.
pub struct Sprite(pub Texel);