use ecs::{Component, World};

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

struct Health(i32);

impl Component for Health {}

struct Scale(f32);

impl Component for Scale {}

struct Tag(&'static str);

impl Component for Tag {}

#[test]
fn single_component_is_a_one_component_archetype() {
    let mut world = World::new();
    let entity = world.spawn();
    world.set_component(entity, Position { x: 1.0, y: 2.0 });

    let sum = world.get::<Position>(entity).map(|pos| pos.x + pos.y);

    assert_eq!(sum, Some(3.0));
}

#[test]
fn tuple_archetype_matches_only_when_every_component_is_present() {
    let mut world = World::new();
    let entity = world.spawn();
    world.set_component(entity, Position { x: 1.0, y: 2.0 });

    let moved = world.get::<(Position, Velocity)>(entity);
    assert!(moved.is_none());

    world.set_component(entity, Velocity { dx: 0.5, dy: -0.5 });

    let moved = world
        .get::<(Position, Velocity)>(entity)
        .map(|(pos, vel)| (pos.x + vel.dx, pos.y + vel.dy));
    assert_eq!(moved, Some((1.5, 1.5)));
}

#[test]
fn larger_tuple_archetype_and_unset_component() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 0.0, y: 0.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 1.0 })
        .set_component(entity, Name("player"));

    let label = world
        .get::<(Position, Velocity, Name)>(entity)
        .map(|(pos, vel, name)| format!("{} at ({}, {}) moving ({}, {})", name.0, pos.x, pos.y, vel.dx, vel.dy));
    assert_eq!(label, Some("player at (0, 0) moving (1, 1)".to_string()));

    world.unset_component::<Name>(entity);
    assert!(!world.has_component::<Name>(entity));

    let label = world.get::<(Position, Velocity, Name)>(entity);
    assert!(label.is_none());
}

#[test]
fn single_component_mutation() {
    let mut world = World::new();
    let entity = world.spawn();
    world.set_component(entity, Position { x: 1.0, y: 2.0 });

    world.get_mut::<Position>(entity).unwrap().x += 10.0;

    let x = world.get::<Position>(entity).map(|pos| pos.x);
    assert_eq!(x, Some(11.0));
}

#[test]
fn tuple_archetype_mutation_updates_every_component_in_place() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 0.0, y: 0.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: -1.0 });

    let applied = world.get_mut::<(Position, Velocity)>(entity).map(|(pos, vel)| {
        pos.x += vel.dx;
        pos.y += vel.dy;
        vel.dx *= 2.0;
        true
    });
    assert_eq!(applied, Some(true));

    let state = world
        .get::<(Position, Velocity)>(entity)
        .map(|(pos, vel)| (pos.x, pos.y, vel.dx, vel.dy));
    assert_eq!(state, Some((1.0, -1.0, 2.0, -1.0)));
}

#[test]
fn tuple_archetype_mutation_misses_without_running_when_a_component_is_absent() {
    let mut world = World::new();
    let entity = world.spawn();
    world.set_component(entity, Position { x: 0.0, y: 0.0 });

    let result = world.get_mut::<(Position, Velocity)>(entity);

    assert!(result.is_none());
}

#[test]
fn edit_component_mutates_in_place_and_is_a_noop_when_absent() {
    let mut world = World::new();
    let entity = world.spawn();
    world.set_component(entity, Position { x: 1.0, y: 2.0 });

    world
        .edit_component::<Position>(entity, |pos| pos.x += 1.0)
        .edit_component::<Velocity>(entity, |vel| vel.dx += 1.0);

    let x = world.get::<Position>(entity).map(|pos| pos.x);
    assert_eq!(x, Some(2.0));
    assert!(!world.has_component::<Velocity>(entity));
}

#[test]
fn three_component_archetype_mutation() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 0.0, y: 0.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 1.0 })
        .set_component(entity, Name("three"));

    let applied = world
        .get_mut::<(Position, Velocity, Name)>(entity)
        .map(|(pos, vel, _name)| {
            pos.x += vel.dx;
            pos.y += vel.dy;
            true
        });
    assert_eq!(applied, Some(true));

    let position = world.get::<Position>(entity).map(|pos| (pos.x, pos.y));
    assert_eq!(position, Some((1.0, 1.0)));

    world.unset_component::<Name>(entity);
    let result = world.get_mut::<(Position, Velocity, Name)>(entity);
    assert!(result.is_none());
}

#[test]
fn single_component_archetype_is_none_when_absent() {
    let mut world = World::new();
    let entity = world.spawn();

    let result = world.get::<Position>(entity).map(|pos| pos.x);

    assert_eq!(result, None);
}

#[test]
fn single_component_mutation_is_none_when_absent() {
    let mut world = World::new();
    let entity = world.spawn();

    let result = world.get_mut::<Position>(entity);

    assert!(result.is_none());
}

#[test]
fn four_component_archetype() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 1.0, y: 1.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 1.0 })
        .set_component(entity, Name("four"))
        .set_component(entity, Health(10));

    let snapshot = world
        .get::<(Position, Velocity, Name, Health)>(entity)
        .map(|(pos, vel, name, health)| (pos.x, vel.dx, name.0, health.0));
    assert_eq!(snapshot, Some((1.0, 1.0, "four", 10)));

    world.unset_component::<Health>(entity);
    let snapshot = world.get::<(Position, Velocity, Name, Health)>(entity);
    assert!(snapshot.is_none());
}

#[test]
fn four_component_archetype_mutation() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 0.0, y: 0.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 1.0 })
        .set_component(entity, Name("four"))
        .set_component(entity, Health(10));

    let applied = world
        .get_mut::<(Position, Velocity, Name, Health)>(entity)
        .map(|(pos, vel, _name, health)| {
            pos.x += vel.dx;
            health.0 -= 1;
            true
        });
    assert_eq!(applied, Some(true));

    let health = world.get::<Health>(entity).map(|h| h.0);
    assert_eq!(health, Some(9));

    world.unset_component::<Name>(entity);
    let result = world.get_mut::<(Position, Velocity, Name, Health)>(entity);
    assert!(result.is_none());
}

#[test]
fn five_component_archetype() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 1.0, y: 1.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 1.0 })
        .set_component(entity, Name("five"))
        .set_component(entity, Health(10))
        .set_component(entity, Scale(2.0));

    let snapshot = world
        .get::<(Position, Velocity, Name, Health, Scale)>(entity)
        .map(|(pos, vel, name, health, scale)| (pos.x, vel.dx, name.0, health.0, scale.0));
    assert_eq!(snapshot, Some((1.0, 1.0, "five", 10, 2.0)));

    world.unset_component::<Scale>(entity);
    let snapshot = world.get::<(Position, Velocity, Name, Health, Scale)>(entity);
    assert!(snapshot.is_none());
}

#[test]
fn five_component_archetype_mutation() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 0.0, y: 0.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 1.0 })
        .set_component(entity, Name("five"))
        .set_component(entity, Health(10))
        .set_component(entity, Scale(2.0));

    let applied = world
        .get_mut::<(Position, Velocity, Name, Health, Scale)>(entity)
        .map(|(pos, vel, _name, health, scale)| {
            pos.x += vel.dx;
            health.0 -= 1;
            scale.0 *= 2.0;
            true
        });
    assert_eq!(applied, Some(true));

    let scale = world.get::<Scale>(entity).map(|s| s.0);
    assert_eq!(scale, Some(4.0));

    world.unset_component::<Health>(entity);
    let result = world.get_mut::<(Position, Velocity, Name, Health, Scale)>(entity);
    assert!(result.is_none());
}

#[test]
fn six_component_archetype() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 1.0, y: 1.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 1.0 })
        .set_component(entity, Name("six"))
        .set_component(entity, Health(10))
        .set_component(entity, Scale(2.0))
        .set_component(entity, Tag("player"));

    let snapshot = world
        .get::<(Position, Velocity, Name, Health, Scale, Tag)>(entity)
        .map(|(pos, vel, name, health, scale, tag)| (pos.x, vel.dx, name.0, health.0, scale.0, tag.0));
    assert_eq!(snapshot, Some((1.0, 1.0, "six", 10, 2.0, "player")));

    world.unset_component::<Tag>(entity);
    let snapshot = world.get::<(Position, Velocity, Name, Health, Scale, Tag)>(entity);
    assert!(snapshot.is_none());
}

#[test]
fn six_component_archetype_mutation() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 0.0, y: 0.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 1.0 })
        .set_component(entity, Name("six"))
        .set_component(entity, Health(10))
        .set_component(entity, Scale(2.0))
        .set_component(entity, Tag("player"));

    let applied = world
        .get_mut::<(Position, Velocity, Name, Health, Scale, Tag)>(entity)
        .map(|(pos, vel, _name, health, scale, _tag)| {
            pos.x += vel.dx;
            health.0 -= 1;
            scale.0 *= 2.0;
            true
        });
    assert_eq!(applied, Some(true));

    let scale = world.get::<Scale>(entity).map(|s| s.0);
    assert_eq!(scale, Some(4.0));

    world.unset_component::<Tag>(entity);
    let result = world.get_mut::<(Position, Velocity, Name, Health, Scale, Tag)>(entity);
    assert!(result.is_none());
}

#[test]
fn duplicate_type_tuple_archetype_reads_the_same_component_twice() {
    let mut world = World::new();
    let entity = world.spawn();
    world.set_component(entity, Position { x: 3.0, y: 4.0 });

    let sum = world.get::<(Position, Position)>(entity).map(|(a, b)| a.x + b.y);

    assert_eq!(sum, Some(7.0));
}

#[test]
#[should_panic]
fn duplicate_type_tuple_archetype_mutation_panics_on_overlapping_keys() {
    let mut world = World::new();
    let entity = world.spawn();
    world.set_component(entity, Position { x: 0.0, y: 0.0 });

    let _ = world.get_mut::<(Position, Position)>(entity);
}
