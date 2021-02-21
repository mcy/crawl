//! Actor components.

use std::collections::HashSet;

use legion::query::component;

use crate::actor::ai::TurnMode;
use crate::geo::Dir;
use crate::geo::Point;
use crate::gfx::texel::Texel;
use crate::input::KeyCode;
use crate::input::KeyModifiers;
use crate::input::UserInput;
use crate::map::Floor;
use crate::map::Tile;
use crate::timing::SystemTimer;

pub mod ai;

/// Component: A "player" actor.
pub struct Player;

#[legion::system(for_each)]
#[write_component(Position)]
#[write_component(Oriented)]
#[filter(component::<Player>())]
pub fn player_movement(
  pos: &mut Position,
  dir: &mut Oriented,
  #[resource] floor: &Floor,
  #[resource] input: &UserInput,
  #[resource] timer: &SystemTimer,
  #[resource] turn_mode: &mut TurnMode,
) {
  let _t = timer.start("actor::player_movement()");

  // Base directions on a Qwerty keyboard. We preserve WASD, which means
  // x and s are swapped in the "naive" interpretation where the center key,
  // s, would be the wait key.
  //
  //  \  |  /
  //   q w e
  // - a s d -
  //   z x c
  //  /  |  \
  fn dir_char(dir: Dir) -> char {
    match dir {
      Dir::N => 'w',
      Dir::W => 'a',
      Dir::S => 's',
      Dir::E => 'd',
      Dir::Nw => 'q',
      Dir::Ne => 'e',
      Dir::Sw => 'z',
      Dir::Se => 'c',
    }
  }

  let shifted = input.has_mod(KeyModifiers::SHIFT);
  for &d in &Dir::all() {
    if input.has_key(KeyCode::Char(dir_char(d))) {
      dir.0 = d;
      if !shifted {
        let new_pos = pos.0 + d.to_point::<i64>();
        match floor.chunk(new_pos).unwrap().tile(new_pos) {
          Tile::Wall | Tile::Void => continue,
          _ => {}
        };
        pos.0 = new_pos;
        *turn_mode = TurnMode::Running;
      }
      // Only select *one* key per step.
      return;
    }
  }

  // Wait is x.
  if !shifted && input.has_key(KeyCode::Char('x')) {
    *turn_mode = TurnMode::Running;
  }
}

/// Component: If this entity has a [`Position`], the renderer camera will focus
/// on it.
pub struct HasCamera;

/// Component: An actor with a position.
pub struct Position(pub Point<i64>);

/// Component: An actor with an orientation.
pub struct Oriented(pub Dir);

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
