use ecs::{Component, World};
use std::sync::atomic::{AtomicI32, Ordering};

struct Position {
    x: f32,
}

impl Component for Position {}

struct Velocity {
    dx: f32,
}

impl Component for Velocity {}

#[test]
fn with_archetype_runs_over_every_matching_entity() {
    let mut world = World::new();
    for i in 0..5 {
        let mut entity = ecs::Entity::new();
        entity.set_component(Position { x: i as f32 });
        world.insert(entity);
    }
    world.insert(ecs::Entity::new());

    let mut sum = 0.0;
    world.with_archetype::<Position>(|pos| sum += pos.x);

    assert_eq!(sum, 0.0 + 1.0 + 2.0 + 3.0 + 4.0);
}

#[test]
fn with_archetype_mut_updates_every_matching_entity_in_place() {
    let mut world = World::new();
    for i in 0..5 {
        let mut entity = ecs::Entity::new();
        entity.set_component(Position { x: i as f32 });
        world.insert(entity);
    }

    world.with_archetype_mut::<Position>(|pos| pos.x *= 2.0);

    let mut sum = 0.0;
    world.with_archetype::<Position>(|pos| sum += pos.x);
    assert_eq!(sum, 0.0 + 2.0 + 4.0 + 6.0 + 8.0);
}

#[test]
fn with_archetype_parallel_runs_over_every_matching_entity() {
    let mut world = World::new();
    for _ in 0..10_000 {
        let mut entity = ecs::Entity::new();
        entity.set_component(Position { x: 1.0 });
        world.insert(entity);
    }
    for _ in 0..10_000 {
        world.insert(ecs::Entity::new());
    }

    let hits = AtomicI32::new(0);
    world.with_archetype_async::<Position>(|_pos| {
        hits.fetch_add(1, Ordering::Relaxed);
    });

    assert_eq!(hits.load(Ordering::Relaxed), 10_000);
}

#[test]
fn with_archetype_parallel_mut_updates_every_matching_entity_in_place() {
    let mut world = World::new();
    for _ in 0..10_000 {
        let mut entity = ecs::Entity::new();
        entity
            .set_component(Position { x: 1.0 })
            .set_component(Velocity { dx: 2.0 });
        world.insert(entity);
    }

    world.with_archetype_async_mut::<(Position, Velocity)>(|(pos, vel)| {
        pos.x += vel.dx;
    });

    let mut sum = 0.0;
    world.with_archetype::<Position>(|pos| sum += pos.x);
    assert_eq!(sum, 3.0 * 10_000.0);
}
