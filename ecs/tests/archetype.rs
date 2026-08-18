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

#[test]
fn three_component_archetype_mutation() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("three"));

    let applied = entity.with_archetype_mut::<(Position, Velocity, Name), _>(
        |(pos, vel, _name)| {
            pos.x += vel.dx;
            pos.y += vel.dy;
            true
        },
    );
    assert_eq!(applied, Some(true));

    let position = entity.with_archetype::<Position, _>(|pos| (pos.x, pos.y));
    assert_eq!(position, Some((1.0, 1.0)));

    entity.unset_component::<Name>();
    let mut ran = false;
    let result = entity.with_archetype_mut::<(Position, Velocity, Name), _>(|_| ran = true);
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
fn single_component_mutation_is_none_when_absent() {
    let mut entity = Entity::new();

    let mut ran = false;
    let result = entity.with_archetype_mut::<Position, _>(|_| ran = true);

    assert_eq!(result, None);
    assert!(!ran);
}

#[test]
fn four_component_archetype() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 1.0, y: 1.0 })
        .set_component(Velocity { dx: 1.0, dy: 1.0 })
        .set_component(Name("four"))
        .set_component(Health(10));

    let snapshot = entity.with_archetype::<(Position, Velocity, Name, Health), _>(
        |(pos, vel, name, health)| (pos.x, vel.dx, name.0, health.0),
    );
    assert_eq!(snapshot, Some((1.0, 1.0, "four", 10)));

    entity.unset_component::<Health>();
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
        .set_component(Health(10));

    let applied = entity.with_archetype_mut::<(Position, Velocity, Name, Health), _>(
        |(pos, vel, _name, health)| {
            pos.x += vel.dx;
            health.0 -= 1;
            true
        },
    );
    assert_eq!(applied, Some(true));

    let health = entity.with_archetype::<Health, _>(|h| h.0);
    assert_eq!(health, Some(9));

    entity.unset_component::<Name>();
    let mut ran = false;
    let result =
        entity.with_archetype_mut::<(Position, Velocity, Name, Health), _>(|_| ran = true);
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
        .set_component(Scale(2.0));

    let snapshot = entity.with_archetype::<(Position, Velocity, Name, Health, Scale), _>(
        |(pos, vel, name, health, scale)| (pos.x, vel.dx, name.0, health.0, scale.0),
    );
    assert_eq!(snapshot, Some((1.0, 1.0, "five", 10, 2.0)));

    entity.unset_component::<Scale>();
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
        .set_component(Scale(2.0));

    let applied = entity.with_archetype_mut::<(Position, Velocity, Name, Health, Scale), _>(
        |(pos, vel, _name, health, scale)| {
            pos.x += vel.dx;
            health.0 -= 1;
            scale.0 *= 2.0;
            true
        },
    );
    assert_eq!(applied, Some(true));

    let scale = entity.with_archetype::<Scale, _>(|s| s.0);
    assert_eq!(scale, Some(4.0));

    entity.unset_component::<Health>();
    let mut ran = false;
    let result = entity
        .with_archetype_mut::<(Position, Velocity, Name, Health, Scale), _>(|_| ran = true);
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
        .set_component(Tag("player"));

    let snapshot = entity.with_archetype::<(Position, Velocity, Name, Health, Scale, Tag), _>(
        |(pos, vel, name, health, scale, tag)| (pos.x, vel.dx, name.0, health.0, scale.0, tag.0),
    );
    assert_eq!(snapshot, Some((1.0, 1.0, "six", 10, 2.0, "player")));

    entity.unset_component::<Tag>();
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
        .set_component(Tag("player"));

    let applied = entity.with_archetype_mut::<(Position, Velocity, Name, Health, Scale, Tag), _>(
        |(pos, vel, _name, health, scale, _tag)| {
            pos.x += vel.dx;
            health.0 -= 1;
            scale.0 *= 2.0;
            true
        },
    );
    assert_eq!(applied, Some(true));

    let scale = entity.with_archetype::<Scale, _>(|s| s.0);
    assert_eq!(scale, Some(4.0));

    entity.unset_component::<Tag>();
    let mut ran = false;
    let result = entity
        .with_archetype_mut::<(Position, Velocity, Name, Health, Scale, Tag), _>(|_| ran = true);
    assert_eq!(result, None);
    assert!(!ran);
}

#[test]
fn duplicate_type_tuple_archetype_reads_the_same_component_twice() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 3.0, y: 4.0 });

    let sum = entity.with_archetype::<(Position, Position), _>(|(a, b)| a.x + b.y);

    assert_eq!(sum, Some(7.0));
}

#[test]
#[should_panic]
fn duplicate_type_tuple_archetype_mutation_panics_on_overlapping_keys() {
    let mut entity = Entity::new();
    entity.set_component(Position { x: 0.0, y: 0.0 });

    let _ = entity.with_archetype_mut::<(Position, Position), _>(|_| ());
}
