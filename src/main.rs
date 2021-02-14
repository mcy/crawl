//! ...

#![deny(unused)]
#![deny(warnings)]

use std::collections::HashSet;
use std::time::Duration;

pub mod actor;
pub mod geo;
pub mod input;
pub mod map;
pub mod render;
pub mod timing;
pub mod ui;

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
    Magic,
    Spacer(Option<usize>),
    X,
    Y,
    Gold,
  }

  impl Widget for WType {
    type State = WState;

    fn update(&self, state: &Self::State, shape: &mut Shape) {
      *shape = match self {
        Self::Health => Shape::Bar {
          label: "HP".into(),
          label_color: colors::RED.into(),

          value_range: (state.health as i32, 200),
          width_range: (10, 20),

          brackets: (
            Texel::new('[').with_fg(colors::WHITE),
            Texel::new(']').with_fg(colors::WHITE),
          ),
          active: Texel::new('|').with_fg(colors::RED),
          inactive: Texel::new('|').with_fg(colors::DARKGRAY),
          include_digits: true,
        },
        Self::Magic => Shape::Bar {
          label: "MP".into(),
          label_color: colors::ROYALBLUE.into(),

          value_range: (50, 50),
          width_range: (10, 15),

          brackets: (
            Texel::new('[').with_fg(colors::WHITE),
            Texel::new(']').with_fg(colors::WHITE),
          ),
          active: Texel::new('*').with_fg(colors::ROYALBLUE),
          inactive: Texel::new(' ').with_fg(colors::DARKGRAY),
          include_digits: false,
        },
        Self::Spacer(limit) => Shape::Fill(Texel::empty(), *limit),
        Self::X => Shape::Scalar {
          label: "x: ".into(),
          label_color: Color::Reset,
          value: state.pos.x() as i32,
          value_color: colors::CYAN.into(),
        },
        Self::Y => Shape::Scalar {
          label: "y: ".into(),
          label_color: Color::Reset,
          value: state.pos.y() as i32,
          value_color: colors::CYAN.into(),
        },
        Self::Gold => Shape::Scalar {
          label: "$".into(),
          label_color: Color::Reset,
          value: state.gold as i32,
          value_color: colors::GOLD.into(),
        },
      }
    }
  }

  let mut bar = WidgetBar::new(WState {
    health: 55,
    pos: Point::zero(),
    gold: 42,
  });
  bar.push(WType::Health, 10);
  bar.push(WType::Spacer(Some(1)), 11);
  bar.push(WType::Magic, 12);
  bar.push(WType::Spacer(None), 20);
  bar.push(WType::X, 30);
  bar.push(WType::Spacer(Some(1)), 31);
  bar.push(WType::Y, 32);
  bar.push(WType::Spacer(None), 40);
  bar.push(WType::Gold, 50);

  let mut timer = SystemTimer::new();
  timer.register("process_visibility");

  let mut resources = Resources::default();
  resources.insert(Mutex::new(render::curses::Curses::init()));
  resources.insert(FrameTimer::new());
  resources.insert(timer);
  resources.insert(floor);
  resources.insert(input::UserInput::new());
  resources.insert(Renderer::new());
  resources.insert(bar);

  #[legion::system]
  fn quit(
    #[resource] input: &mut input::UserInput,
    #[resource] window: &Mutex<render::curses::Curses>,
  ) {
    if input.has_key(input::KeyCode::Char('q')) {
      window.lock().unwrap().die(0);
    }
  }

  #[legion::system]
  #[read_component(Player)]
  #[write_component(Position)]
  fn process_input(
    world: &mut SubWorld,
    #[resource] floor: &Floor,
    #[resource] input: &input::UserInput,
    #[resource] timer: &mut SystemTimer,
    #[resource] widget_bar: &mut WidgetBar<WType>,
  ) {
    let _t = timer.start("process_input");
    let dirs = &[
      (input::KeyCode::Char('a'), Point::new(-1, 0)),
      (input::KeyCode::Char('d'), Point::new(1, 0)),
      (input::KeyCode::Char('w'), Point::new(0, -1)),
      (input::KeyCode::Char('s'), Point::new(0, 1)),
    ];

    for (pos, _) in <(&mut Position, &Player)>::query().iter_mut(world) {
      for &(k, dir) in dirs {
        if input.has_key(k) {
          let new_pos = pos.0 + dir;
          match floor.chunk(new_pos).unwrap().tile(new_pos) {
            Tile::Wall | Tile::Void => continue,
            _ => {}
          };
          pos.0 += dir;
        }
      }
      widget_bar.state_mut().pos = pos.0;
      widget_bar.mark_dirty();
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
    let camera = <&Position>::query()
      .filter(query::component::<HasCamera>())
      .iter(world)
      .map(|Position(p)| *p)
      .next()
      .unwrap_or(Point::zero());

    let mut scene = Scene::new(camera, true);
    for (_, chunk) in floor.chunks_in(scene.viewport()) {
      scene.push(0, chunk.image());
    }

    for (pos, Sprite(tx)) in <(&Position, &Sprite)>::query().iter(world) {
      scene.push(
        1,
        RectVec::new(Rect::with_dims(1, 1).centered_on(pos.0), *tx),
      );
    }

    let mut view_mask = RectVec::new(scene.viewport(), Texel::new(' '));
    for fov in <&Fov>::query().iter(world) {
      for p in &fov.seen {
        view_mask
          .get_mut(*p)
          .map(|t| *t = Texel::empty().with_fg(colors::GRAY));
      }
      for p in &fov.visible {
        view_mask.get_mut(*p).map(|t| *t = Texel::empty());
      }
    }

    scene.push(2, view_mask);

    let widgets = widget_bar.draw(80);
    let mut texels = RectVec::new(
      Rect::with_dims(80, 1).centered_on(scene.camera() + Point::new(0, 12)),
      Texel::empty(),
    );
    texels.data_mut().copy_from_slice(widgets);

    scene.push(3, texels);

    let fps = frame_timer.measure_fps(Duration::from_millis(500));
    let count = frame_timer.frame_count();
    scene.debug(format!("fps: {:.2}, count: {}", fps, count));

    scene.debug(format!(
      "vis: {:.2}us",
      timer
        .measure("process_visibility", Duration::from_millis(500))
        .as_micros()
    ));

    let mut window = window.lock().unwrap();
    renderer.bake(scene, &mut *window);
    frame_timer.end_frame(60);
  }

  let mut schedule = Schedule::builder()
    .add_system(input::start_frame_system())
    .add_system(quit_system())
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
