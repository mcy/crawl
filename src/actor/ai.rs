//! Actor AI components and systems.

use rand::seq::SliceRandom as _;

use crate::actor::Position;
use crate::geo::graph;
use crate::geo::Point;
use crate::map::Floor;
use crate::map::Tile;
use crate::timing::SystemTimer;

/// Describes the current state of the AI turn.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TurnMode {
  /// Indicates that we're waiting for player input, so no AI should execute.
  Waiting,

  /// Indicates that the player has moved, so AI may take one move forward.
  Running,
}

/// System: Ends an AI turn at the end of a frame.
#[legion::system]
pub fn end_turn(#[resource] mode: &mut TurnMode) {
  *mode = TurnMode::Waiting;
}

/// Component: An actor which can pathfind to a goal.
///
/// When the [`pathfind()`] system is installed, every `Pathfind` entity with a
/// [`Postition`] will A* to its goal point. The path will only be recalculated
/// if the entity encounters a barrier in the way.
pub struct Pathfind {
  pub goal: Point,
  pub path: Vec<Point>,
}

impl Pathfind {
  /// Creates a new `Pathfind`.
  pub fn new() -> Self {
    Pathfind {
      goal: Point::zero(),
      path: Vec::new(),
    }
  }

  /// Instructs the AI to walk to a random room in `floor`.
  pub fn random_walk(&mut self, floor: &Floor) {
    if let Some(room) = floor.rooms().choose(&mut rand::thread_rng()) {
      self.goal = room.center();
    }
  }

  /// Recomputes the path towards this `Pathfind`'s goal.
  pub fn repath(&mut self, current: Point, floor: &Floor) {
    self.path = graph::manhattan_a_star(current, self.goal, |p| {
      floor
        .chunk(p)
        .map(|c| *c.tile(p) == Tile::Ground)
        .unwrap_or(false)
    })
    .unwrap_or(Vec::new());
  }

  /// Computes the next point that the entity should walk to, if one is
  /// available.
  pub fn next_pos(&mut self, current: Point, floor: &Floor) -> Option<Point> {
    if self.goal == current {
      return None;
    }

    // Check that the cached path is valid, which is given by our current
    // position being the last element. If it isn't, we re-path.
    if Some(&current) != self.path.last() {
      self.repath(current, floor);
    }

    self.path.pop();
    self.path.last().cloned()
  }
}

/// System: Steps forward the AI for each every [`Pathfind`] entity.
#[legion::system(for_each)]
#[write_component(Position)]
#[write_component(Pathfind)]
pub fn pathfind(
  pos: &mut Position,
  pf: &mut Pathfind,
  #[resource] floor: &Floor,
  #[resource] mode: &TurnMode,
  #[resource] timer: &SystemTimer,
) {
  let _t = timer.start("actor::ai::pathfind()");
  if *mode != TurnMode::Running {
    return;
  }

  match pf.next_pos(pos.0, floor) {
    Some(p) => {
      let is_walkable = floor
        .chunk(p)
        .map(|c| *c.tile(p) == Tile::Ground)
        .unwrap_or(false);
      if is_walkable {
        pos.0 = p;
      } else {
        pf.repath(pos.0, floor);
      }
    }
    None => pf.random_walk(floor),
  }
}
