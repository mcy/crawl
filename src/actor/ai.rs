//! Actor AI components and systems.

use std::collections::HashSet;

use rand::seq::IteratorRandom as _;
use rand::seq::SliceRandom as _;

use legion::query::component;
use legion::query::IntoQuery;
use legion::world::SubWorld;
use legion::Entity;

use crate::actor::Fov;
use crate::actor::Player;
use crate::actor::Position;
use crate::actor::Tangible;
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
/// [`Position`] will A* to its goal point. The path will only be recalculated
/// if the entity encounters a barrier in the way.
pub struct Pathfind {
  script: Vec<Box<dyn Tactic>>,
  goal: Option<Point>,
  path: Vec<Point>,
}

impl Pathfind {
  /// Creates a new `Pathfind`, using the given list of [`Tactic`]s to generate
  /// new goals.
  pub fn new(script: Vec<Box<dyn Tactic>>) -> Self {
    Pathfind {
      script,
      goal: None,
      path: Vec::new(),
    }
  }

  /// Re-runs this `Pathfind`'s goal-finding script.
  ///
  /// This function goes through each [`Tactic`] in the script, trying to find
  /// one which produces a new goal.
  pub fn refresh_goal(
    &mut self,
    fov: Option<&Fov>,
    world: &mut SubWorld,
    floor: &Floor,
  ) {
    for tactic in &mut self.script {
      if self.goal.is_some() && !tactic.run_always() {
        continue;
      }

      if let Some(goal) = tactic.generate_goal(fov, world, floor) {
        let requires_repath = self.goal != Some(goal);
        self.goal = Some(goal);
        if requires_repath {
          self.path.clear();
        }
        return;
      }
    }
  }

  /// Recomputes the path towards this `Pathfind`'s goal.
  pub fn repath(&mut self, current: Point, floor: &Floor, _occupied: &HashSet<Point>) {
    if let Some(goal) = self.goal {
      self.path = graph::manhattan_a_star(current, goal, |p| {
        // !occupied.contains(&p) &&
        floor
          .chunk(p)
          .map(|c| *c.tile(p) == Tile::Ground)
          .unwrap_or(false)
      })
      .unwrap_or(Vec::new());
    }
  }

  /// Computes the next point that the entity should walk to, if one is
  /// available.
  pub fn next_pos(&mut self, current: Point, floor: &Floor, occupied: &HashSet<Point>) -> Option<Point> {
    if self.goal.is_none() || self.goal == Some(current) {
      self.goal = None;
      return None;
    }

    // Check that the cached path is valid, which is given by our current
    // position being the last element. If it isn't, we re-path.
    if Some(&current) != self.path.last() {
      self.repath(current, floor, occupied);
    }

    self.path.pop();
    let next = self.path.last().cloned();
    if self.path.is_empty() {
      // We're done; make sure we can generate a new goal!
      self.goal = None;
    }
    next
  }
}

/// An AI tactic that uses information about the world to build goals to work
/// towards.
pub trait Tactic: Send + Sync {
  /// Whether to re-execute this tactic every simulation step.
  ///
  /// This should return `true` when the goal target (such as a moving entity)
  /// might change every tick.
  fn run_always(&self) -> bool {
    false
  }

  /// Attempts to generate a new goal, using the provided information.
  ///
  /// `fov` is the FOV of the current actor.
  /// `world` has acccess to all components that are readable by [`pathfind()`],
  /// except for [`Pathfind`] components.
  fn generate_goal(
    &mut self,
    fov: Option<&Fov>,
    world: &mut SubWorld,
    floor: &Floor,
  ) -> Option<Point>;
}

// `Tactic` is object safe!
impl dyn Tactic {}

/// A tactic that causes the entity to aimlessly wander around the floor.
///
/// Every time a new goal is needed, it picks a random point of a random room,
/// and A*s to it.
pub struct Wander;
impl Tactic for Wander {
  fn generate_goal(
    &mut self,
    _: Option<&Fov>,
    _: &mut SubWorld,
    floor: &Floor,
  ) -> Option<Point> {
    let mut rng = rand::thread_rng();
    let room = floor.rooms().choose(&mut rng)?;
    room.points().choose(&mut rng)
  }
}

/// A tactic for chasing a player in-view of the entity.
///
/// This goal is executed
pub struct Chase {
  target: Option<Entity>,
}
impl Chase {
  /// Creates a new `Chase`.
  pub fn new() -> Self {
    Self { target: None }
  }
}
impl Tactic for Chase {
  fn run_always(&self) -> bool {
    true
  }
  fn generate_goal(
    &mut self,
    fov: Option<&Fov>,
    world: &mut SubWorld,
    _: &Floor,
  ) -> Option<Point> {
    // First, check whether the entity we're chasing (if any) is currently in
    // sight. If not, delete it.
    fn check_if_visible(
      entity: Option<Entity>,
      fov: Option<&Fov>,
      world: &mut SubWorld,
    ) -> Option<Entity> {
      let entity = entity?;
      let mut query = <&Position>::query().filter(component::<Player>());
      let pos = query.get(world, entity).ok()?;

      if let Some(fov) = fov {
        if fov.visible.contains(&pos.0) {
          Some(entity)
        } else {
          None
        }
      } else {
        // In this case, the entity has no Fov component, making it
        // "omniscient".
        Some(entity)
      }
    }
    self.target = check_if_visible(self.target, fov, world);

    // Now, if there *isn't* a target, go and check if there is one we can use
    if self.target.is_none() {
      'outer: for chunk in <&Position>::query()
        .filter(component::<Player>())
        .iter_chunks(world)
      {
        for (entity, pos) in chunk.into_iter_entities() {
          if let Some(fov) = fov {
            if fov.visible.contains(&pos.0) {
              self.target = Some(entity);
              break 'outer;
            }
          } else {
            // In this case, the entity has no Fov component, making it
            // "omniscient".
            self.target = Some(entity);
            break 'outer;
          }
        }
      }
    }

    // Finally, if we *do* have an entity, use its position as our goal.
    self
      .target
      .and_then(|e| Some(<&Position>::query().get(world, e).ok()?.0))
  }
}

/// System: Steps forward the AI for each every [`Pathfind`] entity.
#[legion::system]
#[read_component(Fov)]
#[read_component(Player)]
#[read_component(Tangible)]
#[write_component(Position)]
#[write_component(Pathfind)]
pub fn pathfind(
  world: &mut SubWorld,
  #[resource] floor: &Floor,
  #[resource] mode: &TurnMode,
  #[resource] timer: &SystemTimer,
) {
  let _t = timer.start("actor::ai::pathfind()");
  if *mode != TurnMode::Running {
    return;
  }

  // First, kick all of the scripts to generate new goals, if necessary. This
  // does *not* mutate positions.
  let mut query = <(&mut Pathfind, Option<&Fov>)>::query();
  let (mut query_world, mut rest) = world.split_for_query(&query);
  for (pf, fov) in query.iter_mut(&mut query_world) {
    pf.refresh_goal(fov, &mut rest, floor);
  }

  let mut occupied = <&Position>::query()
    //.filter(component::<&Tangible>())
    .iter(world)
    .map(|p| p.0)
    .collect::<HashSet<_>>();

  // Now, step forward all of the pathfinding AIs. This requires mutating
  // positions, but does not require splitting the world.
  let mut q = <(&mut Pathfind, &mut Position, Option<&Tangible>)>::query();
  for (pf, pos, tangible) in q.iter_mut(world) {
    if let Some(p) = pf.next_pos(pos.0, floor, &occupied) {
      let is_walkable = floor
        .chunk(p)
        .map(|c| *c.tile(p) == Tile::Ground)
        .unwrap_or(false);

      // As an optimization, we assume that there is only ever one actor in a
      // given position, so we remove pos.0 and add p, though only if this
      // entity is tangible!
      // 
      // We try this a few times to make sure it converges, since there are
      // situations where a previous move invalidates a path.
      for _ in 0..3 {
        if is_walkable && !occupied.contains(&p) {
          if tangible.is_some() {
            occupied.remove(&pos.0);
            occupied.insert(p);
          }
          pos.0 = p;
          break
        } else {
          pf.repath(pos.0, floor, &occupied);
        }
      }
    }
  }
}
