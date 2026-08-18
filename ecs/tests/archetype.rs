use moonhowl_ecs::{Component, Entity};

struct Position {
    x: f32,
    y: f32,
}

impl Component for Position {}

struct Velocity {
    dx: f32,
    dy: f32,
}

impl Component for Velocity {}

struct Name(&'static str);

impl Component for Name {}

#[test]
fn single_component_is_a_one_component_archetype() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 });

    let sum = entity.with_archetype::<Position, _>(|pos| pos.x + pos.y);

    assert_eq!(sum, Some(3.0));
}

#[test]
fn tuple_archetype_matches_only_when_every_component_is_present() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 });

    let mut ran = false;
    let moved = entity.with_archetype::<(Position, Velocity), _>(|(position, velocity)| ran = true);
    assert_eq!(moved, None);
    assert!(!ran);

    entity.set_component(Velocity { dx: 0.5, dy: -0.5 });

    let moved = entity.with_archetype::<(Position, Velocity), _>(|(pos, vel)| {
        (pos.x + vel.dx, pos.y + vel.dy)
    });
    assert_eq!(moved, Some((1.5, 1.5)));
}

#[test]
fn larger_tuple_archetype_and_unset_component() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("player"));

    let label = entity
        .with_archetype::<(Position, Velocity, Name), _>(|(pos, vel, name)| {
            format!("{} at ({}, {}) moving ({}, {})", name.0, pos.x, pos.y, vel.dx, vel.dy)
        });
    assert_eq!(label, Some("player at (0, 0) moving (1, 1)".to_string()));

    entity.unset_component::<Name>();
    assert!(!entity.has_component::<Name>());

    let label = entity.with_archetype::<(Position, Velocity, Name), _>(|_| ());
    assert_eq!(label, None);
}

#[test]
fn single_component_mutation() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 });

    entity.with_archetype_mut::<Position, _>(|pos| {
        pos.x += 10.0;
    });

    let x = entity.with_archetype::<Position, _>(|pos| pos.x);
    assert_eq!(x, Some(11.0));
}

#[test]
fn tuple_archetype_mutation_updates_every_component_in_place() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: -1.0 });

    let applied = entity.with_archetype_mut::<(Position, Velocity), _>(|(pos, vel)| {
        pos.x += vel.dx;
        pos.y += vel.dy;
        vel.dx *= 2.0;
        true
    });
    assert_eq!(applied, Some(true));

    let state = entity
        .with_archetype::<(Position, Velocity), _>(|(pos, vel)| (pos.x, pos.y, vel.dx, vel.dy));
    assert_eq!(state, Some((1.0, -1.0, 2.0, -1.0)));
}

#[test]
fn tuple_archetype_mutation_misses_without_running_when_a_component_is_absent() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 0.0, y: 0.0 });

    let mut ran = false;
    let result = entity.with_archetype_mut::<(Position, Velocity), _>(|_| ran = true);

    assert_eq!(result, None);
    assert!(!ran);
}

#[test]
fn edit_component_mutates_in_place_and_is_a_noop_when_absent() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 });

    entity
        .edit_component::<Position>(|pos| pos.x += 1.0)
        .edit_component::<Velocity>(|vel| vel.dx += 1.0);

    let x = entity.with_archetype::<Position, _>(|pos| pos.x);
    assert_eq!(x, Some(2.0));
    assert!(!entity.has_component::<Velocity>());
}
