//! ...

#![deny(unused)]
#![deny(warnings)]
#![deny(missing_docs)]

use std::collections::HashSet;
use std::time::Duration;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

pub mod actor;
pub mod geo;
pub mod map;
pub mod render;
pub mod timing;
pub mod ui;

#[allow(missing_docs)]
fn main() {
  use std::sync::Mutex;

  use crate::actor::*;
  use crate::geo::*;
  use crate::map::*;
  use crate::render::texel::*;
  use crate::render::*;
  use crate::timing::*;
  use crate::ui::widget::*;

  use legion::query;
  use legion::world::SubWorld;
  use legion::IntoQuery as _;
  use legion::Resources;
  use legion::Schedule;
  use legion::World;

  let mut world = World::default();
  world.push((
    Player,
    HasCamera,
    Position(Point::zero()),
    Tangible,
    Fov {
      range: Point::new(20, 10),
      visible: HashSet::new(),
      seen: HashSet::new(),
    },
    Sprite(Texel::new('@')),
  ));

  let mut floor = Floor::new();
  floor.rooms_and_corridors(
    50,
    Rect::with_dims(200, 200).centered_on(Point::zero()),
    Point::new(10, 10),
    Point::new(30, 30),
  );

  #[allow(unused)]
  struct WState {
    health: u32,
    pos: Point,
    gold: u32,
  }

  enum WType {
    Health,
    Spacer,
    X,
    Y,
    Gold,
  }

  impl Widget for WType {
    type State = WState;

    fn update(&self, state: &Self::State, shape: &mut Shape) {
      *shape = match self {
        Self::Health => Shape::Bar {
          label: Some("HP".into()),
          fill_color: colors::RED,
          value_range: (state.health as i32, 200),
          width_range: (10, 20),
        },
        Self::Spacer => Shape::Fill(Texel::new('\0')),
        Self::X => Shape::Scalar {
          label: Some("x: ".into()),
          color: colors::CYAN,
          value: state.pos.x() as i32,
        },
        Self::Y => Shape::Scalar {
          label: Some(" y: ".into()),
          color: colors::CYAN,
          value: state.pos.y() as i32,
        },
        Self::Gold => Shape::Scalar {
          label: Some("$".into()),
          color: colors::GOLD,
          value: state.gold as i32,
        },
      }
    }
  }

  let mut bar = WidgetBar::new(WState {
    health: 55,
    pos: Point::zero(),
    gold: 42,
  });
  bar.push(WType::Health, 0);
  bar.push(WType::Spacer, 1);
  bar.push(WType::X, 2);
  bar.push(WType::Y, 3);
  bar.push(WType::Spacer, 4);
  bar.push(WType::Gold, 5);

  let mut timer = SystemTimer::new();
  timer.register("process_visibility");

  struct Inputs {
    keys: HashSet<KeyEvent>,
  }

  let mut resources = Resources::default();
  resources.insert(Mutex::new(render::curses::Curses::init()));
  resources.insert(FrameTimer::new());
  resources.insert(timer);
  resources.insert(floor);
  resources.insert(Inputs {
    keys: HashSet::new(),
  });
  resources.insert(Renderer::new());
  resources.insert(bar);

  #[legion::system]
  fn consume_inputs(
    #[resource] input: &mut Inputs,
    #[resource] window: &Mutex<render::curses::Curses>,
    #[resource] timer: &mut SystemTimer,
  ) {
    let _t = timer.start("consume_inputs");
    let mut window = window.lock().unwrap();
    input.keys.clear();
    for k in window.keys() {
      input.keys.insert(k);
    }

    if input
      .keys
      .contains(&KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()))
    {
      window.die(0);
    }
  }

  #[legion::system]
  #[read_component(Player)]
  #[write_component(Position)]
  fn process_input(
    world: &mut SubWorld,
    #[resource] floor: &Floor,
    #[resource] input: &mut Inputs,
    #[resource] timer: &mut SystemTimer,
    #[resource] widget_bar: &mut WidgetBar<WType>,
  ) {
    let _t = timer.start("process_input");
    let dirs = &[
      (KeyCode::Char('a'), Point::new(-1, 0)),
      (KeyCode::Char('d'), Point::new(1, 0)),
      (KeyCode::Char('w'), Point::new(0, -1)),
      (KeyCode::Char('s'), Point::new(0, 1)),
    ];

    for (pos, _) in <(&mut Position, &Player)>::query().iter_mut(world) {
      for &(k, dir) in dirs {
        if input
          .keys
          .contains(&KeyEvent::new(k, KeyModifiers::empty()))
        {
          let new_pos = pos.0 + dir;
          match floor.chunk(new_pos).unwrap().tile(new_pos) {
            Tile::Wall | Tile::Void => continue,
            _ => {}
          };
          pos.0 += dir;
        }
      }
      widget_bar.state_mut().pos = pos.0;
    }
  }

  #[legion::system(for_each)]
  #[read_component(Position)]
  #[write_component(Fov)]
  fn process_visibility(
    pos: &Position,
    fov: &mut Fov,
    #[resource] floor: &Floor,
    #[resource] timer: &mut SystemTimer,
  ) {
    let _t = timer.start("process_visibility");
    fov.visible.clear();
    geo::fov::milazzo(
      pos.0,
      fov.range,
      &mut |p| {
        floor
          .chunk(p)
          .map(|c| *c.tile(p) != Tile::Ground)
          .unwrap_or(true)
      },
      &mut |p| {
        fov.visible.insert(p);
        fov.seen.insert(p);
      },
    );
  }

  #[legion::system]
  #[read_component(HasCamera)]
  #[read_component(Position)]
  #[read_component(Sprite)]
  #[read_component(Fov)]
  fn render(
    world: &SubWorld,
    #[resource] frame_timer: &mut FrameTimer,
    #[resource] timer: &mut SystemTimer,
    #[resource] floor: &Floor,
    #[resource] window: &Mutex<render::curses::Curses>,
    #[resource] renderer: &mut Renderer,
    #[resource] widget_bar: &mut WidgetBar<WType>,
  ) {
    let mut window = window.lock().unwrap();
    let mut scene = Scene {
      elements: Vec::new(),
      debug: Vec::new(),
      camera: Point::zero(),
      void: Texel::new('#'),
    };

    for pos in <&Position>::query()
      .filter(query::component::<HasCamera>())
      .iter(world)
    {
      scene.camera = pos.0;
      break;
    }

    let (rows, cols) = window.dims();
    let viewport =
      Rect::with_dims(cols as i64, rows as i64).centered_on(scene.camera);

    for (_, chunk) in floor.chunks_in(viewport) {
      scene.elements.push(Element {
        priority: 0,
        kind: ElementKind::Image(chunk.image()),
      });
    }

    for (pos, Sprite(tx)) in <(&Position, &Sprite)>::query().iter(world) {
      scene.elements.push(Element {
        priority: 1,
        kind: ElementKind::Image(RectVec::new(
          Rect::with_dims(1, 1).centered_on(pos.0),
          *tx,
        )),
      });
    }

    let mut visible_points = HashSet::new();
    let mut seen_points = HashSet::new();
    for fov in <&Fov>::query().iter(world) {
      for p in &fov.visible {
        visible_points.insert(p);
      }
      for p in &fov.seen {
        seen_points.insert(p);
      }
    }

    scene.elements.push(Element {
      priority: 2,
      kind: ElementKind::Shader(Box::new(move |p, tx| {
        if visible_points.contains(&p) {
          tx
        } else if seen_points.contains(&p) {
          Texel::colored(tx.glyph(), Some(palette::named::GRAY), None)
        } else {
          Texel::new(' ')
        }
      })),
    });

    let widgets = widget_bar.draw(80);
    let mut texels = RectVec::new(
      Rect::with_dims(80, 1).centered_on(scene.camera + Point::new(0, 12)),
      Texel::new('\0'),
    );
    texels.data_mut().copy_from_slice(widgets);

    scene.elements.push(Element {
      priority: 3,
      kind: ElementKind::Image(texels),
    });

    let fps = frame_timer.measure_fps(Duration::from_millis(500));
    let count = frame_timer.frame_count();
    scene
      .debug
      .push(format!("fps: {:.2}, count: {}", fps, count));

    scene.debug.push(format!(
      "vis: {:.2}us",
      timer.measure("process_visibility", Duration::from_millis(500)).as_micros()
    ));

    renderer.bake(scene, &mut *window);
    frame_timer.end_frame(60);
  }

  let mut schedule = Schedule::builder()
    .add_system(consume_inputs_system())
    .add_system(process_input_system())
    .flush()
    .add_system(process_visibility_system())
    .flush()
    .add_system(render_system())
    .build();

  loop {
    schedule.execute(&mut world, &mut resources);
  }
}
