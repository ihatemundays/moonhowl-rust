use ecs::{Component, Entity};

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
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    let sum = entity.with_archetype::<Position, _>(|pos| pos.x + pos.y);

    assert_eq!(sum, Some(3.0));
}

#[test]
fn tuple_archetype_matches_only_when_every_component_is_present() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    let mut ran = false;
    let moved = entity.with_archetype::<(Position, Velocity), _>(|(_position, _velocity)| ran = true);
    assert_eq!(moved, None);
    assert!(!ran);

    entity.set_component(Velocity { dx: 0.5, dy: -0.5 }).commit();

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
        .set_component(Name("player"))
        .commit();

    let label = entity
        .with_archetype::<(Position, Velocity, Name), _>(|(pos, vel, name)| {
            format!("{} at ({}, {}) moving ({}, {})", name.0, pos.x, pos.y, vel.dx, vel.dy)
        });
    assert_eq!(label, Some("player at (0, 0) moving (1, 1)".to_string()));

    entity.unset_component::<Name>().commit();
    assert!(!entity.has_component::<Name>());

    let label = entity.with_archetype::<(Position, Velocity, Name), _>(|_| ());
    assert_eq!(label, None);
}

#[test]
fn set_component_is_deferred_until_commit() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 });

    assert!(!entity.has_component::<Position>());

    entity.commit();

    assert!(entity.has_component::<Position>());
}

#[test]
fn commands_apply_in_order_on_commit() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 2.0 })
        .set_component(Position { x: 3.0, y: 4.0 })
        .unset_component::<Position>()
        .commit();

    assert!(!entity.has_component::<Position>());
}

#[test]
fn set_component_overwrites_the_previous_value_on_commit() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    entity.set_component(Position { x: 11.0, y: 2.0 }).commit();

    let x = entity.with_archetype::<Position, _>(|pos| pos.x);
    assert_eq!(x, Some(11.0));
}

#[test]
fn tuple_archetype_reflects_component_overwrites_after_commit() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: -1.0 })
        .commit();

    entity
        .set_component(Position { x: 1.0, y: -1.0 })
        .set_component(Velocity { dx: 2.0, dy: -1.0 })
        .commit();

    let state = entity
        .with_archetype::<(Position, Velocity), _>(|(pos, vel)| (pos.x, pos.y, vel.dx, vel.dy));
    assert_eq!(state, Some((1.0, -1.0, 2.0, -1.0)));
}

#[test]
fn unset_component_on_an_absent_type_is_a_harmless_no_op() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 1.0, y: 2.0 }).commit();

    entity.unset_component::<Velocity>().commit();

    assert!(entity.has_component::<Position>());
    assert!(!entity.has_component::<Velocity>());
}

#[test]
fn three_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("three"))
        .commit();

    entity.set_component(Position { x: 1.0, y: 1.0 }).commit();

    let position = entity.with_archetype::<Position, _>(|pos| (pos.x, pos.y));
    assert_eq!(position, Some((1.0, 1.0)));

    entity.unset_component::<Name>().commit();
    let mut ran = false;
    let result = entity.with_archetype::<(Position, Velocity, Name), _>(|_| ran = true);
    assert_eq!(result, None);
    assert!(!ran);
}

#[test]
fn single_component_archetype_is_none_when_absent() {
    let entity = Entity::new();

    let result = entity.with_archetype::<Position, _>(|pos| pos.x);

    assert_eq!(result, None);
}

#[test]
fn four_component_archetype() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 1.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("four"))
        .set_component(Health(10))
        .commit();

    let snapshot = entity.with_archetype::<(Position, Velocity, Name, Health), _>(
        |(pos, vel, name, health)| (pos.x, vel.dx, name.0, health.0),
    );
    assert_eq!(snapshot, Some((1.0, 1.0, "four", 10)));

    entity.unset_component::<Health>().commit();
    let snapshot = entity.with_archetype::<(Position, Velocity, Name, Health), _>(|_| ());
    assert_eq!(snapshot, None);
}

#[test]
fn four_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("four"))
        .set_component(Health(10))
        .commit();

    entity
        .set_component(Position { x: 1.0, y: 0.0 })
        .set_component(Health(9))
        .commit();

    let health = entity.with_archetype::<Health, _>(|h| h.0);
    assert_eq!(health, Some(9));

    entity.unset_component::<Name>().commit();
    let mut ran = false;
    let result =
        entity.with_archetype::<(Position, Velocity, Name, Health), _>(|_| ran = true);
    assert_eq!(result, None);
    assert!(!ran);
}

#[test]
fn five_component_archetype() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 1.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("five"))
        .set_component(Health(10))
        .set_component(Scale(2.0))
        .commit();

    let snapshot = entity.with_archetype::<(Position, Velocity, Name, Health, Scale), _>(
        |(pos, vel, name, health, scale)| (pos.x, vel.dx, name.0, health.0, scale.0),
    );
    assert_eq!(snapshot, Some((1.0, 1.0, "five", 10, 2.0)));

    entity.unset_component::<Scale>().commit();
    let snapshot = entity.with_archetype::<(Position, Velocity, Name, Health, Scale), _>(|_| ());
    assert_eq!(snapshot, None);
}

#[test]
fn five_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("five"))
        .set_component(Health(10))
        .set_component(Scale(2.0))
        .commit();

    entity
        .set_component(Position { x: 1.0, y: 0.0 })
        .set_component(Health(9))
        .set_component(Scale(4.0))
        .commit();

    let scale = entity.with_archetype::<Scale, _>(|s| s.0);
    assert_eq!(scale, Some(4.0));

    entity.unset_component::<Health>().commit();
    let mut ran = false;
    let result = entity
        .with_archetype::<(Position, Velocity, Name, Health, Scale), _>(|_| ran = true);
    assert_eq!(result, None);
    assert!(!ran);
}

#[test]
fn six_component_archetype() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 1.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("six"))
        .set_component(Health(10))
        .set_component(Scale(2.0))
        .set_component(Tag("player"))
        .commit();

    let snapshot = entity.with_archetype::<(Position, Velocity, Name, Health, Scale, Tag), _>(
        |(pos, vel, name, health, scale, tag)| (pos.x, vel.dx, name.0, health.0, scale.0, tag.0),
    );
    assert_eq!(snapshot, Some((1.0, 1.0, "six", 10, 2.0, "player")));

    entity.unset_component::<Tag>().commit();
    let snapshot =
        entity.with_archetype::<(Position, Velocity, Name, Health, Scale, Tag), _>(|_| ());
    assert_eq!(snapshot, None);
}

#[test]
fn six_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("six"))
        .set_component(Health(10))
        .set_component(Scale(2.0))
        .set_component(Tag("player"))
        .commit();

    entity
        .set_component(Position { x: 1.0, y: 0.0 })
        .set_component(Health(9))
        .set_component(Scale(4.0))
        .commit();

    let scale = entity.with_archetype::<Scale, _>(|s| s.0);
    assert_eq!(scale, Some(4.0));

    entity.unset_component::<Tag>().commit();
    let mut ran = false;
    let result = entity
        .with_archetype::<(Position, Velocity, Name, Health, Scale, Tag), _>(|_| ran = true);
    assert_eq!(result, None);
    assert!(!ran);
}

#[test]
fn duplicate_type_tuple_archetype_reads_the_same_component_twice() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 3.0, y: 4.0 }).commit();

    let sum = entity.with_archetype::<(Position, Position), _>(|(a, b)| a.x + b.y);

    assert_eq!(sum, Some(7.0));
}
