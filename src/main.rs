//! ...

#![deny(unused)]
#![deny(warnings)]

use std::collections::HashSet;
use std::time::Duration;

pub mod actor;
pub mod geo;
pub mod gfx;
pub mod input;
pub mod map;
pub mod timing;
pub mod ui;

fn main() {
  use crate::actor::*;
  use crate::geo::*;
  use crate::gfx::texel::*;
  use crate::map::*;
  use crate::timing::*;
  use crate::ui::widget::*;

  use legion::query;
  use legion::world::SubWorld;
  use legion::IntoQuery as _;
  use legion::Resources;
  use legion::Schedule;
  use legion::World;

  let mut floor = Floor::new();
  floor.rooms_and_corridors(
    50,
    Rect::with_dims(200, 200).centered_on(Point::zero()),
    Point::new(10, 10),
    Point::new(30, 30),
  );
  let rooms = floor.rooms();

  let mut world = World::default();
  world.push((
    Player,
    HasCamera,
    Position(rooms[0].center()),
    Tangible,
    Fov {
      range: Point::new(20, 10),
      visible: HashSet::new(),
      seen: HashSet::new(),
    },
    Sprite(Texel::new('@')),
  ));

  for room in &rooms[1..] {
    world.push((
      Position(room.center()),
      Tangible,
      Fov {
        range: Point::new(20, 10),
        visible: HashSet::new(),
        seen: HashSet::new(),
      },
      Sprite(Texel::new('K')),
      ai::Pathfind::new(),
    ));
  }

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

  let mut resources = Resources::default();
  resources.insert(gfx::Curses::init());
  resources.insert(FrameTimer::new());
  resources.insert(SystemTimer::new());
  resources.insert(floor);
  resources.insert(input::UserInput::new());
  resources.insert(actor::ai::TurnMode::Waiting);
  resources.insert(gfx::Renderer::new());
  resources.insert(bar);

  #[legion::system]
  fn quit(
    #[resource] input: &mut input::UserInput,
    #[resource] window: &mut gfx::Curses,
  ) {
    if input.has_key(input::KeyCode::Char('q')) {
      window.die(0);
    }
  }

  #[legion::system(for_each)]
  #[write_component(Position)]
  #[filter(legion::component::<Player>())]
  fn process_input(
    &mut Position(ref mut pos): &mut Position,
    #[resource] floor: &Floor,
    #[resource] input: &input::UserInput,
    #[resource] timer: &SystemTimer,
    #[resource] turn_mode: &mut actor::ai::TurnMode,
  ) {
    let _t = timer.start("process_input()");

    let dirs = &[
      (input::KeyCode::Char('a'), Point::new(-1, 0)),
      (input::KeyCode::Char('d'), Point::new(1, 0)),
      (input::KeyCode::Char('w'), Point::new(0, -1)),
      (input::KeyCode::Char('s'), Point::new(0, 1)),
      (input::KeyCode::Char('f'), Point::new(0, 0)),
    ];

    for &(k, dir) in dirs {
      if input.has_key(k) {
        let new_pos = *pos + dir;
        match floor.chunk(new_pos).unwrap().tile(new_pos) {
          Tile::Wall | Tile::Void => continue,
          _ => {}
        };
        *pos += dir;
        *turn_mode = actor::ai::TurnMode::Running;
      }
    }
  }

  #[legion::system(for_each)]
  #[read_component(Position)]
  #[filter(legion::component::<Player>())]
  fn update_widgets(
    &Position(pos): &Position,
    #[resource] timer: &SystemTimer,
    #[resource] widget_bar: &mut WidgetBar<WType>,
  ) {
    let _t = timer.start("update_widgets()");

    let state = widget_bar.state_mut();
    if state.pos != pos {
      state.pos = pos;
      widget_bar.mark_dirty();
    }
  }

  #[legion::system(for_each)]
  #[read_component(Position)]
  #[write_component(Fov)]
  fn process_visibility(
    &Position(pos): &Position,
    fov: &mut Fov,
    #[resource] floor: &Floor,
    #[resource] timer: &SystemTimer,
  ) {
    let _t = timer.start("process_visibility()");
    fov.visible.clear();
    geo::fov::milazzo(
      pos,
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
  #[read_component(ai::Pathfind)]
  fn render(
    world: &SubWorld,
    #[resource] frame_timer: &mut FrameTimer,
    #[resource] timer: &SystemTimer,
    #[resource] floor: &Floor,
    #[resource] window: &gfx::Curses,
    #[resource] renderer: &mut gfx::Renderer,
    #[resource] widget_bar: &mut WidgetBar<WType>,
  ) {
    let t = timer.start("render()");
    let camera = <&Position>::query()
      .filter(query::component::<HasCamera>())
      .iter(world)
      .map(|Position(p)| *p)
      .next()
      .unwrap_or(Point::zero());

    let mut scene = gfx::Scene::new(camera, true);
    let viewport = scene.viewport();

    let mut map_layer = scene.image_layer(0);
    for (_, chunk) in floor.chunks_in(viewport) {
      map_layer.push(chunk.image());
    }
    map_layer.finish();

    let mut sprite_layer = scene.image_layer(1);
    for (pos, Sprite(tx)) in <(&Position, &Sprite)>::query().iter(world) {
      sprite_layer
        .push(RectVec::new(Rect::with_dims(1, 1).centered_on(pos.0), *tx));
    }

    sprite_layer.finish();

    let mut fov_layer = scene.image_layer(2);
    let mut fov_mask = RectVec::new(viewport, Texel::new(' '));
    for fov in <&Fov>::query()
      .filter(legion::component::<Player>())
      .iter(world)
    {
      for p in &fov.seen {
        fov_mask
          .get_mut(*p)
          .map(|t| *t = Texel::empty().with_fg(colors::GRAY));
      }
      for p in &fov.visible {
        fov_mask.get_mut(*p).map(|t| *t = Texel::empty());
      }
    }
    fov_layer.push(fov_mask);
    fov_layer.finish();

    let mut ui_layer = scene.image_layer(3);
    let widgets = widget_bar.draw(80);
    let mut widget_data = RectVec::new(
      Rect::with_dims(80, 1)
        .centered_on(ui_layer.scene().camera() + Point::new(0, 12)),
      Texel::empty(),
    );
    widget_data.data_mut().copy_from_slice(widgets);
    ui_layer.push(widget_data);
    ui_layer.finish();

    let fps = frame_timer.measure_fps(Duration::from_millis(500));
    let count = frame_timer.frame_count();
    scene.debug(format!("fps: {:.2}, count: {}", fps, count));

    scene.debug("Timings:".into());
    for (system, duration) in timer.measure_all(Duration::from_millis(500)) {
      scene.debug(format!(
        " {}: {:.4}ms",
        system,
        duration.as_secs_f64() * 1000.0
      ));
    }
    t.finish();
    let _t = timer.start("renderer.bake()");

    renderer.bake(scene, window);
    frame_timer.end_frame(60);
  }

  let mut schedule = Schedule::builder()
    .add_system(input::start_frame_system())
    .add_system(quit_system())
    .add_system(process_input_system())
    .add_system(update_widgets_system())
    .flush()
    .add_system(process_visibility_system())
    .add_system(actor::ai::pathfind_system())
    .flush()
    .add_system(actor::ai::end_turn_system())
    .add_system(render_system())
    .build();

  loop {
    schedule.execute(&mut world, &mut resources);
  }
}
