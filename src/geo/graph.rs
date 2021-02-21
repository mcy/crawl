//! Graph algorithms, primarially for use by AI.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;

use crate::geo::Dir;
use crate::geo::Point;

/// Implements the A* pathfinding algorithm with Manhattan distance and
/// heuristic functions.
///
/// See [`a_star()`].
pub fn manhattan_a_star(
  start: Point,
  goal: Point,
  can_walk: impl FnMut(Point) -> bool,
) -> Option<Vec<Point>> {
  a_star(
    start,
    goal,
    can_walk,
    |a, b| (a - b).manhattan() as f64,
    move |n| (n - goal).manhattan() as f64,
  )
}

/// Implements the A* pathfinding algorithm.
///
/// This function will attempt to find a path from `start` to `goal`; if no path
/// could be found, `None` is returned.
///
/// The provided functions serve the following purposes:
/// - `can_walk` returns true if a particular point is accessible for the
///   purposes of this search.
/// - `distance` measures the distance between two points. Manhattan distance is
///   recommended here.
/// - `heuristc` is the A* heuristic function, which roughly describes the cost
///   to reach the goal from a particular node.
///   
/// The path returned is in *reverse order*; that is, the goal will be the first
/// element of the path.
pub fn a_star(
  start: Point,
  goal: Point,
  mut can_walk: impl FnMut(Point) -> bool,
  mut distance: impl FnMut(Point, Point) -> f64,
  mut heuristic: impl FnMut(Point) -> f64,
) -> Option<Vec<Point>> {
  #[derive(Copy, Clone)]
  struct Node(f64, Point);
  impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
      self.0 == other.0
    }
  }
  impl Eq for Node {}
  impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      self.0.partial_cmp(&other.0).map(Ordering::reverse)
    }
  }
  impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
      self.partial_cmp(other).unwrap_or(Ordering::Less)
    }
  }
  let mut open_nodes = BinaryHeap::<Node>::new();

  let mut came_from = HashMap::new();
  let mut g_scores = HashMap::new();

  g_scores.insert(start, 0.0);
  open_nodes.push(Node(heuristic(start), start));

  while let Some(Node(_, mut current)) = open_nodes.pop() {
    if current == goal {
      // We're done, let's build a path back from the goal.
      let mut path = vec![current];
      while let Some(&next) = came_from.get(&current) {
        current = next;
        path.push(current);
      }
      return Some(path);
    }

    for &d in &Dir::all() {
      let neighbor = current + d.to_point::<i64>();
      if !can_walk(neighbor) {
        continue;
      }

      let tentative_g =
        g_scores.get(&current).cloned().unwrap_or(f64::INFINITY)
          + distance(current, neighbor);
      if tentative_g < g_scores.get(&neighbor).cloned().unwrap_or(f64::INFINITY)
      {
        came_from.insert(neighbor, current);
        g_scores.insert(neighbor, tentative_g);
        open_nodes.push(Node(tentative_g + heuristic(neighbor), neighbor));
      }
    }
  }

  None
}
