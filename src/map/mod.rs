//! Dungeon maps.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::convert::TryInto as _;

use rand::distributions::Bernoulli;
use rand::distributions::Distribution as _;
use rand::distributions::Open01;
use rand::distributions::Uniform;

use crate::geo::Point;
use crate::geo::Rect;
use crate::geo::RectVec;
use crate::render::texel::Texel;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Tile {
  Void,
  Wall,
  Ground,
}

const WIDTH: usize = 32;

fn normalize(pos: Point) -> Point {
  Point::new(pos.x() & !(WIDTH as i64 - 1), pos.y() & !(WIDTH as i64 - 1))
}

pub struct Chunk {
  pos: Point,
  tiles: Box<[Tile; WIDTH * WIDTH]>,
}

impl Chunk {
  pub fn new(pos: Point) -> Chunk {
    Chunk {
      pos,
      tiles: vec![Tile::Void; WIDTH * WIDTH]
        .into_boxed_slice()
        .try_into()
        .unwrap(),
    }
  }

  pub fn image(&self) -> RectVec<Texel> {
    let mut rect = RectVec::new(self.rect(), Texel::new('\0'));
    for (tx, tile) in rect.data_mut().iter_mut().zip(self.tiles.iter()) {
      match tile {
        Tile::Void => {}
        Tile::Wall => *tx = Texel::new('+'),
        Tile::Ground => *tx = Texel::new('.'),
      };
    }
    rect
  }

  pub fn rect(&self) -> Rect {
    Rect::new(self.pos, self.pos + Point::new(WIDTH as i64, WIDTH as i64))
  }

  pub fn tile(&self, pos: Point) -> &Tile {
    let x = pos.x() as usize & (WIDTH - 1);
    let y = pos.y() as usize & (WIDTH - 1);

    &self.tiles[x + y * WIDTH]
  }

  pub fn tile_mut(&mut self, pos: Point) -> &mut Tile {
    let x = pos.x() as usize & (WIDTH - 1);
    let y = pos.y() as usize & (WIDTH - 1);

    &mut self.tiles[x + y * WIDTH]
  }
}

pub struct Floor {
  // Invariant: keys are always multiples of WIDTH, rounding towards negative
  // infinity.
  chunks: HashMap<Point, Chunk>,
}

impl Floor {
  pub fn new() -> Floor {
    Floor {
      chunks: HashMap::new(),
    }
  }

  pub fn chunk(&self, pos: Point) -> Option<&Chunk> {
    self.chunks.get(&normalize(pos))
  }

  /// Returns the `Chunk` containing the given position.
  pub fn chunk_mut(&mut self, pos: Point) -> &mut Chunk {
    self
      .chunks
      .entry(normalize(pos))
      .or_insert_with(move || Chunk::new(normalize(pos)))
  }

  pub fn chunks_in(
    &self,
    rect: Rect,
  ) -> impl Iterator<Item = (Rect, &Chunk)> + '_ {
    rect
      .disect(Rect::with_dims(WIDTH as i64, WIDTH as i64))
      .filter_map(move |r| self.chunk(r.corners().0).map(move |c| (r, c)))
  }

  pub fn add_room(&mut self, room: Rect) {
    for p in room.points() {
      let tile = if room.boundary_contains(p) {
        Tile::Wall
      } else {
        Tile::Ground
      };

      let slot = self.chunk_mut(p).tile_mut(p);
      if tile > *slot {
        *slot = tile;
      }
    }
  }

  pub fn add_horizontal(&mut self, start: Point, len: i64) {
    for dx in 0..=len.abs() {
      for dy in -1..=1 {
        let tile = match dy {
          0 => Tile::Ground,
          _ => Tile::Wall,
        };

        let p = start + Point::new(dx * len.signum(), dy);
        let slot = self.chunk_mut(p).tile_mut(p);
        if tile > *slot {
          *slot = tile;
        }
      }
    }
  }

  pub fn add_vertical(&mut self, start: Point, len: i64) {
    for dy in 0..=len.abs() {
      for dx in -1..=1 {
        let tile = match dx {
          0 => Tile::Ground,
          _ => Tile::Wall,
        };

        let p = start + Point::new(dx, dy * len.signum());
        let slot = self.chunk_mut(p).tile_mut(p);
        if tile > *slot {
          *slot = tile;
        }
      }
    }
  }

  pub fn rooms_and_corridors(
    &mut self,
    count: usize,
    bounds: Rect,
    min_size: Point,
    max_size: Point,
  ) {
    let mut rng = rand::thread_rng();

    let mut rooms = Vec::<Rect>::new();
    for _ in 0..count {
      let (start, end) = bounds.corners();
      let x = Uniform::new(start.x(), end.x()).sample(&mut rng);
      let y = Uniform::new(start.y(), end.y()).sample(&mut rng);

      let w = Uniform::new(min_size.x(), max_size.y()).sample(&mut rng);
      let h = Uniform::new(min_size.x(), max_size.y()).sample(&mut rng);

      let room = Rect::with_dims(w as _, h as _).centered_on(Point::new(x, y));
      if rooms.iter().any(|r| r.intersect(room).is_some()) {
        continue;
      }

      self.add_room(room);

      if let Some(prev) = rooms.last() {
        let x: f64 = Open01.sample(&mut rng);
        if x > 0.7 {
          continue;
        }

        if Bernoulli::new(0.5).unwrap().sample(&mut rng) {
          self.add_horizontal(
            prev.center(),
            room.center().x() - prev.center().x(),
          );
          self
            .add_vertical(room.center(), prev.center().y() - room.center().y());
        } else {
          self
            .add_vertical(prev.center(), room.center().y() - prev.center().y());
          self.add_horizontal(
            room.center(),
            prev.center().x() - room.center().x(),
          );
        }
      }

      rooms.push(room);
    }
  }
}
